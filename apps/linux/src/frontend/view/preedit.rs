//! 要显示在应用光标处的 preedit 行。

use std::ops::Range;

/// preedit 行：整行下划线，光标位置与淡色区间都按字符（`char`）数算，与 IBus 的属性下标一致。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreeditView {
    /// 整行文本。
    pub text: String,

    /// 光标位置（字符数）。
    pub cursor: usize,

    /// 画淡的区间（光标之后不参与候选的拼音）。
    pub dimmed: Vec<Range<usize>>,
}
