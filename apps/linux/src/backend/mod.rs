//! 前端与引擎之间的接缝：IBus 层只认 [`Backend`]，不关心背后是进程内的 Router 还是测试用的假实现（[`EchoBackend`]）。

mod echo;

use std::sync::{Arc, Mutex};

use glimmer_platform::protocol::{ClientMessage, ServerMessage};

pub use echo::EchoBackend;

/// 协议的一端：收 [`ClientMessage`]，需要回话的消息回 [`ServerMessage`]。语义与 Windows DLL ↔ Server 的管道一致，
/// 只是换成了进程内调用。
pub trait Backend {
    /// 发一条消息。`Key` / `Commit` / `Poll` / `Selection` 这类一问一答的消息返回答复；
    /// `OpenSession` / `ModeChanged` / `PositionCandidates` 这类通知返回 `None`。
    fn send(&mut self, message: ClientMessage) -> Option<ServerMessage>;
}

/// 所有引擎实例共用的一个后端（Router 本来就按会话分派状态）。
pub type SharedBackend = Arc<Mutex<dyn Backend + Send>>;

/// 把一个后端包成 [`SharedBackend`]。
pub fn shared(backend: impl Backend + Send + 'static) -> SharedBackend {
    Arc::new(Mutex::new(backend))
}
