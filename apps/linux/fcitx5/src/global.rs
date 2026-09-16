//! 进程级的后端与日志：插件加载后建一次，所有输入上下文共用（Router 本来就按会话分派状态）。

use std::ffi::{CStr, c_char, c_int};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use glimmer_linux::backend::shared;
use glimmer_linux::{EchoBackend, SharedBackend, build_backend, init_logging};
use tracing_appender::non_blocking::WorkerGuard;

use crate::guard;

/// 进程内唯一的后端；`glimmer_backend_init` 成功后才有。
static BACKEND: OnceLock<SharedBackend> = OnceLock::new();

/// 日志文件写入线程的 guard：活到 `glimmer_shutdown`，那时取出来丢掉以冲刷缓冲。
static LOG_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);

/// 空闲节拍在后端还没建好时的缺省间隔（毫秒）。
const IDLE_TICK_MS: u32 = 1000;

/// 当前后端；还没初始化为 `None`。
pub(crate) fn backend() -> Option<SharedBackend> {
    BACKEND.get().cloned()
}

/// 建进程内的全局后端并装日志，返回 0；已经建过直接返回 0；装配失败返回 -1。
/// `data_root` 是随包资源根（装机是 `/usr/lib/glimmer`），空指针按可执行文件位置找；`echo` 非零用回显后端（只验链路）。
///
/// # Safety
/// `data_root` 为空或指向以 NUL 结尾的字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_backend_init(data_root: *const c_char, echo: c_int) -> c_int {
    let root = (!data_root.is_null())
        // SAFETY: 调用方保证非空时是以 NUL 结尾的字符串
        .then(|| unsafe { CStr::from_ptr(data_root) })
        .map(|root| PathBuf::from(root.to_string_lossy().into_owned()));
    guard("glimmer_backend_init", -1, || {
        if BACKEND.get().is_some() {
            return 0;
        }
        let mut log_guard = LOG_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if log_guard.is_none() {
            *log_guard = init_logging("glimmer-fcitx5");
        }
        drop(log_guard);
        let backend = if echo != 0 {
            tracing::info!("Fcitx5 插件用回显后端");
            shared(EchoBackend::new())
        } else {
            tracing::info!(root = ?root, "Fcitx5 插件装配 Router");
            match build_backend(root) {
                Ok(backend) => shared(backend),
                Err(error) => {
                    tracing::error!(%error, "Router 装配失败");
                    return -1;
                }
            }
        };
        // 并发初始化时后来者的后端丢掉，用先建好的那个
        let _ = BACKEND.set(backend);
        0
    })
}

/// 空闲节拍：推进后端（接模型、看配置、落盘），返回下一次最晚多少毫秒后再调（至少 1）。
#[unsafe(no_mangle)]
pub extern "C" fn glimmer_tick() -> u32 {
    guard("glimmer_tick", IDLE_TICK_MS, || {
        let Some(backend) = backend() else {
            return IDLE_TICK_MS;
        };
        let mut backend = backend
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let wait = backend.tick().as_millis();
        u32::try_from(wait).unwrap_or(u32::MAX).max(1)
    })
}

/// 插件卸载 / fcitx5 退出前调一次：学习数据落盘，冲刷日志。
#[unsafe(no_mangle)]
pub extern "C" fn glimmer_shutdown() {
    guard("glimmer_shutdown", (), || {
        if let Some(backend) = backend() {
            backend
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .shutdown();
        }
        tracing::info!("Fcitx5 插件关闭");
        LOG_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
    });
}
