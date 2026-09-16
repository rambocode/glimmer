//! 微明的 Linux 前端：IBus 引擎进程。进程内持有后端（下一阶段接 Router），经 D-Bus 与 ibus-daemon 对话。
//!
//! 分层：`ibus` 只管 D-Bus 与 IBus 对象编码；`frontend` 是与 D-Bus 无关的会话状态机；`key` 做 keysym → 协议按键；
//! `backend` 是接 Router 的接缝。装配入口是 [`run`]。

pub mod backend;
pub mod frontend;
pub mod ibus;
pub mod key;

mod error;
mod startup;

pub use backend::{Backend, EchoBackend, RouterBackend, SharedBackend};
pub use error::LinuxError;
pub use startup::{build_backend, init_logging};

/// 以 IBus 组件身份运行：找总线地址、导出工厂、服务到 daemon 断开。阻塞当前线程。
pub fn run(backend: impl Backend + Send + 'static) -> Result<(), LinuxError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(LinuxError::Runtime)?;
    let backend = backend::shared(backend);
    let ticker = runtime.spawn(tick_forever(backend.clone()));
    let result = runtime.block_on({
        let backend = backend.clone();
        async move {
            let address = ibus::address::resolve()?;
            ibus::serve(backend, &address).await
        }
    });
    ticker.abort();
    if let Ok(mut backend) = backend.lock() {
        backend.shutdown();
    }
    result
}

/// 按后端要的节拍一直调 [`Backend::tick`]。锁只在 tick 期间拿，按键处理不会被睡眠挡住。
async fn tick_forever(backend: SharedBackend) {
    loop {
        let wait = match backend.lock() {
            Ok(mut backend) => backend.tick(),
            // 别的线程处理按键时 panic 了：后端状态不可信，停掉节拍，按键那边会各自报错
            Err(_) => return,
        };
        tokio::time::sleep(wait).await;
    }
}
