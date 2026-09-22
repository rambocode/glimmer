//! 按编号顺序导出全部计数：`dict-convert supplement` 在已有模型上补词时用。

use super::{NgramModel, word_text};

impl NgramModel {
    /// 词表：(词, 一元计数)，下标即编号，含句首标记。
    pub fn unigrams(&self) -> impl Iterator<Item = (&str, u32)> + '_ {
        self.entries
            .iter()
            .map(|entry| (word_text(&self.words, entry), entry.count))
    }

    /// 全部二元：(前词编号, 后词编号, 计数)，按前词、后词编号升序；编号与 [`Self::unigrams`] 的顺序一致。
    pub fn bigrams(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.offsets
            .windows(2)
            .enumerate()
            .flat_map(move |(previous, range)| {
                self.successors[range[0] as usize..range[1] as usize]
                    .iter()
                    .map(move |s| (previous as u32, s.word, s.count))
            })
    }

    /// 全部三元：(前二词编号, 前词编号, 后词编号, 计数)。段号是二元在 CSR 里的下标，
    /// 所以外层按前二词走一遍二元 CSR、内层再走三元 CSR 就能还原三个词。
    pub fn trigrams(&self) -> impl Iterator<Item = (u32, u32, u32, u32)> + '_ {
        self.offsets
            .windows(2)
            .enumerate()
            .flat_map(move |(earlier, range)| {
                (range[0] as usize..range[1] as usize).flat_map(move |slot| {
                    let previous = self.successors[slot].word;
                    let (start, end) = match self.trigram_offsets.get(slot + 1) {
                        Some(&end) => (self.trigram_offsets[slot] as usize, end as usize),
                        None => (0, 0),
                    };
                    self.trigram_successors[start..end]
                        .iter()
                        .map(move |s| (earlier as u32, previous, s.word, s.count))
                })
            })
    }
}
