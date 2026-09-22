//! 一个词的生成结果，JSONL 里一行一个。

mod english_gloss_entry;
mod gloss_entry;
mod japanese_sense;
mod pinyin;

pub use english_gloss_entry::EnglishGlossEntry;
pub use gloss_entry::GlossEntry;
pub use japanese_sense::JapaneseSense;
pub use pinyin::{PinyinEntry, PinyinReading};
