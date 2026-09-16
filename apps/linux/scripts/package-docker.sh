#!/usr/bin/env bash
# 在 macOS（或任何有 Docker 的机器）上用 Docker 打 Linux deb：容器里跑 package.sh。
#
#   apps/linux/scripts/package-docker.sh                 # 本机架构（Apple Silicon 上是 arm64）
#   GLIMMER_DOCKER_PLATFORM=linux/amd64 apps/linux/scripts/package-docker.sh   # amd64（走 Rosetta / QEMU，慢）
#
#   GLIMMER_DOCKER_BASE=ubuntu:22.04 GLIMMER_SKIP_FCITX5=1 apps/linux/scripts/package-docker.sh   # 在 22.04 上编，Depends 与发版包一致
#
# 镜像 <基础镜像> + rustup 1.96.0（与 rust-toolchain.toml 一致）+ dpkg-dev，基础镜像有 Fcitx5 5.1 开发包时再装插件的构建依赖，
# 按平台与基础镜像建一次后复用（glimmer-deb-builder:<arch>-<基础镜像>-rust<版本>-<依赖版本>）。
# 仓库挂到 /src；cargo 的 target 放 target/linux-docker/（ubuntu:24.04）或 target/linux-docker-<基础镜像>/（其他，glibc 不同的产物不混用），
# registry 缓存放命名 volume。成品仍在 target/deb/Glimmer-<版本>-<arch>.deb。
#
#   GLIMMER_DOCKER_BASE    基础镜像，缺省 ubuntu:24.04。没有 Fcitx5 5.1 开发包的（ubuntu:22.04、debian:12）要配 GLIMMER_SKIP_FCITX5=1 或 GLIMMER_FCITX5_STAGE
#   GLIMMER_SKIP_FCITX5=1  只打 IBus 包（原样传给 package.sh）
#   GLIMMER_IBUS_BIN / GLIMMER_FCITX5_STAGE  预编产物（含义见 package.sh），必须在仓库目录里（容器只挂仓库），传进去前换成容器路径
#   GLIMMER_DATA_DIR  产品数据目录（含 generated/ 与 model/），缺省仓库的 data/；worktree 里没有数据时指向主工作树的 data/，只读挂载
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PLATFORM="${GLIMMER_DOCKER_PLATFORM:-}"
case "${PLATFORM:-linux/$(uname -m)}" in
  linux/amd64|linux/x86_64) PLATFORM="linux/amd64"; TAG="amd64" ;;
  linux/arm64|linux/aarch64) PLATFORM="linux/arm64"; TAG="arm64" ;;
  *) echo "不认识的平台: $PLATFORM" >&2; exit 1 ;;
esac
BASE="${GLIMMER_DOCKER_BASE:-ubuntu:24.04}"
# ubuntu:22.04 → ubuntu22.04，用于镜像标签与 target 目录名
BASE_SLUG="${BASE//[^A-Za-z0-9.]/}"
TOOLCHAIN="$(sed -n 's/^channel = "\(.*\)"/\1/p' "$ROOT/rust-toolchain.toml")"
TARGET_SUBDIR="linux-docker"
[[ "$BASE" == ubuntu:24.04 ]] || TARGET_SUBDIR="linux-docker-$BASE_SLUG"

# 镜像不存在才建；工具链升级或下面的 apt 包清单变了时改镜像名里的版本号，旧镜像自然不再被用。
# Fcitx5 插件的构建依赖只在基础镜像的 libfcitx5core-dev ≥ 5.1 时装（22.04 / Debian 12 只有 5.0，编不了插件）
IMAGE="glimmer-deb-builder:$TAG-$BASE_SLUG-rust$TOOLCHAIN-deps3"
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  docker build --platform "$PLATFORM" -t "$IMAGE" - <<EOF
FROM $BASE
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      ca-certificates curl build-essential pkg-config libssl-dev dpkg-dev git \
    && v="\$(apt-cache policy libfcitx5core-dev | sed -n 's/^ *Candidate: *//p')" \
    && if [ -n "\$v" ] && [ "\$v" != "(none)" ] && dpkg --compare-versions "\$v" ge 5.1; then \
      DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        cmake extra-cmake-modules libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev fcitx5-modules-dev; \
    fi \
    && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:\$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain $TOOLCHAIN \
    && chmod -R a+rwX /opt/rustup /opt/cargo
EOF
fi

# 容器里没有 Fcitx5 5.1 开发包、又要编插件时先拦下，免得 cargo 编完才在 cmake 失败
if [[ "${GLIMMER_SKIP_FCITX5:-}" != 1 && -z "${GLIMMER_FCITX5_STAGE:-}" ]] \
  && ! docker run --rm --platform "$PLATFORM" "$IMAGE" dpkg -s libfcitx5core-dev >/dev/null 2>&1; then
  echo "$BASE 里没有 Fcitx5 5.1 开发包，编不了插件：设 GLIMMER_SKIP_FCITX5=1 只打 IBus 包，或给 GLIMMER_FCITX5_STAGE（24.04 上编好的插件安装目录）" >&2
  exit 1
fi

# in_container <宿主路径>：仓库里的路径换成容器里 /src 下的路径；空串原样返回，仓库外的直接失败
in_container() {
  [[ -n "$1" ]] || return 0
  local abs
  abs="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
  [[ "$abs" == "$ROOT"/* ]] || { echo "$1 不在仓库 $ROOT 里，容器看不到" >&2; return 1; }
  printf '/src/%s' "${abs#"$ROOT"/}"
}
IBUS_BIN_IN="$(in_container "${GLIMMER_IBUS_BIN:-}")"
FCITX5_STAGE_IN="$(in_container "${GLIMMER_FCITX5_STAGE:-}")"

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
  -e CARGO_TARGET_DIR="/src/target/$TARGET_SUBDIR" \
  -e GLIMMER_GIT_REV="$GIT_REV" \
  -e GLIMMER_IBUS_BIN="$IBUS_BIN_IN" \
  -e GLIMMER_FCITX5_STAGE="$FCITX5_STAGE_IN" \
  -e GLIMMER_SKIP_FCITX5="${GLIMMER_SKIP_FCITX5:-}" \
  -w /src \
  "$IMAGE" \
  apps/linux/scripts/package.sh
