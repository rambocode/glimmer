# 开发约定

改代码前先看这一页：架构上不能越的线、代码怎么组织、版本号与提交信息怎么写、改了行为要同步哪些文档。
设计的来龙去脉在 [design/architecture.md](design/architecture.md)，各 crate 的实现要点在 [notes/crate-notes.md](notes/crate-notes.md)。

## 架构约束（不要违反）

长版与理由见 [design/architecture.md](design/architecture.md)。

- **Core 与平台层严格解耦。** `glimmer-core` 及其兄弟 crate 必须平台无关：词库、拼音解析、候选生成、排序、学习、翻译、文本变换全部属于 Core。
  平台层（IMK / TSF / IBus-Fcitx）只做两件事：把系统输入事件翻译成 Core 的输入，把 Core 返回的帧画到候选窗口。
  **平台层里不允许出现排序逻辑、词库访问、翻译调用或文本变换。** 判断标准：把 IMK 换成 TSF，不应该需要改 Core 的任何一行。
- **一个候选词只显示一种辅助语言。** 用户配置 Primary Language + 单个 Learning Language。不要设计成 `translations: Vec<Translation>` 或
  `HashMap<Lang, String>` 这类多语言并列的数据结构，那会在 API 层面把「一次只学一种语言」这条产品原则给破坏掉。翻译是候选词的 annotation（可选、单条）。
- **输入优先于学习。** 任何为学习功能增加的延迟、弹窗、UI 干扰都是设计错误。翻译查询不能阻塞候选生成，Core 必须能在翻译尚未就绪时先返回候选。
- **输入方案是配置项，不是模式。** 双拼、注音这类键盘方案放 `[general]` 里当设置，中 / 英切换始终是布尔；新方案不能改变别的方案的既定按键行为（[user/getting-started/keys.md](user/getting-started/keys.md)）。
- **显示面自绘、控件面原生。** 候选窗、拼音行、状态条这类显示面由渲染器出位图各平台贴图（主题靠它）；偏好设置、菜单、安装器用各平台原生控件。

## 代码组织

- **一个类型一个文件。** 一个 struct / enum / trait 及其 impl 单独一个文件；模块文件只做 `mod` 声明、re-export 与自由函数，不把一个 crate 平铺在 `lib.rs` 里。
- **子模块用目录。** 有子模块的模块用 `foo/mod.rs`，**不用** `foo.rs` + `foo/` 并列。
- **同词干的兄弟文件收进目录，绝不用文件名前缀分组。** `key_event.rs` + `key_outcome.rs` → `key/mod.rs` + `key/{event,outcome}.rs`，哪怕没有 `key.rs` 这个共同父文件；
  `query/` + `querying.rs` 这种也不行。判断：两个及以上文件名共享一段前缀且同属一个概念，就收进以那段前缀命名的目录。
- **一个职责连带它专用的类型收进一个目录。** `engine/commit/mod.rs` 放上屏部分，`chain.rs` / `last.rs` 放只有它用的类型。
- **大类型的 `impl` 按职责拆成子模块。** 每个文件一个 `impl Foo { … }`（`host/settings.rs` 这样），结构体与构造留在 `mod.rs`，跨文件用到的私有方法标 `pub(super)`。
- **文件长度。** 单文件不超过 800 行，目标 500 行以内；测试超过 200 行搬到 `tests.rs`（多时 `tests/` 按主题分文件）。
- **导入写精确路径。** 不用 `use super::*` / `use foo::*`（`#[cfg(test)] mod tests` 里的 `use super::*` 除外）。
  子文件从父模块拿的类型写 `use super::{Engine, Candidate};`，父模块没定义的直接 `use crate::…` 或 `use <crate>::…`，不绕 `super::` 转手。
  存量的 glob 改到那个文件时顺手换掉，不专门清扫。

## 命名与注释

- 代码标识符一律英文，注释与文档用中文；`thiserror` 的 `#[error]` 文案用英文，日志与 UI 文案用中文。
- 新文件都要有 `//!` 文件头；结构体 / 枚举字段逐条 `///` 注释，字段之间空一行。
- 不写装饰性分隔注释（`// ====`），提交钩子会拦。

## 依赖与配置

- 依赖用 `cargo add` 加，共用包提到根 `[workspace.dependencies]`；错误用 `thiserror` 不用 `anyhow`；日志用 `tracing` 门面。
- 快捷键一律进 `[shortcut]` 可配置，不写死键码。

## 版本号

- `crates/*` 用 `version.workspace = true`；**`apps/*` 各壳是独立发布的产品，写死自己的 `version`**（Windows 读 `server/Cargo.toml`）。
- 发版之间带 `-dev`（mac `0.1.3-dev`、win `0.1.0-alpha.3-dev`），打包脚本再接 git 短哈希成 `0.1.3-dev-1a2b3c4`（脏加 `+`，Cargo.toml 里只写 `-dev`）。
- 发版提交去掉 `-dev` 打标签（`macos-v<版本>` / `windows-v<版本>`），标签后再改成下一个 `-dev`；带 `-dev` 的标签 CI 拒绝；pkg / Inno 只认数字点号。

## 提交信息

- 中文冒号格式：`macOS：……` / `Windows：……` / `Core：……` / `仓库：……` / `文档：……`；不加 AI 署名。

## 文档同步

- 实现与规划分歧时以代码为准并改文档。技术方向写 `docs/design/`，计划写 `docs/plan/`，工程记录写 `docs/notes/`；不往 README 里加技术内容。
- **用户能感知的行为改了（按键、菜单、偏好设置、配置文件、数据文件），同一个提交里改 `docs/user/` 对应的页**，按键改动同时改 `keys.md`。
- 改了某个 crate 的实现要点（数据文件、常数、生成命令），同步 `docs/notes/crate-notes.md`。

## 提交前检查

- 钩子：`.githooks/pre-commit`（禁装饰性分隔注释 + fmt + clippy），`.githooks/pre-push`（全 workspace 测试）；`git config core.hooksPath .githooks` 启用一次。
- 排序 / 整句 / 纠错的改动先跑 `apps/cli` 再合。

## CI 与发版

- CI 三个 job（Linux 全量 / macOS 壳 / Windows 三 crate）都 `--locked`；Dependabot 升 actions；每周 `cargo audit`。
- 发版：推 `<平台>-v<版本>` 标签触发 `release.yml`，门禁是版本号 = 标签且不带 -dev、标签在 main 上、产品数据按 SHA256SUMS 校验。
- CHANGELOG 手写、发版时由维护者统一改（PR 不动它）。流程与 Secrets 见 [notes/release.md](notes/release.md)。

## 外部 PR

- 从 main 开分支，一个 PR 只做一件事、只碰一个平台（Core 改动单独一个）。
- 维护者对着 main 审，squash 合并保留作者署名。PR 模板里的合并前清单就是审核标准。
