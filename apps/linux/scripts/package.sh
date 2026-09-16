#!/usr/bin/env bash
# 把 glimmer-linux（IBus 引擎进程 glimmer-ibus）打成 deb。只在 Linux 上跑（CI 的 ubuntu runner 或 Docker 里）；
# macOS 本机用同目录的 package-docker.sh。
#
#   apps/linux/scripts/package.sh     # 出 target/deb/Glimmer-<版本>-<amd64|arm64>.deb
#
# 需要 cargo、dpkg-deb、dpkg-shlibdeps（dpkg-dev）。
# 架构：缺省编译本机架构；GLIMMER_TARGET=x86_64-unknown-linux-gnu / aarch64-unknown-linux-gnu 交叉编译（要自备链接器）。
# 产品数据：data/generated/ 里有 dict.qj 就按产品数据装（缺哪个随包文件打警告），没有就只带 assets/sample/ 样例词库并打警告；
# 与 bundle.sh 不同，这里不从 TSV 重打 .qj，CI 从 data Release 解出的就是 .qj，本机先跑过 bundle.sh 或 data-bundle.sh 流程。
# 本地整句模型 data/model/model.qjm、五笔码表 data/generated/wubi86.qj 有就带，没有就跳过。
#
# 版本号（apps/linux/Cargo.toml 的 version，各平台壳独立）：
#   - 文件名用 Cargo 原样的版本；带 -dev 时接 git 短哈希（工作区有改动再加 +）：Glimmer-0.1.0-alpha.1-dev-1a2b3c4-arm64.deb
#   - deb 的 Version 字段把 - 换成 ~（~ 排在一切之前，预发布版低于正式版）：0.1.0-alpha.1 → 0.1.0~alpha.1；
#     dev 版再接 +g<短哈希>（有改动加 .dirty）：0.1.0~alpha.1~dev+g1a2b3c4，排在 0.1.0~alpha.1 之前。
#
# 调试用环境变量：
#   GLIMMER_BIN      直接用这个二进制，跳过 cargo build（验证打包流程用）
#   GLIMMER_GIT_REV  git 短哈希（Docker 里的 worktree 读不到 git 时由 package-docker.sh 从宿主传进来，脏的带 +）
#
# 布局：/usr/lib/glimmer/glimmer-ibus，资源与它同级（data/ 与 assets/，glimmer_platform::resources::bundled_root 认这个）；
# IBus 组件 /usr/share/ibus/component/glimmer.xml；图标 /usr/share/icons/hicolor/256x256/apps/glimmer.png；许可 /usr/share/doc/glimmer/copyright。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
BIN_NAME="glimmer-ibus"
PACKAGING="$ROOT/apps/linux/packaging"
TARGET="${GLIMMER_TARGET:-}"
case "${TARGET:-$(uname -m)}" in
  x86_64-unknown-linux-gnu|x86_64) ARCH="amd64" ;;
  aarch64-unknown-linux-gnu|aarch64|arm64) ARCH="arm64" ;;
  *) echo "不认识的架构: ${TARGET:-$(uname -m)}" >&2; exit 1 ;;
esac

cd "$ROOT"
GIT_REV="${GLIMMER_GIT_REV:-}"
if [[ -z "$GIT_REV" ]]; then
  GIT_REV="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
  if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then GIT_REV="${GIT_REV}+"; fi
fi
# 构建标识进诊断信息（与 bundle.sh 同一个编译期变量）
export GLIMMER_BUILD="${GIT_REV} · $(date +%Y-%m-%d)"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' apps/linux/Cargo.toml | head -1)"
[[ -n "$VERSION" ]] || { echo "apps/linux/Cargo.toml 里没找到 version" >&2; exit 1; }
if [[ "$VERSION" == *-dev ]]; then
  FILE_VERSION="$VERSION-$GIT_REV"
  REV_CLEAN="${GIT_REV%+}"
  DEB_VERSION="${VERSION//-/\~}+g$REV_CLEAN"
  [[ "$GIT_REV" == *+ ]] && DEB_VERSION="$DEB_VERSION.dirty"
else
  FILE_VERSION="$VERSION"
  DEB_VERSION="${VERSION//-/\~}"
fi

if [[ -n "${GLIMMER_BIN:-}" ]]; then
  BIN="$GLIMMER_BIN"
  echo "注意: 用 GLIMMER_BIN=$BIN，跳过 cargo build" >&2
else
  BUILD_ARGS=(build --release --locked -p glimmer-linux)
  # 认 CARGO_TARGET_DIR（package-docker.sh 把容器里的 target 放 target/linux-docker）
  TARGET_DIR="${CARGO_TARGET_DIR:-target}"
  BIN="$TARGET_DIR/release/$BIN_NAME"
  if [[ -n "$TARGET" ]]; then
    BUILD_ARGS+=(--target "$TARGET")
    BIN="$TARGET_DIR/$TARGET/release/$BIN_NAME"
  fi
  cargo "${BUILD_ARGS[@]}"
fi
[[ -x "$BIN" ]] || { echo "缺二进制 $BIN" >&2; exit 1; }

# 每个架构一个工作目录，成品都放 target/deb/，两个架构接着打互不覆盖
WORK="$ROOT/target/deb/$ARCH"
STAGE="$WORK/root"
LIB="$STAGE/usr/lib/glimmer"
DEB="$ROOT/target/deb/Glimmer-$FILE_VERSION-$ARCH.deb"
rm -rf "$WORK"
mkdir -p "$STAGE/DEBIAN" "$LIB" "$STAGE/usr/share/ibus/component" \
  "$STAGE/usr/share/icons/hicolor/256x256/apps" "$STAGE/usr/share/doc/glimmer"

