# 跨平台 UI 与自绘渲染器（2026-09-13）

## 起因

主题功能提上日程，同时有人建议用 Flutter 一类跨平台框架，「一套 UI 跑 Windows / macOS / Linux / iOS / Android」。
这篇记录调研结论与选定的方向：**候选窗、状态条、手机键盘这类「显示面」用一个 Rust 自绘渲染器输出位图，各平台只贴位图；
设置程序继续用各平台原生 UI，不自绘。** 在此之前定过「Windows 候选窗改 Direct2D + DirectWrite」，被本方向取代（见末节）。

## 输入法的 UI 分两种

| 种类 | 例子 | 要求 | 结论 |
|---|---|---|---|
| 显示面 | 候选窗、悬浮状态条、手机键盘面板 | 无边框、不抢焦点、置顶、按键后毫秒级出现；输入只有点击与拖拽 | 只画矩形 / 阴影 / 渐变 / 文字，适合自绘 |
| 控件面 | 设置程序 | 表单、表格、滚动、文本框、无障碍 | 需要完整控件库，自绘等于写一个 toolkit，不做 |

现有代码量（2026-09-13）：Core 15,655 行；macOS 候选窗 + 菜单栏 1,644 行、偏好设置 3,041 行；Windows 候选窗 + 状态条 + 托盘 2,057 行、设置程序 1,491 行。
平台层不到总量两成，Core 已经是共享的那「一套」，能收拢的只有显示面的绘制部分。

## 调研（2026-09-13）

