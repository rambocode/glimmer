//! 一个 IBus 引擎实例的前端状态（对应 tsf 的 `TextService`）：持有后端、当前协议会话、中英模式、组句与否，
//! 把 IBus 的事件翻成 [`ClientMessage`]、把 [`ServerMessage`] 摊成 [`Output`]。不依赖 D-Bus，单测直接驱动。
//!
//! 按职责拆成子模块：按键与中英切换在 `key`，焦点 / 重置 / 上屏在 `focus`，其余 IBus 事件（翻页、点选、光标、前文、轮询）在 `context`。

mod context;
mod focus;
mod key;

use std::sync::atomic::{AtomicU64, Ordering};

use glimmer_platform::protocol::{
    ClientMessage, Frame, KeyOutcome, ScreenRect, ServerMessage, SessionId,
};

use super::output::Output;
use super::view::{auxiliary_text, candidate_view, preedit_view};
use crate::backend::SharedBackend;
use crate::key::ShiftTap;

/// 进程内会话号，所有引擎实例共用，单调递增。
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// 一个引擎实例的前端状态。
pub struct Session {
    /// 协议的另一端。
    backend: SharedBackend,

    /// 当前打开的协议会话；失焦后关掉，下次获焦 / 按键时再开。
    id: Option<SessionId>,

    /// 获焦时 IBus 报的客户端名（`gtk3-im:firefox` 之类），开会话时当 `app` 带给 Router。
    app: Option<String>,

    /// 持久的英文模式（单击 Shift / 点属性切换）。
    english: bool,

    /// 正在组句（上一帧不空）。
    composing: bool,

    /// 单击 Shift 的判定。
    shift_tap: ShiftTap,

    /// 应用报来的光标前文（最多 [`SURROUNDING_CHARS`] 字），组句起始时送给 Router。
    before_cursor: Option<String>,

    /// 应用报来的光标矩形，组句中送给 Router 摆候选窗口。
    cursor: Option<ScreenRect>,

    /// 输入框是私密的（密码 / PIN / 应用要求不学习）。
    private: bool,

    /// 上次报给 Router 的私密状态；`None` 是这个会话还没报过。
    reported_private: Option<bool>,

    /// 上次画出去的帧，轮询结果没变就不重画。
    last_frame: Frame,
}

/// 送给 Router 的光标前文字数上限（与协议 `Surrounding` 的约定一致）。
const SURROUNDING_CHARS: usize = 64;

impl Session {
    /// 新的引擎实例状态，中文模式、没有会话。
    pub fn new(backend: SharedBackend) -> Self {
        Self {
            backend,
            id: None,
            app: None,
            english: false,
            composing: false,
            shift_tap: ShiftTap::default(),
            before_cursor: None,
            cursor: None,
            private: false,
            reported_private: None,
            last_frame: Frame::default(),
        }
    }

    /// 正在组句：IBus 层据此开关轮询。
    pub fn composing(&self) -> bool {
        self.composing
    }

    /// 当前是不是英文模式。
    pub fn english(&self) -> bool {
        self.english
    }

    /// 发一条消息等答复。后端锁被毒化（别的线程 panic）时照样取出来用：状态是协议消息级的，不会半更新。
    fn call(&self, message: ClientMessage) -> Option<ServerMessage> {
        let mut backend = self
            .backend
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        backend.send(message)
    }

    /// 发一条不需要答复的通知。
    fn notify(&self, message: ClientMessage) {
        let _ = self.call(message);
    }

    /// 当前会话号；还没开就开一个，并补报私密状态。
    fn session(&mut self) -> SessionId {
        if let Some(id) = self.id {
            return id;
        }
        let id = SessionId(NEXT_SESSION.fetch_add(1, Ordering::Relaxed));
        self.notify(ClientMessage::OpenSession {
            session: id,
            app: self.app.clone(),
            protocol: glimmer_platform::protocol::PROTOCOL_VERSION,
        });
        self.id = Some(id);
        self.reported_private = None;
        self.report_privacy();
        id
    }

    /// 私密状态与上次报的不同才发（对应 tsf 客户端的 `set_private`）。
    fn report_privacy(&mut self) {
        let Some(session) = self.id else {
            return;
        };
        if self.reported_private == Some(self.private) {
            return;
        }
        // 没报过且不私密时 Router 本来就按不私密起算，省一条消息。
        if self.reported_private.is_none() && !self.private {
            self.reported_private = Some(false);
            return;
        }
        self.notify(ClientMessage::Privacy {
            session,
            private: self.private,
        });
        self.reported_private = Some(self.private);
    }

    /// 处理一条答复：上屏文本与帧追加到 `out`，返回这次按键算不算被吃掉。
    /// `RequestSelection`（翻译选中文字）第一版回空选区。
    // TODO: 经 IBus 的 surrounding text 取选区（anchor ≠ cursor 时两者之间就是选区）。
    fn apply(&mut self, reply: ServerMessage, out: &mut Vec<Output>) -> bool {
        match reply {
            ServerMessage::KeyResult {
                outcome,
                commit,
                frame,
                ..
            } => {
                let consumed = outcome == KeyOutcome::Consumed;
                // 放行的键 Router 没动缓冲区，不碰 preedit（与 DLL 一致），只同步组句状态。
                if !consumed {
                    self.composing = !frame.is_empty();
                    return false;
                }
                if let Some(text) = commit {
                    out.push(Output::Commit(text));
                }
                self.render(frame, out);
                true
            }
            ServerMessage::Update { frame, .. } => {
                self.render(frame, out);
                false
            }
            ServerMessage::Committed { text, .. } => {
                if let Some(text) = text.filter(|text| !text.is_empty()) {
                    out.push(Output::Commit(text));
                }
                self.render(Frame::default(), out);
                false
            }
            ServerMessage::RequestSelection { session, request } => {
                let message = ClientMessage::Selection {
                    session,
                    request,
                    text: String::new(),
                    rect: self.cursor.unwrap_or(ScreenRect {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                    }),
                };
                if let Some(reply) = self.call(message) {
                    self.apply(reply, out);
                }
                true
            }
            ServerMessage::ModeSync { .. } => false,
        }
    }

    /// 画一帧：preedit、辅助文字、候选表依次更新或收起。
    fn render(&mut self, frame: Frame, out: &mut Vec<Output>) {
        self.composing = !frame.is_empty();
        out.push(Output::Preedit(preedit_view(&frame)));
        out.push(Output::Auxiliary(auxiliary_text(&frame)));
        out.push(Output::Candidates(candidate_view(&frame)));
        self.last_frame = frame;
    }
}
