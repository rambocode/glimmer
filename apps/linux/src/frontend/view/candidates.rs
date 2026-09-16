//! 要画进 IBus 候选表的当前页。

use super::row::CandidateRow;

/// 当前页候选。翻页由 Router 管，IBus 只看到这一页；面板的翻页按钮经 `PageUp` / `PageDown` 回到 Router。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateView {
    /// 本页候选，按顺序。
    pub rows: Vec<CandidateRow>,

    /// 高亮的候选下标（页内）。
    pub highlight: usize,

    /// 竖排（`true`）还是横排。
    pub vertical: bool,
}
