//! Rime 码表头里的一条造词规则（`encoder.rules` 的一项）。

/// 造词规则：词长落在 `[min_length, max_length]` 时按 `formula` 取码（`AaAbBaBb` 这类）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderRule {
    /// 适用的最小词长（字数）。
    pub min_length: usize,

    /// 适用的最大词长（字数）；`length_equal` 时与 `min_length` 相等。
    pub max_length: usize,

    /// 取码公式，大写字母是第几个字、小写是该字全码第几位、`Z` 是末字。
    pub formula: String,
}
