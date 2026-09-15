//! 候选布局的一格；空位不携带候选，由各平台绘制和跳过。

use super::super::Candidate;

/// 排布里的一格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell<'a> {
    /// 本地候选。
    Local(&'a Candidate),

    /// 云端词。
    Cloud(&'a Candidate),

    /// 固定位置之前尚无真实候选的空格。
    Empty,
}

impl<'a> Cell<'a> {
    pub fn candidate(self) -> Option<&'a Candidate> {
        match self {
            Cell::Local(c) | Cell::Cloud(c) => Some(c),
            Cell::Empty => None,
        }
    }
}
