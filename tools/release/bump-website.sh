#!/usr/bin/env bash
# 让官网（rambocode/glimmer-web，Cloudflare Workers Builds 按仓库提交自动构建）重新构建：
# 往它的 src/content/upstream.json 写一笔「本次发版的标签 / 文档提交」并推一个提交。
# 官网构建时拉最新 Release 的 releases.json，文档按记下的提交号拉主仓库 docs/user；只在发版时调用，文档只随发版更新。
#
# 用法：tools/release/bump-website.sh（在 release.yml 里跑，读 GITHUB_REF_NAME 标签与 GITHUB_SHA）
# 需要环境变量 GLIMMER_WEB_TOKEN：对 glimmer-web 有 Contents: write 的 fine-grained PAT。
set -euo pipefail

token=${GLIMMER_WEB_TOKEN:?缺 GLIMMER_WEB_TOKEN}
repo=${GLIMMER_WEB_REPO:-rambocode/glimmer-web}
sha=${GITHUB_SHA:-$(git rev-parse HEAD)}
tag=${GITHUB_REF_NAME:?缺 GITHUB_REF_NAME（版本标签）}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git clone -q --depth 1 "https://x-access-token:${token}@github.com/${repo}.git" "$work/web"
cd "$work/web"

file=src/content/upstream.json
mkdir -p "$(dirname "$file")"
message="同步主仓库：发版 ${tag}"
python3 - "$file" "$tag" "$sha" <<'PY'
import json, sys, datetime
path, release, sha = sys.argv[1:4]
json.dump(
    {"release": release, "docs": sha, "updated": datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")},
    open(path, "w", encoding="utf-8"),
    ensure_ascii=False,
    indent=2,
)
open(path, "a", encoding="utf-8").write("\n")
PY
git add "$file"
if git diff --cached --quiet; then
  echo "官网仓库无需更新"
  exit 0
fi
git -c user.name="glimmer-ci" -c user.email="ci@glimmer.app" commit -q -m "$message" -m "主仓库提交 ${sha}"
git push -q origin HEAD
echo "已推送到 ${repo}：${message}"
