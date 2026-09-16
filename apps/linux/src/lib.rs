//! 微明的 Linux 前端：IBus 引擎进程。进程内持有后端（下一阶段接 Router），经 D-Bus 与 ibus-daemon 对话。
//!
//! 分层：`ibus` 只管 D-Bus 与 IBus 对象编码；`frontend` 是与 D-Bus 无关的会话状态机；`key` 做 keysym → 协议按键；
//! `backend` 是接 Router 的接缝。装配入口是 [`run`]。

pub mod backend;
pub mod frontend;
pub mod ibus;
pub mod key;

mod error;

pub use backend::{Backend, EchoBackend, SharedBackend};
pub use error::LinuxError;

/// 以 IBus 组件身份运行：找总线地址、导出工厂、服务到 daemon 断开。阻塞当前线程。
pub fn run(backend: impl Backend + Send + 'static) -> Result<(), LinuxError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(LinuxError::Runtime)?;
    let backend = backend::shared(backend);
    runtime.block_on(async move {
        let address = ibus::address::resolve()?;
        ibus::serve(backend, &address).await
    })
}
