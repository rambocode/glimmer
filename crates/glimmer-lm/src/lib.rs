//! 整句转换用的词级 n-gram 语言模型：实现 Core 的 `LanguageModel` trait。
//! 从 TSV 加载（`from_paths`），或直接映射 `.qj`（`from_path`，`dict-convert pack lm` 生成，启动近零耗时）。
//!
//! TSV 由 `tools/dict-convert bigram` 从语料统计得到：
//!
//! ```text
//! lm-unigram.tsv    词\t计数              （`<s>` 是句首标记，计数为句子数）
//! lm-bigram.tsv     前词\t后词\t计数
//! lm-trigram.tsv    前前词\t前词\t后词\t计数   （可选；没有它就是纯二元模型）
//! ```
//!
//! 概率是三元 → 二元 → 一元的回退，每层绝对折扣（见 [`Smoothing`]）：
//! P(w|u,v) = max(c(u,v,w)−D₃, 0)/c(u,v) + γ₃·P(w|v)，P(w|v) 同式再退到一元。
//! 模型里没有的词交回 Core 用词库词频兜底。

mod error;
mod model;
mod smoothing;
mod successor;
mod word_entry;

pub use error::LmError;
pub use model::{NgramModel, SENTENCE_START};
pub use smoothing::{BackoffMode, Smoothing};
