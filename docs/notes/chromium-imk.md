# Chromium 系浏览器下的 IMK 行为（2026-09-15）

ego lite / Chrome 这类 Chromium 壳里，IMK 送来的事件和别的应用不一样，两处都踩过：

## flagsChanged 每个事件送两遍

同一个 Shift 按下（或松开）事件会在 1–2 ms 内送到 `handleEvent:client:` 两次，键码、修饰键、时间戳完全相同。
`ShiftTap` 原来每个事件都把待确认的按下 `take()` 走，第二遍按下时 Shift 已是按住状态、不会重新登记，
随后的松开永远配不上按下，浏览器里单击 Shift 就切不了中英。现在同一个键在按住状态下的重复按下不动待确认状态；
松开只触发一次（第二遍松开时 `down` 已是 false）。

## 换焦点时先 activate 新会话、后 deactivate 旧会话

同一个浏览器窗口里在文本框之间挪焦点，IMK 先给新会话 `activateServer:`，隔十几到两百多毫秒才给旧会话 `deactivateServer:`。
旧会话的 deactivate 若走完整收尾，会把新会话正在敲的拼音原样上屏、收掉菜单栏状态项、停掉配置监视定时器，
表现为「浏览器里有时敲不出字 / 状态项没了，切到别的应用再切回来就好」。现在 `Host::active_controller` 记着当前会话的对象地址，
只有它自己的 deactivate 才拆全局状态，旧会话的 deactivate 只重置自己的 Shift 手势。
