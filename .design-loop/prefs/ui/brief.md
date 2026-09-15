# UI brief：青简 macOS 偏好设置窗口

- 模式：build（用户要求「美化设置界面」，直接改代码并验证）
- approval_mode：autonomous（用户未给参考稿，按 macOS 13+ 系统设置风格自主决定）
- 平台：macOS 13+，AppKit（objc2），窗口固定尺寸不可缩放
- 语言：中文界面
- 证据：`target/debug/qingjian-macos --preferences-preview [页序号] [dark]` 起窗口，`screencapture -l` 截图
- 预算：最多 4 轮实现—评审；self-review（无独立 reviewer）

## 交互契约（必须保留）
- 十个页：通用 / 候选窗口 / 快捷键 / 自定义短语 / 模糊音 / 词库 / 云服务 / 高级 / 统计 / 关于，控件与文案不变
- 控件改动仍经 `changed:` → Host 写配置；底部状态行仍显示配置错误（红）与临时提示（灰）
- 自定义短语编辑表单是独立窗口，不动

## 基线问题（baseline/tab1.png）
1. 十个标签放不下，「关于」被截掉
2. 窗口按最高的一页定高，矮页下面大片空白
3. NSTabView 老式外观，没有分组

## 冻结目标（v1）
- 左侧栏：图标 + 名称的源列表（Sidebar 材质），选中高亮；标题栏透明并贴到侧栏
- 右侧：页标题（17pt semibold）+ 内容；窗口高度随所选页动画变化，最矮不低于侧栏
- 内容分组装进圆角卡片（控件背景色 + 分隔线边框），组间距 14pt
- 深浅色都要正确
