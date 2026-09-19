//! 词图上一条部分路径的末尾节点。

use super::arrival::Arrival;

/// 一条部分路径的末尾节点：覆盖到某个位置、以某个词结尾。
#[derive(Debug)]
pub(super) struct Node {
    /// 这个词从第几个音节开始。
    pub start: usize,

    /// 词。
    pub text: String,

    /// 词的音节。
    pub syllables: Vec<String>,

    /// 是占位音节。
    pub placeholder: bool,

    /// 到达这个节点的走法，按得分降序，至少一种。
    pub arrivals: Vec<Arrival>,
}

impl Node {
    /// 最优的那种走法：束剪枝与后继的上文都看它。
    pub fn best(&self) -> &Arrival {
        &self.arrivals[0]
    }
}
