# 交付：青简 macOS 偏好设置窗口美化

状态：accepted（self-review，四关通过；深浅色、十页全部截图核对）

## 改了什么
- `apps/macos/src/preferences/sidebar/`：新增，侧栏源列表（SF Symbol + 页名，Sidebar 材质）
- `apps/macos/src/preferences/pager.rs`：新增，右侧翻页器，页标题带，窗口高度随页伸缩
- `apps/macos/src/preferences/layout.rs`：`end_group()` 分组，每组一张圆角卡片；`plain()` 给短语编辑表单
- `apps/macos/src/preferences/window.rs`：装配改为侧栏 + 翻页器；超过 620 pt 的页装进滚动视图
- `apps/macos/src/preferences/panel.rs`：透明标题栏、隐藏标题、可拖背景
- `apps/macos/src/preferences/controls.rs`：说明小字高度按 `cellSizeForBounds` 实测
- `apps/macos/src/main.rs` / `preferences/mod.rs`：`--preferences-preview [页] [dark]` 开发预览入口
- 文档：`docs/design/candidate-ui.md`、`docs/design/architecture.md`

## 证据
- 基线：`baseline/tab1.png`
- 第 1 轮：`rounds/1/p0..p9.png`, `p2-dark.png`（发现：快捷键页过高；说明小字估行偏多）
- 第 2 轮：`rounds/2/p2.png`（滚动）、`p9.png`（行高实测）
- 第 3 轮：`rounds/3/`（余下页面 + 深色通用页）

## 未验证
- 短语编辑 sheet（需要 Host，预览入口起不了）；只改了 `Layout::plain` 的高度算法，与原来一致
- 真机 IMK 进程里的打开 / 关闭（需 `bundle.sh --install`）
