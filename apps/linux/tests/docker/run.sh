#!/usr/bin/env bash
# 在 Docker（ubuntu:24.04）里编 glimmer-ibus、起真的 ibus-daemon、跑 e2e.py。
# 宿主机直接运行本脚本；它把仓库挂到容器的 /work，再以 --in-container 在容器里调用自己。
set -euo pipefail

IMAGE=glimmer-linux-e2e
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../../.." && pwd)"

if [[ "${1:-}" != "--in-container" ]]; then
    docker build -t "$IMAGE" "$HERE"
    exec docker run --rm \
        -v "$ROOT":/work \
        -v glimmer-linux-cargo-registry:/opt/cargo/registry \
        -e CARGO_TARGET_DIR=/work/target/linux-docker \
        -w /work \
        "$IMAGE" bash /work/apps/linux/tests/docker/run.sh --in-container
fi

cargo build -p glimmer-linux --bin glimmer-ibus
EXEC="$CARGO_TARGET_DIR/debug/glimmer-ibus"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/component" "$TMP/config" "$TMP/cache"
sed "s|@EXEC@|$EXEC|" "$HERE/glimmer.xml.in" > "$TMP/component/glimmer.xml"

export IBUS_COMPONENT_PATH="$TMP/component"
export XDG_CONFIG_HOME="$TMP/config"
export XDG_CACHE_HOME="$TMP/cache"
export GSETTINGS_BACKEND=memory
export RUST_LOG=debug
unset DISPLAY WAYLAND_DISPLAY

# 私有总线由 ibus-daemon 自己起；dbus-run-session 给它一条会话总线（ibus 启动时会连）。
dbus-run-session -- bash -c '
    set -euo pipefail
    ibus-daemon --panel=disable --emoji-extension=disable --config=disable --cache=none --verbose > "$XDG_CACHE_HOME/ibus-daemon.log" 2>&1 &
    daemon=$!
    for _ in $(seq 100); do
        if ls "$XDG_CONFIG_HOME"/ibus/bus/* > /dev/null 2>&1; then break; fi
        sleep 0.1
    done
    status=0
    python3 '"$HERE"'/e2e.py || status=$?
    kill "$daemon" 2> /dev/null || true
    wait "$daemon" 2> /dev/null || true
    if [[ $status -ne 0 ]]; then
        echo "---- ibus-daemon 与 glimmer-ibus 日志 ----"
        cat "$XDG_CACHE_HOME/ibus-daemon.log"
    fi
    exit $status
'