install -m 755 "$BIN" "$LIB/$BIN_NAME"
# 本机架构才 strip（交叉编译的二进制本机 strip 认不了）；调试符号不随包
if [[ -z "$TARGET" && -z "${GLIMMER_BIN:-}" ]] && command -v strip >/dev/null 2>&1; then
  strip --strip-unneeded "$LIB/$BIN_NAME"
fi

# 随 git 的资源：与 glimmer.iss 的清单一致
install -D -m 644 assets/emoji/emoji-zh.tsv "$LIB/assets/emoji/emoji-zh.tsv"
install -D -m 644 assets/emoji/emoji-en.tsv "$LIB/assets/emoji/emoji-en.tsv"
install -D -m 644 assets/levels/levels-en.tsv "$LIB/assets/levels/levels-en.tsv"
install -D -m 644 assets/levels/levels-ja.tsv "$LIB/assets/levels/levels-ja.tsv"
install -D -m 644 assets/sample/dict.tsv "$LIB/assets/sample/dict.tsv"

# 产品数据：只装运行时要的 .qj / .tsv，不装 dev 中间产物
GEN="data/generated"
if [[ -f "$GEN/dict.qj" ]]; then
  for f in dict.qj lm.qj glossary-en.qj glossary-ja.qj glossary-zh.qj english.tsv; do
    if [[ -f "$GEN/$f" ]]; then
      install -D -m 644 "$GEN/$f" "$LIB/data/generated/$f"
    else
      echo "警告: 缺 $GEN/$f，包里不带它" >&2
    fi
  done
  if ls "$GEN"/dicts/*.qj >/dev/null 2>&1; then
    install -d -m 755 "$LIB/data/generated/dicts"
    install -m 644 "$GEN"/dicts/*.qj "$LIB/data/generated/dicts/"
  else
    echo "警告: 没有 $GEN/dicts/*.qj，包里不带领域词库" >&2
  fi
  echo "使用 $GEN/ 的产品数据（自建词库）"
else
  echo "警告: 没有 $GEN/dict.qj，只带 assets/sample/ 样例词库（不是产品词库，分发前先下载或生成产品数据）" >&2
fi
# 五笔 86 码表（LGPL-3.0，许可与署名随包）：没有就不装，开着五笔时引擎当没开
if [[ -f "$GEN/wubi86.qj" ]]; then
  install -D -m 644 "$GEN/wubi86.qj" "$LIB/data/generated/wubi86.qj"
  install -D -m 644 assets/wubi/LICENSE.LGPL-3.0 "$LIB/assets/wubi/LICENSE.LGPL-3.0"
  install -D -m 644 assets/wubi/AUTHORS "$LIB/assets/wubi/AUTHORS"
  echo "打包五笔 86 码表：$GEN/wubi86.qj"
else
  echo "注意: 没有 $GEN/wubi86.qj，包里不带五笔码表（生成命令见 assets/wubi/README.md）" >&2
fi
# 本地整句模型单文件（tools/release/pack-model.sh 打成）：没有就不装，引擎不重排
MODEL="${GLIMMER_MODEL_DIR:-data/model}/model.qjm"
if [[ -f "$MODEL" ]]; then
  install -D -m 644 "$MODEL" "$LIB/data/model/model.qjm"
  echo "打包本地整句模型：$MODEL"
fi

# IBus 组件、图标、许可
sed -e "s/@VERSION@/$FILE_VERSION/g" "$PACKAGING/glimmer.xml.in" > "$STAGE/usr/share/ibus/component/glimmer.xml"
chmod 644 "$STAGE/usr/share/ibus/component/glimmer.xml"
install -m 644 "$PACKAGING/glimmer.png" "$STAGE/usr/share/icons/hicolor/256x256/apps/glimmer.png"
install -m 644 "$PACKAGING/copyright" "$STAGE/usr/share/doc/glimmer/copyright"

# 动态库依赖由 dpkg-shlibdeps 从二进制算（它要求当前目录有 debian/control，给它一个最小的）；算不出来退回只依赖 libc6
SHLIBS_DEPENDS="libc6"
mkdir -p "$WORK/shlibs/debian"
printf 'Source: glimmer\n\nPackage: glimmer\nArchitecture: any\n' > "$WORK/shlibs/debian/control"
if deps="$(cd "$WORK/shlibs" && dpkg-shlibdeps -O -e "$LIB/$BIN_NAME" 2>/dev/null)"; then
  deps="$(printf '%s\n' "$deps" | sed -n 's/^shlibs:Depends=//p')"
  [[ -n "$deps" ]] && SHLIBS_DEPENDS="$deps"
else
  echo "注意: dpkg-shlibdeps 失败，Depends 只写 libc6" >&2
fi
rm -rf "$WORK/shlibs"

INSTALLED_SIZE="$(du -sk --exclude=DEBIAN "$STAGE" | cut -f1)"
sed -e "s/@DEB_VERSION@/$DEB_VERSION/g" -e "s/@ARCH@/$ARCH/g" \
  -e "s/@INSTALLED_SIZE@/$INSTALLED_SIZE/g" -e "s/@SHLIBS_DEPENDS@/$SHLIBS_DEPENDS/g" \
  "$PACKAGING/control.in" > "$STAGE/DEBIAN/control"
install -m 755 "$PACKAGING/postinst" "$STAGE/DEBIAN/postinst"
install -m 755 "$PACKAGING/postrm" "$STAGE/DEBIAN/postrm"

# 目录一律 755（宿主 umask 或 Docker 挂载可能给出别的权限）
find "$STAGE" -type d -exec chmod 755 {} +
dpkg-deb --build --root-owner-group -Zxz "$STAGE" "$DEB"
echo "deb: $DEB（Version $DEB_VERSION，$ARCH）"
sha256sum "$DEB"
