#!/usr/bin/env bash
# 在 Docker（ubuntu:24.04）里编 Rust 静态库与 C++ 插件、装到暂存目录、起真的 fcitx5、跑 e2e.py（开发模式，样例词库）。
# 宿主机直接运行本脚本；它把仓库挂到容器的 /work，再以 --in-container 在容器里调用自己。
set -euo pipefail

IMAGE=glimmer-fcitx5-e2e
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../../../.." && pwd)"

if [[ "${1:-}" != "--in-container" ]]; then
    docker build -t "$IMAGE" "$HERE"
    exec docker run --rm \
        -v "$ROOT":/work \
        -v glimmer-linux-cargo-registry:/opt/cargo/registry \
        -e CARGO_TARGET_DIR=/work/target/linux-docker \
        -w /work \
        "$IMAGE" bash /work/apps/linux/fcitx5/tests/docker/run.sh --in-container
fi

# 用 release：arm64 Linux 上 debug 构建编不过 gemm-f16（原因见 apps/linux/tests/docker/run.sh）；顺带复用 IBus 测试与打包的编译缓存
cargo build --release --locked -p glimmer-fcitx5

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
# 与打包同一套命令：前缀 /usr、DESTDIR 暂存，e2e.py 从暂存目录加载插件
cmake -S /work/apps/linux/fcitx5/addon -B "$TMP/build" -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr \
    -DGLIMMER_RUST_LIB="$CARGO_TARGET_DIR/release/libglimmer_fcitx5.a" -DGLIMMER_DATA_ROOT=/usr/lib/glimmer > "$TMP/cmake.log" \
    || { cat "$TMP/cmake.log"; exit 1; }
cmake --build "$TMP/build" -j
DESTDIR="$TMP/stage" cmake --install "$TMP/build"

export GLIMMER_FCITX5_STAGE="$TMP/stage"
# 仓库根：data/ 没有产品数据时 Router 退回 assets/sample/ 的样例词库（里面有「你好 ni hao」）
export GLIMMER_DATA_ROOT=/work
export RUST_LOG=info
dbus-run-session -- python3 "$HERE/e2e.py"
