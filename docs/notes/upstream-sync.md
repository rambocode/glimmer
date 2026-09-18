# 同步上游 qingjian

本仓库是 [qingjian-team/qingjian](https://github.com/qingjian-team/qingjian) 的硬分叉（品牌改成微明，crate 名 `glimmer-*`）。上游的改动定期搬过来，方法是
**先把上游历史整树改名，再逐个 cherry-pick**：改名后每个提交的父提交也是改名过的树，cherry-pick 的三方合并只看真正的差异。
两个脚本在 `tools/upstream/`。

## 步骤

1. `git fetch upstream`，找上次同步的上游基点（上一次记录在本页末尾）。
2. 改名：`git branch -f upstream-renamed upstream/main`，然后
   `FILTER_BRANCH_SQUELCH_WARNING=1 git filter-branch -f --tree-filter "$PWD/tools/upstream/rename-tree.sh" -- <基点>^..upstream-renamed`（42 个提交约两分钟）。
   验证：`git grep -il qingjian upstream-renamed -- ':!CHANGELOG.md' ':!Cargo.lock' ':!assets/glossary'` 应只剩 cosmic-text 那个 URL 与 `.qj` 魔数。
3. 开 worktree 与分支：`git worktree add ../glimmer-sync -b sync-upstream main`，`ln -s "$PWD/data" ../glimmer-sync/data`。
4. 列出要挑的：`paste <(git rev-list --reverse <基点>..upstream/main) <(git rev-list --reverse <基点>'..upstream-renamed) > picks.tsv`，删掉要跳过的行
   （上游图标 / 版本号 / 发版脚本这类我们自己定的东西，以及与我们实现撞车、要手工搬的功能）。
5. 在 worktree 里跑 `tools/upstream/pick.sh picks.tsv done.txt`，冲突逐个解。常见冲突：
   - CHANGELOG：脚本自动取我们的；最后把上游条目按平台并进「未发布」小节，issue 号标「上游 #N」。
   - 我们自己拆过目录的文件（macOS `imk/controller/`）：保留我们的拆法，上游对那些文件的小改动按 hunk 手工搬。
   - macOS 偏好设置控件 tag 撞号：上游新加的项改到空号，`Setting` 的正反两个 match 都要改。
   - 双方各加了一个枚举成员（双拼方案这类）：两个都留，`ALL` 长度、各壳的列表、测试一起改。
6. 收尾：`cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、
   `cargo check --target x86_64-pc-windows-gnu -p glimmer-windows-server -p glimmer-windows-tsf -p glimmer-windows-settings`；
   提交信息里的「青简」用 `git filter-branch --msg-filter "sed s/青简/微明/g" -- main..sync-upstream` 换掉；合回 main 后删 worktree 与 `upstream-renamed`。

## 记录

- 2026-09-17：基点 `4fdc8c1`，50 个提交（见 memory）。
- 2026-09-18：基点 `db9f318`，41 个提交，挑了 31 个。跳过：菜单栏图标 `7ff91ac`、安装包改名与 releases.json 加 schema_version `5fe018c`（发版流程是自己的）、版本号 `b5d50b5` `137821a`、
  五笔 86 与混输 `9cfeec4` `535f6fc` `5809370` `7e9ab4c` `ddc49a5` `af3400a`（我们已有三版五笔，混输与 `[general] scheme` 按上游设计手工移植）、
  Linux 架构规划 `eedd1f4`（与我们已落地的实现不同）。保留了我们的 `controller/` 拆法与图标；采用了上游的 `dictionary/` 拆分与 Server 测试拆分。
