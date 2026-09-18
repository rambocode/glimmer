#!/usr/bin/env bash
# 把改名后的上游提交按顺序 cherry-pick 进同步分支。
#   tools/upstream/pick.sh <picks.tsv> <done.txt>
# picks.tsv 每行「上游哈希 改名哈希」（用 paste <(git rev-list --reverse 基点..upstream/main) <(git rev-list --reverse 基点'..upstream-renamed) 生成后删掉要跳过的行）；
# done.txt 记已挑的上游哈希前 7 位，脚本自己追加。
# CHANGELOG / Cargo.lock 冲突自动取我们的（条目事后手工并进「未发布」小节，Cargo.lock 让 cargo 重生成）；只改 CHANGELOG 的提交变空就跳过；
# 其他冲突停下来手工解，解完 git add -A && git cherry-pick --continue，再把上游哈希追加进 done.txt、重跑本脚本。
# 每条提交信息末尾补「上游提交：<哈希>」；钩子在挑的过程中关掉，最后统一跑 fmt / clippy / test。
set -uo pipefail
picks=${1:?picks.tsv}; done_file=${2:?done.txt}; touch "$done_file"
G="git -c core.hooksPath=/dev/null -c core.editor=true"
finish() {
  git add -A
  $G commit --amend -q -m "$(git log -1 --format=%B)

上游提交：$1"
  echo -n "${1:0:7} " >> "$done_file"
}
while read -r o r; do
  grep -q "${o:0:7}" "$done_file" && continue
  if ! $G cherry-pick "$r" >/dev/null 2>&1; then
    others=$(git diff --name-only --diff-filter=U | grep -v '^CHANGELOG.md$' | grep -v '^Cargo.lock$')
    if [ -n "$others" ]; then echo "CONFLICT at $(git log -1 --format='%h %s' "$r")"; git status --short | grep -v '^[AMDR] ' | head -30; exit 1; fi
    git checkout --ours CHANGELOG.md Cargo.lock 2>/dev/null; git add CHANGELOG.md Cargo.lock 2>/dev/null
    if ! $G cherry-pick --continue >/dev/null 2>&1; then
      if git diff --cached --quiet && git diff --quiet; then $G cherry-pick --skip >/dev/null 2>&1; echo -n "${o:0:7} " >> "$done_file"; echo "skip(empty) $(git log -1 --format=%s "$r")"; continue; fi
      echo "CONTINUE-FAILED $r"; git status --short | head; exit 1
    fi
    finish "$o"; echo "ok(changelog=ours) $(git log -1 --format='%h %s')"; continue
  fi
  finish "$o"; echo "ok $(git log -1 --format='%h %s')"
done < "$picks"
echo ALL-DONE
