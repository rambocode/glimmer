//! 词图的边来源：Viterbi 向它要「从第几个位置到第几个位置有哪些词」。

use std::sync::Arc;

use crate::sentence::SpanWord;

/// 一张词图。位置从 0 到 [`Self::positions`]，一条边是盖住 `[start, end)` 的一个词。
pub trait Lattice {
    /// 有几个位置（拼音是音节数，五笔是编码字母数）。
    fn positions(&self) -> usize;

    /// 一条边最多盖几个位置。
    fn max_span(&self) -> usize;

    /// 词库总词频的对数：语言模型不认识的词按词频兜底时用。
    fn log_total(&self) -> f64;

    /// 盖住 `[start, end)` 的词，已排好、截好；没有就是空。
    fn words(&mut self, start: usize, end: usize) -> Arc<[SpanWord]>;

    /// 这条边在 [`SpanWord::penalty`] 之外还要多扣多少（拼音的原样成词保护）；不扣为 0。
    fn extra_penalty(&self, start: usize, end: usize, word: &SpanWord) -> f64;

    /// 从 `start` 起一条边都没有时，顶在这一个位置上的占位文本。
    fn placeholder(&self, start: usize) -> &str;
}
