#!/usr/bin/env bash
# 本机重建并部署官网（glimmer-web，Cloudflare Workers）：记一笔发版标签 → 清掉下载列表缓存 → 构建 → 部署 → 核对下载地址。
# CI 里没配 GLIMMER_WEB_TOKEN 时 release.yml 不会触发官网重建，发版后用它手动上线。
#
#   tools/release/deploy-website.sh macos-v0.1.6 windows-v0.1.0-alpha.5   # 标签只用来写提交信息与 upstream.json
#   tools/release/deploy-website.sh                                       # 不记发版，只按主仓库 HEAD 重建部署
#
# 环境变量：GLIMMER_WEB_DIR 官网仓库目录，缺省是主仓库旁边的 ../glimmer-web；要先 npx wrangler login 过。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WEB="${GLIMMER_WEB_DIR:-$(cd "$ROOT/.." && pwd)/glimmer-web}"
SITE="https://glimmerinput.app"
[[ -d "$WEB/.git" ]] || { echo "错误: 找不到官网仓库 ${WEB}（用 GLIMMER_WEB_DIR 指定）" >&2; exit 1; }

# 文档按主仓库已推送的提交拉，本地没推的提交官网构建拉不到
DOCS_SHA="$(git -C "$ROOT" rev-parse HEAD)"
git -C "$ROOT" fetch -q origin
git -C "$ROOT" merge-base --is-ancestor "$DOCS_SHA" origin/main || { echo "错误: 主仓库 HEAD $DOCS_SHA 还没推到 origin/main" >&2; exit 1; }

cd "$WEB"
if [[ $# -gt 0 ]]; then
  tags="$(IFS='、'; echo "$*")"
  echo "==> 记录发版 $tags"
  python3 - src/content/upstream.json "$tags" "$DOCS_SHA" <<'PY'
import datetime, json, sys
path, release, sha = sys.argv[1:4]
record = {"release": release, "docs": sha, "updated": datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")}
with open(path, "w", encoding="utf-8") as f:
    json.dump(record, f, ensure_ascii=False, indent=2)
    f.write("\n")
PY
  git add src/content/upstream.json
  git diff --cached --quiet || git commit -q -m "同步主仓库：发版 $tags" -m "主仓库提交 $DOCS_SHA"
fi

echo "==> 构建"
# sync-releases 一小时内有本地缓存就不重拉，发版后必须删掉，否则部署出旧的下载列表
rm -f src/content/releases.json
GLIMMER_DOCS_SOURCE=git npm run build

echo "==> 部署"
npx wrangler deploy

echo "==> 核对下载页的安装包地址"
sleep 5
page="$(curl -fsSL "$SITE/download?t=$(date +%s)")"
bad=0
while IFS= read -r url; do
  code="$(curl -s -o /dev/null -IL -w '%{http_code}' "$url")"
  echo "  $code $url"
  [[ "$code" == 200 ]] || bad=1
done < <(grep -oE 'https://github.com/[^"]+/releases/download/[^"]+' <<<"$page" | sort -u)
[[ $bad == 0 ]] || { echo "错误: 下载页有打不开的安装包地址" >&2; exit 1; }
echo "官网已上线：$SITE/download"
