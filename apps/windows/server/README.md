# glimmer-windows-server

微明 Windows 输入法的 **Server 进程**（bin `glimmer-server`）：持有唯一的输入内核
`glimmer-core::Engine`，跑在所有应用进程之外，通过命名管道给 TSF DLL（`../tsf`）提供候选。
Windows 端的整体结构、为什么内核要在进程外、构建与注册步骤，见 `../README.md`。

## 模块

- `assembly` / `dispatch::Router`：来自平台无关的 `crates/glimmer-server`（与 Linux 引擎进程共用），`lib.rs` 原样再导出。
- `ipc`：长度前缀帧的收发循环；`ipc::pipe`（`cfg(windows)`）在 `\\.\pipe\glimmer` 上起命名管道服务。
- `ui`（`cfg(windows)`）：候选窗与悬浮状态条的自绘 UI 线程，作为 Router 的 `CandidateSink` / `StatusSink`。

bin `src/main.rs` 只做配置读取、Engine 装配与启动；逻辑都在库部分；Router 的集成测试在 `crates/glimmer-server/tests/`，这里的 `tests/` 只测传输（`ipc_loop` 内存流、`pipe` 命名管道）。
