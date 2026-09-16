//! 输入法 Server 的平台无关部分：Engine 装配（[`assembly`]）与协议分派（[`Router`]）。
//! Windows Server 进程与 Linux IBus 引擎进程共用；传输（命名管道 / D-Bus）与候选窗绘制留在各平台壳里，
//! 经 [`dispatch::CandidateSink`] / [`dispatch::StatusSink`] 注入。

pub mod assembly;
pub mod dispatch;
pub mod error;
pub mod startup;

pub use assembly::{AssemblySpec, LanguageModelFiles, WubiSpec};
pub use dispatch::{Router, RouterConfig};
pub use error::ServerError;
pub use startup::{StartupPaths, build_router};
