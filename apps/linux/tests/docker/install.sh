#!/usr/bin/env bash
# 装机端到端测试：在干净的基础镜像（缺省 ubuntu:24.04）里 apt 装 deb，两个框架各测一遍，验的是用户装到的东西（包布局、随包词库、真 Router）：
#   1. IBus：起 ibus-daemon（走系统组件目录），跑 e2e_router.py；
#   2. Fcitx5：再 apt 装 fcitx5，跑 apps/linux/fcitx5/tests/docker/e2e.py 的装机模式（插件从系统插件目录加载）。
# IBus 先测、fcitx5 后装，保证 IBus 那一遍跑在没有 fcitx5 的系统上（验 Depends 里的「或」对 IBus 用户成立）。
#
#   apps/linux/tests/docker/install.sh target/deb/Glimmer-<版本>-arm64.deb
#   apps/linux/tests/docker/install.sh --ibus-only target/deb/Glimmer-<版本>-arm64.deb   # 只测 IBus（GLIMMER_SKIP_FCITX5=1 打的包）
#   GLIMMER_DOCKER_BASE=debian:12 apps/linux/tests/docker/install.sh --ibus-only <deb>      # 换基础镜像（ubuntu:22.04、debian:12 等）
#
# 基础镜像的 fcitx5 低于 5.1（ubuntu:22.04、debian:12）时 Fcitx5 那一遍跳过并说明，只算 IBus 的结果。
#
# deb 先用 apps/linux/scripts/package-docker.sh 打。
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ "${1:-}" != "--in-container" ]]; then
    IBUS_ONLY=0
    if [[ "${1:-}" == "--ibus-only" ]]; then IBUS_ONLY=1; shift; fi
    DEB="$(cd "$(dirname "${1:?用法：install.sh [--ibus-only] <deb 路径>}")" && pwd)/$(basename "$1")"
    exec docker run --rm \
        -v "$DEB":/tmp/glimmer.deb:ro \
        -v "$HERE":/tests:ro \
        -v "$HERE/../../fcitx5/tests/docker":/fcitx5-tests:ro \
        -e IBUS_ONLY="$IBUS_ONLY" \
        "${GLIMMER_DOCKER_BASE:-ubuntu:24.04}" bash /tests/install.sh --in-container
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
echo "IBus：PASS"

if [[ "${IBUS_ONLY:-0}" == 1 ]]; then
    exit 0
fi

# 插件要 Fcitx5 5.1：发行版的 fcitx5 更老时装不上也不该测（Depends 的「或」让 IBus 用户照常装），跳过
FCITX5_VERSION="$(apt-cache policy fcitx5 | sed -n 's/^ *Candidate: *//p')"
if [[ -z "$FCITX5_VERSION" || "$FCITX5_VERSION" == "(none)" ]] || ! dpkg --compare-versions "$FCITX5_VERSION" ge 5.1; then
    echo "Fcitx5：跳过（$(. /etc/os-release && echo "$PRETTY_NAME") 的 fcitx5 是 ${FCITX5_VERSION:-无}，插件要 5.1 或更新）"
    exit 0
fi

# Fcitx5：包里要有插件三件套，缺了说明打包时跳过了插件
for f in /usr/share/fcitx5/addon/glimmer.conf /usr/share/fcitx5/inputmethod/glimmer.conf; do
    [[ -f "$f" ]] || { echo "包里没有 $f（GLIMMER_SKIP_FCITX5=1 打的包用 --ibus-only）" >&2; exit 1; }
done
ls /usr/lib/*/fcitx5/glimmer.so > /dev/null || { echo "包里没有 fcitx5/glimmer.so" >&2; exit 1; }
apt-get install -y -qq --no-install-recommends fcitx5 fcitx5-modules python3-dbus > /dev/null

# 用新的 HOME，不继承 IBus 那一遍的用户数据与日志
HOME="$(mktemp -d)"
export HOME XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache" XDG_DATA_HOME="$HOME/.local/share"
mkdir -p "$XDG_CACHE_HOME"

dbus-run-session -- bash -c '
    set -euo pipefail
    status=0
    (cd /fcitx5-tests && python3 e2e.py --installed) || status=$?
    echo "---- 插件日志（~/.local/share/glimmer/logs）----"
    tail -n 30 "$HOME"/.local/share/glimmer/logs/*.log 2> /dev/null || true
    exit $status
'
echo "Fcitx5：PASS"
