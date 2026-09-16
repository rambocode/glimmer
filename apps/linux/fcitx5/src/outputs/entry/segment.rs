//! preedit 行的一段。

use std::ffi::CString;

/// 同一种样式的一段 preedit：Fcitx5 的 `Text` 按段带格式，淡色段对应 IBus 那边的灰色区间。
#[derive(Debug)]
pub struct Segment {
    /// 这段文字。
    pub text: CString,

    /// 画淡（光标之后不参与候选的拼音）。
    pub dimmed: bool,
}
