//! 前端会话对 IBus 的一条指令；IBus 层逐条翻成 D-Bus 信号。

use super::view::{CandidateView, PreeditView};

/// 会话处理完一个事件后要对 IBus 做的事，按顺序执行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// 上屏（`CommitText`）。
    Commit(String),

    /// 更新 preedit（`UpdatePreeditText`）；`None` 收起（`HidePreeditText`）。
    Preedit(Option<PreeditView>),

    /// 更新候选表（`UpdateLookupTable`）；`None` 收起（`HideLookupTable`）。
    Candidates(Option<CandidateView>),

    /// 更新辅助文字（`UpdateAuxiliaryText`）；`None` 收起（`HideAuxiliaryText`）。
    Auxiliary(Option<String>),

    /// 登记属性（`RegisterProperties`），带当前中英模式。
    RegisterProperties {
        /// 当前是英文模式。
        english: bool,
    },

    /// 中英模式变了（`UpdateProperty`）。
    Mode {
        /// 现在是英文模式。
        english: bool,
    },
}
