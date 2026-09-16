//! 候选表里的一行，已换成 C 字符串。

use std::ffi::CString;

/// 一个候选：标签、文字、译词注解。
#[derive(Debug)]
pub struct Candidate {
    /// 数字键标签（`1`…`9`，第十个起为空串）。
    pub label: CString,

    /// 候选文字。
    pub text: CString,

    /// 学习语言的译词；没有为 `None`。
    pub comment: Option<CString>,
}
