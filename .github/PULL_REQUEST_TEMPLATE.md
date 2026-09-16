<!-- 谢谢投稿。下面的清单是合并前维护者会逐条看的，提前对一遍能少一轮往返；不适用的项划掉即可。 -->

## 改了什么

<!-- 一两句：解决什么问题、怎么解的。关联的 issue 写「关闭 #编号」。 -->

## 平台

- [ ] macOS
- [ ] Windows
- [ ] Linux
- [ ] iOS
- [ ] Android
- [ ] 与平台无关（Core / 数据 / 文档）

## 合并前清单

- [ ] `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 本机全过
- [ ] **用户能感知的行为变了（按键、候选、菜单、设置项、配置文件、数据文件），`docs/user/` 对应页面已同步改**，按键改动同时更新 `docs/user/getting-started/keys.md`
- [ ] 一个功能只在一个平台实现时，已在文档里标明平台，并在 PR 里说明另一平台的差距
- [ ] 代码标识符英文、注释与文档中文；新文件有 `//!` 文件头；单文件不超过 800 行
- [ ] 提交信息用 Conventional Commits（`fix(core): ……` / `feat(windows): ……`，说明写中文）；其余约定见 [docs/contributing.md](../docs/contributing.md)
- [ ] 不改 `CHANGELOG.md`（发版时由维护者统一写）

## 怎么验证的

<!-- 真机操作步骤、截图或录屏。Windows 请注明 10 还是 11。 -->