- **iOS 键盘扩展内存上限**：社区实测 48–60 MB（phys_footprint），超出由 jetsam 静默杀掉、无崩溃日志、切回系统键盘。
  手机端的拦路虎是内存不是 UI：随包 56 MB 模型与词库必须裁剪；任何带引擎的 UI 框架都塞不进去。
  来源：[The three hard constraints of an iOS keyboard extension](https://dev.to/tbds_2dadf2b626f315902eae/the-three-hard-constraints-of-an-ios-keyboard-extension-46af)、
  [react-native #31910（48 MB 崩溃）](https://github.com/facebook/react-native/issues/31910)。
- **Flutter 进 iOS 扩展**：伞状 issue [flutter #124287](https://github.com/flutter/flutter/issues/124287) 的 `app_extension_safe` 引擎标志仍在开发，
  键盘扩展 [flutter #59753](https://github.com/flutter/flutter/issues/59753) 以链接错误收场；[官方文档](https://docs.flutter.dev/platform-integration/ios/app-extensions) 只覆盖 Share 一类扩展。今天不可用。
- **Flutter 桌面窗口控制**：无边框仍是开放需求 [flutter #71042](https://github.com/flutter/flutter/issues/71042)，透明 / 置顶靠
  [window_manager](https://github.com/leanflutter/window_manager)、flutter_acrylic 这类插件，Windows 上走未公开的 `SetWindowCompositionAttribute`（[flutter #96732](https://github.com/flutter/flutter/issues/96732)）。能做但脆。
- **Avalonia（C#，Skia 自绘）**：`ShowActivated=false` + `Topmost` + `TransparencyLevelHint` 是一等属性，Windows / macOS 能做不抢焦点的置顶窗
  （[讨论 #15805](https://github.com/AvaloniaUI/Avalonia/discussions/15805)）；Linux 上 `ShowActivated` 失效（[#17186](https://github.com/AvaloniaUI/Avalonia/issues/17186)）。
  是排除 WebView 之后最顺的现成框架，但要常驻 .NET 进程（NativeAOT 后仍几十 MB）、macOS 要改成进程外渲染、手机键盘进不去。
- **WebView（Tauri 等）**：已排除，各平台 WebView 渲染不一致，且 Windows 依赖 WebView2 运行时（水杉 issue #68 就是缺它）。
- **Linux**：输入法是 Fcitx5 / IBus 的插件，候选窗由框架画，我们只出引擎，显示面几乎为零。
- **一个被修正的判断**：「候选窗必须原生」说得太死。Windows 候选窗已经由 Server 进程画、DLL 走 IPC 发帧，
  启动成本只付一次，延迟就是那趟 IPC；所以桌面端「常驻进程 + 自绘」在架构上成立。自绘渲染器正是利用这一点，而且不需要引入框架。

## 主题的两条路

主题 = 颜色、字体、圆角、阴影、透明、渐变、布局。

1. **共用主题模型 + 各平台各自的渲染器**（Rime：一份 YAML，Squirrel 与 Weasel 各画各的）。已知痛点是特性长期不对齐。原「D2D 重写」属于这条。
2. **一个渲染器**。显示面一套代码、像素级一致，主题只做一次，用户可以自己写主题文件分享。

选第 2 条。

## 方案

一个 Rust crate 负责布局 + 绘制，输入是 Core 给的 `Frame`（候选、拼音行、译文、分页、提示）与主题数据，输出一张 BGRA 位图与命中区域表；
各平台只做两件事：把位图贴上窗口、把点击 / 拖拽坐标传回来。

| 平台 | 贴位图的方式 | 改动 |
|---|---|---|
| Windows | 分层窗口 `UpdateLayeredWindow` 本来就吃位图，`server/src/ui/layered/` 的合成器换成贴渲染器输出 | 比 D2D 改动更小 |
| macOS | NSPanel 上一个 CALayer，`contents` 设 CGImage；窗口层级 / Space 处理不变 | 几十行 |
| Linux | 不用，Fcitx5 自己画 | 无 |
| iOS / Android | 键盘 View 贴位图，命中测试在 Rust 里做 | 自绘位图占几 MB，符合内存上限 |

技术选型：

- 2D 栅格 `tiny-skia`：纯 Rust，CPU。候选窗 2 倍屏几百乘几百像素，亚毫秒；不上 GPU（vello / wgpu 对这个尺寸是负担）。
- 文字 `cosmic-text`：fontdb 找字体 + rustybuzz 整形 + swash 栅格，带回退链与彩色 emoji。**这是唯一的技术风险点。**
- 主题文件 TOML，随包给内置主题，用户目录可加。

## spike：先验证再投入

在分支 `renderer-spike` 上做，目标是画一行「微明 hello 🙂 日本語」加圆角与柔和阴影，两个平台截图与原生并排比较。要回答四个问题：

1. **彩色 emoji**：Apple Color Emoji 是 sbix 位图，Segoe UI Emoji 是 COLRv0，swash 声称都支持，眼见为实。
2. **中日字形回退**：Han 统一编码下必须带 locale，否则 macOS 上会拿 Hiragino 给中文画日式字形；确认 cosmic-text 按 zh-CN / ja-JP 选到 PingFang / 微软雅黑 / Hiragino / Yu Gothic。
3. **字体库加载**：fontdb 扫全系统字体几百毫秒、几十 MB，要改成按需加载几个家族；手机上尤其。
4. **文字观感**：Windows 上是灰度抗锯齿没有 ClearType，与原生并排看能否接受（Flutter、部分配置的 Chrome 也是灰度）。

验收：四条都过，且首帧渲染耗时与内存不劣于现有 GDI / AppKit 实现，就把 Windows 与 macOS 一起换到渲染器；任一条不过，退回「各平台各自渲染」（Windows 走 D2D）。

## spike 结果（2026-09-13 晚，macOS）

代码：`crates/glimmer-render`（渲染器）、`tools/render-preview`（离线出 PNG + 量宽度）、`apps/macos/src/candidates/bitmap/`（壳侧贴位图；`[general] renderer = "system"` 切回 AppKit 绘制，偏好设置「候选窗口」页可选，是过渡期退路，稳定一个版本后删）。
对比方法：TextEdit 里敲 `nihao`，`screencapture -l` 抓真实候选窗；同一帧人工抄进样例，渲染成 PNG 并排；再把渲染器装进壳抓真机。

| 验收 | 结果 |
| --- | --- |
| 彩色 emoji | 过。Apple Color Emoji（sbix）经 swash 出 RGBA 位图，👋 🙂 与原生一致 |
| 中日字形回退 | 过。locale `zh-CN` 下汉字全部落到 PingFang SC（含「日本語」「骨直曜」），没拿 Hiragino 画中文；假名落到 PingFang HK |
| 字体按需加载 | 过。5 个文件 67 张面，release 1.5–4 ms（mmap，只解析名字表与 cmap）；预览工具整进程峰值 RSS 16 MB |
| 文字观感 | 过（mac）。补了三样才对齐，见下；Windows 灰度抗锯齿待 box 真机 |

首帧：2 倍屏 266×300 pt 一帧 release 1 ms（首帧 6 ms，含字形栅格缓存冷启动）。壳内字体库 3.8 ms。

**对齐原生要补的三样**（都是 CoreText 对系统字体默默做的事）：

1. **光学字号 `opsz`**：SF 是变量字体，CoreText 在 20 pt 以下用 opsz = 17（Text 视觉尺寸），cosmic-text 只设 `wght`，落在缺省的 28（Display），
   小字号英文窄 16–24%。补法：给 cosmic-text 打补丁 `FontSystem::set_optical_size`（shaper 位置与 swash 栅格都带 opsz），放在 [qingjian-team/cosmic-text](https://github.com/qingjian-team/cosmic-text) 的 `glimmer-opsz` 分支（基于 0.19.0，一个提交），workspace `[patch.crates-io]` 钉 rev；上游收了就回 crates.io。
   补丁是全局一个值，字体实例缓存没按它分键；候选窗几种字号都在 20 pt 以下落同一档，够用，做主题字号可调时要改成按字号分键。
2. **`trak` 字距表**：SF 按字号给每个字形加减间距（11 pt +12、12 pt 0、16 pt −40 个字体单位，值 / upem × 字号 = 点），PingFang 没有正常轨。
   渲染器自己解析 `trak`（`fonts/trak.rs`），按字形所用字体各查各的。补完后「hello」「ni'hao」「1/6」「phr. you change」三个字号的宽度与 `NSAttributedString.size()` 到小数点后两位相等。
3. **笔画加深**：CoreText 对文字抗锯齿有一层 gamma，线性混合出来的字偏细，深色背景尤其明显。主题里加 `text_gamma`（浅色 0.85、深色 0.75），放大并排看笔画粗细一致。

**剩下的已知差异**（每个字形 ≤ 1 pt，并排看不出，记着就行）：
- 「·」（U+00B7）CoreText 在中文系统上用 `.CJKSymbolsFallbackSC` 的宽点（5.76 pt），我们用 SF 自己的（3.56 pt）；一行译文差 2 pt。
- 汉字 CoreText 用私有的 `.PingFang UI Text SC`（advance 0.993 em），我们用公开的 PingFang SC（1 em）。
- 假名 CoreText 用 `.CJKSymbolsFallbackSC`，我们用 PingFang HK。
- 云朵是矢量描边近似 SF Symbol `cloud`，比原生略粗。

真机再过了一遍：纠错后的拼音行（删除线 + 淡色剩余）、候选行里的云端词、开着候选窗切系统深浅色、配置 `renderer` 热切换两个方向；1 倍外接屏没设备没验。
假名原先落到 PingFang HK，原因是 ヒラギノ角ゴシック W3 字重 300 被 cosmic-text 的字重匹配筛掉，换 W4 后落 Hiragino Sans。
笔画加深用极性相关的覆盖率 gamma 不是我们独创：[muri #71](https://github.com/MattJackson/muri/issues/71) 得出同样结论（macOS 的字体平滑按前景 / 背景极性调），
[skip.house](https://skip.house/blog/macos-font-rendering) 提到 Patrick Walton 逆向出了 macOS 的膨胀公式（pathfinder），以后想逐像素对齐可以查它。

字体可选：`[general] font` 指定字族名，mac 壳用 CoreText 按字族名查出文件（`CTFontDescriptorCreateMatchingFontDescriptors` → `kCTFontURLAttribute`）交给渲染器只加载那几个文件，系统字体仍在后面当回退；没装就退回系统字体并记日志。真机验过 Kaiti SC 与不存在的字体名。

**结论**：四条都过，mac 上位图渲染器可以替换 AppKit 绘制。下一步 Windows：`server/src/ui/layered/` 换成贴渲染器输出，box 上看灰度抗锯齿与 Segoe UI Emoji（COLRv0）；过了就两端一起换、做主题 TOML。
主题以后要放图片 / 动图 / 花边：渲染器输出就是一张位图，装饰只是多叠几层，不用换底子。

## 不做的事

- 设置程序不自绘。三桌面端设置程序将来若要收拢，另议（Avalonia 是候选，Tauri 因 WebView 排除）。
- 手机键盘的壳仍是 Swift / Kotlin，只是面板内容由渲染器出位图。
- 不引入 Flutter：多一门语言与工具链，收益只覆盖设置页，扩展支持未完成。

## 与既有计划的关系

- 「Windows 候选窗 + 状态条改 Direct2D + DirectWrite」暂停，等 spike 结果。
- `candidate-ui.md` 里「每个平台用原生绘制」与「Windows 用 Direct2D」两句已按本文修正。
