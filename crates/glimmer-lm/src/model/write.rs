//! 落盘成 `.qj`：内存里的各段原样写，加上元数据。

use std::path::Path;

use glimmer_format::{Kind, Metadata, Writer};

use crate::error::LmError;

use super::{
    CONTEXT_SUMS_TAG, CONTINUATIONS_TAG, ENTRIES_TAG, HASH_TAG, NgramModel, OFFSETS_TAG,
    SUCCESSORS_TAG, TRIGRAM_OFFSETS_TAG, TRIGRAM_SUCCESSORS_TAG, WORDS_TAG,
};

impl NgramModel {
    /// 写成 `.qj`。`metadata.entries` 会填成二元 + 三元的条数。
    ///
    /// 两张派生表（每个前词的后继计数之和、每个词的不同前词数）也写进去：算它们要整段扫一遍几百万条二元，
    /// 放文件里一共只多 1 MB，换来启动仍然只是 mmap。没有三元时不写那两节，输出与改之前的文件同构。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), LmError> {
        let metadata = Metadata {
            entries: (self.bigram_count() + self.trigram_count()) as u64,
            ..metadata.clone()
        };
        let mut writer = Writer::new(Kind::LanguageModel, &metadata)?
            .section(WORDS_TAG, self.words.as_bytes())
            .section(ENTRIES_TAG, self.entries.as_bytes())
            .section(HASH_TAG, self.index.as_bytes())
            .section(OFFSETS_TAG, self.offsets.as_bytes())
            .section(SUCCESSORS_TAG, self.successors.as_bytes())
            .section(CONTEXT_SUMS_TAG, self.context_sums.as_bytes())
            .section(CONTINUATIONS_TAG, self.continuations.as_bytes());
        if !self.trigram_successors.is_empty() {
            writer = writer
                .section(TRIGRAM_OFFSETS_TAG, self.trigram_offsets.as_bytes())
                .section(TRIGRAM_SUCCESSORS_TAG, self.trigram_successors.as_bytes());
        }
        writer.write_to(path)?;
        Ok(())
    }
}
