#!/usr/bin/env bash
# 装机端到端测试：在干净的 ubuntu:24.04 里 apt 装 deb，起 ibus-daemon（走系统组件目录），跑 e2e_router.py。
# 验的是用户装到的东西：包布局、组件 XML、随包词库、真 Router。
#
#   apps/linux/tests/docker/install.sh target/deb/Glimmer-<版本>-arm64.deb
#
# deb 先用 apps/linux/scripts/package-docker.sh 打。
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ "${1:-}" != "--in-container" ]]; then
    DEB="$(cd "$(dirname "${1:?用法：install.sh <deb 路径>}")" && pwd)/$(basename "$1")"
    exec docker run --rm \
        -v "$DEB":/tmp/glimmer.deb:ro \
        -v "$HERE":/tests:ro \
        ubuntu:24.04 bash /tests/install.sh --in-container
fi

export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends /tmp/glimmer.deb dbus python3 python3-gi gir1.2-ibus-1.0 > /dev/null
dbus-uuidgen > /etc/machine-id
echo "已装：$(dpkg-query -W -f='${Package} ${Version} ${Architecture}' glimmer)"

HOME="$(mktemp -d)"
export HOME
export XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache" GSETTINGS_BACKEND=memory
mkdir -p "$XDG_CACHE_HOME"
unset DISPLAY WAYLAND_DISPLAY

dbus-run-session -- bash -c '
    set -euo pipefail
    ibus-daemon --panel=disable --emoji-extension=disable --config=disable --verbose > "$XDG_CACHE_HOME/ibus-daemon.log" 2>&1 &
    daemon=$!
    for _ in $(seq 100); do
        if ls "$XDG_CONFIG_HOME"/ibus/bus/* > /dev/null 2>&1; then break; fi
        sleep 0.1
    done
    status=0
    (cd /tests && python3 e2e_router.py) || status=$?
    kill "$daemon" 2> /dev/null || true
    wait "$daemon" 2> /dev/null || true
    echo "---- 引擎日志（~/.local/share/glimmer/logs）----"
    tail -n 30 "$HOME"/.local/share/glimmer/logs/*.log 2> /dev/null || true
    if [[ $status -ne 0 ]]; then
        echo "---- ibus-daemon 日志 ----"
        tail -n 80 "$XDG_CACHE_HOME/ibus-daemon.log"
    fi
    exit $status
'
