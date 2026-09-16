#!/usr/bin/env bash
# 在 macOS（或任何有 Docker 的机器）上用 Docker 打 Linux deb：容器里跑 package.sh。
#
#   apps/linux/scripts/package-docker.sh                 # 本机架构（Apple Silicon 上是 arm64）
#   GLIMMER_DOCKER_PLATFORM=linux/amd64 apps/linux/scripts/package-docker.sh   # amd64（走 Rosetta / QEMU，慢）
#
# 镜像 ubuntu:24.04 + rustup 1.96.0（与 rust-toolchain.toml 一致）+ dpkg-dev，按平台建一次后复用（glimmer-deb-builder:<arch>）。
# 仓库挂到 /src；cargo 的 target 放 target/linux-docker/（不和宿主的 target/ 混用），registry 缓存放命名 volume。
# 成品仍在 target/deb/Glimmer-<版本>-<arch>.deb。
#
#   GLIMMER_DATA_DIR  产品数据目录（含 generated/ 与 model/），缺省仓库的 data/；worktree 里没有数据时指向主工作树的 data/，只读挂载
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PLATFORM="${GLIMMER_DOCKER_PLATFORM:-}"
case "${PLATFORM:-linux/$(uname -m)}" in
  linux/amd64|linux/x86_64) PLATFORM="linux/amd64"; TAG="amd64" ;;
  linux/arm64|linux/aarch64) PLATFORM="linux/arm64"; TAG="arm64" ;;
  *) echo "不认识的平台: $PLATFORM" >&2; exit 1 ;;
esac
IMAGE="glimmer-deb-builder:$TAG"
TOOLCHAIN="$(sed -n 's/^channel = "\(.*\)"/\1/p' "$ROOT/rust-toolchain.toml")"

# 镜像不存在才建；工具链升级时镜像名里带上版本号，旧镜像自然不再被用
IMAGE="$IMAGE-rust$TOOLCHAIN"
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  docker build --platform "$PLATFORM" -t "$IMAGE" - <<EOF
FROM ubuntu:24.04
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      ca-certificates curl build-essential pkg-config libssl-dev dpkg-dev git \
    && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:\$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain $TOOLCHAIN \
    && chmod -R a+rwX /opt/rustup /opt/cargo
EOF
fi

# git 短哈希在宿主上算好传进去：worktree 的 .git 指向挂载范围之外，容器里读不到
GIT_REV="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
if [[ -n "$(git -C "$ROOT" status --porcelain 2>/dev/null)" ]]; then GIT_REV="${GIT_REV}+"; fi

MOUNTS=(-v "$ROOT:/src")
if [[ -n "${GLIMMER_DATA_DIR:-}" ]]; then
  MOUNTS+=(-v "$(cd "$GLIMMER_DATA_DIR" && pwd):/src/data:ro")
fi

docker run --rm --platform "$PLATFORM" \
  "${MOUNTS[@]}" \
  -v "glimmer-cargo-registry-$TAG:/opt/cargo/registry" \
  -e CARGO_TARGET_DIR=/src/target/linux-docker \
  -e GLIMMER_GIT_REV="$GIT_REV" \
  -e GLIMMER_BIN="${GLIMMER_BIN:-}" \
  -w /src \
  "$IMAGE" \
  apps/linux/scripts/package.sh
