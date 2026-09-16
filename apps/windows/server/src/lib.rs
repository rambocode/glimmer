//! Server 的库部分：传输（[`ipc`]）与自绘 UI；Engine 装配与协议分派来自平台无关的 `glimmer-server`，
//! 这里原样再导出，供 bin 与集成测试共用。

pub mod ipc;
/// 候选窗口 / 状态条的自绘线程；仅 Windows。
#[cfg(windows)]
pub mod ui;

pub use glimmer_server::{
    AssemblySpec, LanguageModelFiles, Router, RouterConfig, ServerError, WubiSpec, assembly,
    dispatch, error,
};
