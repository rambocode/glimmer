# IMK 输入法菜单：没有 action 的菜单项一改显示状态就崩

2026-09-16，接五笔时踩到。

## 现象

偏好设置里把五笔关掉（或热加载改配置关掉），输入法进程 1 到 3 秒后退出。崩溃报告：

```text
*** CFRelease() called with NULL ***
CoreFoundation  CFRelease
InputMethodKit  -[_IMKServerLegacy _copySynchronizedActions:withMenuItems:]
InputMethodKit  -[_IMKServerLegacy menusDictionary_CommonWithController:]
InputMethodKit  __57-[_IMKServerLegacy menusDictionaryWithClientAsync:reply:]_block_invoke
```

macOS 26 / 27 都复现，两次独立触发栈完全一样。

## 原因

输入法菜单（`IMKInputController::menu`）里新加了一行「输入方案：86 五笔」：没有 action、`setEnabled(false)`，五笔开着显示、关着 `setHidden(true)`。
IMK 把菜单同步给系统时按项记 action（`_copySynchronizedActions`），没有 action 的项记成 NULL；这一项的显示状态一变，
它释放旧记录时对 NULL 记录 `CFRelease`，直接 `SIGTRAP`。菜单里原来也有没 action 的行（版本号、配置错误提示），
但它们从没在运行中改过显示状态，所以从没触发过。

## 修法

`menubar/menu.rs::action_item`：所有菜单项都绑 `menuAction:` 选择器和 target，没有动作的行 tag 为 0，
`MenuAction::from_tag(0)` 解不出动作就不做事（这些行本来就 disabled，点不到）。
方案行改成常驻，只改标题（「输入方案：拼音」/「输入方案：86 五笔」），不再隐藏 / 显示切换。

## 教训

- IMK 的菜单是跨进程同步的，`NSMenuItem` 上任何「运行中变化」（隐藏、增删）都要按 IMK 的口味来；
  要显示状态文字，优先改标题而不是增删 / 隐藏项。
- 输入法进程崩溃不弹窗，只在 `~/Library/Logs/DiagnosticReports/glimmer-macos-*.ips` 留报告；
  热加载类改动测完要看一眼进程还在不在（`ps -eo pid,comm | grep glimmer-macos`）与报告数有没有涨。
- 配置监视器只在会话激活时每秒查一次，用脚本改配置复现时焦点要停在有输入法会话的文本框里，否则改了也不加载。
