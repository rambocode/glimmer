//! 一个词的一个读音与占比。

use serde::{Deserialize, Serialize};

/// 一个读音及其在这个词的用例里大致占的比例。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinyinReading {
    /// 每个字一个音节，格式同 [`super::PinyinEntry::pinyin`]。
    pub pinyin: Vec<String>,

    /// 占比，0–1，同一个词的各读音合计为 1。
    pub share: f64,
}
