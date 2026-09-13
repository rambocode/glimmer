//! preedit 的一段文字及其画法。

use super::style::PreeditStyle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreeditSegment {
    /// 文本。
    pub text: String,

    /// 画法。
    pub style: PreeditStyle,
}
