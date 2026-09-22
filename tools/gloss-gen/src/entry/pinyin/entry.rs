//! 一个词的拼音标注。

use serde::{Deserialize, Serialize};

use super::PinyinReading;

/// 一个中文词的拼音标注（多音字按词义定读音），JSONL 里一行一个。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinyinEntry {
    /// 中文词。
    pub word: String,

    /// 主读音：每个字一个音节，不带声调，ü 写 v（`chong qing`）。旧版文件只有这一项。
    pub pinyin: Vec<String>,

    /// 全部常用读音与占比，主读音在第一个；一个词不同词义读音不同时有多条（一行 `yi hang` 一行字 / `yi xing` 一行人）。
    /// 旧版文件没有这一项，读回时为空，按只有主读音处理。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readings: Vec<PinyinReading>,
}
