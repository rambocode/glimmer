//! 已有语言模型（`lm.qj`）的计数，展开成 `bigram` 统计时同样的内存形状，好在上面合成补充词的计数。
//! 三元原样带着走：补词只按成分合成一元与二元（理由见 `bigram` 的模块注释），三元不动也不能丢。

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use glimmer_lm::NgramModel;

use crate::bigram::Vocabulary;
use crate::error::ConvertError;

/// 语言模型的一元 / 二元计数，编号与 `lm.qj` 的词表顺序一致，补进来的词接在后面。
pub(crate) struct LmCounts {
    /// 编号 → 词。
    pub(crate) words: Vec<String>,

    /// 词 → 编号。
    pub(crate) ids: HashMap<String, u32>,

    /// 编号 → 一元计数。
    pub(crate) unigram: Vec<u64>,

    /// (前词编号 << 32 | 后词编号) → 二元计数，与 `bigram::synthesize_phrases` 同一种键。
    pub(crate) bigram: HashMap<u64, u32>,

    /// 模型里原有的二元，按 (前词, 后词) 编号升序，写回 TSV 时保持原顺序。
    rows: Vec<(u32, u32, u32)>,

    /// 模型里原有的三元 (前二词, 前词, 后词, 计数)，原样写回。
    triples: Vec<(u32, u32, u32, u32)>,
}

impl LmCounts {
    /// 打开 `lm.qj` 并展开全部计数。
    pub(crate) fn load(path: &Path) -> Result<Self, ConvertError> {
        let model = NgramModel::from_path(path)?;
        let mut words = Vec::with_capacity(model.word_count());
        let mut ids = HashMap::with_capacity(model.word_count());
        let mut unigram = Vec::with_capacity(model.word_count());
        for (id, (word, count)) in model.unigrams().enumerate() {
            ids.insert(word.to_owned(), id as u32);
            words.push(word.to_owned());
            unigram.push(u64::from(count));
        }
        let rows: Vec<(u32, u32, u32)> = model.bigrams().collect();
        let triples: Vec<(u32, u32, u32, u32)> = model.trigrams().collect();
        let bigram = rows
            .iter()
            .map(|&(first, second, count)| ((u64::from(first) << 32) | u64::from(second), count))
            .collect();
        tracing::info!(words = words.len(), bigrams = rows.len(), trigrams = triples.len(), path = %path.display(), "语言模型计数已展开");
        Ok(Self {
            words,
            ids,
            unigram,
            bigram,
            rows,
            triples,
        })
    }

    /// 把 `text` 按分词词表切开、换成模型里的编号；切不成两个以上的词、或有成分不在模型里时返回 `None`。
    /// `vocabulary` 里不能有 `text` 自己（调用方先摘掉），否则它只切出它自己。
    pub(crate) fn components(&self, text: &str, vocabulary: &Vocabulary) -> Option<Vec<u32>> {
        let mut tokens = Vec::new();
        vocabulary.segment(text, &mut tokens);
        if tokens.len() < 2 {
            return None;
        }
        tokens
            .iter()
            .map(|token| {
                let word = &vocabulary.words[(*token)? as usize];
                self.ids.get(word).copied()
            })
            .collect()
    }

    /// 在词表末尾加一个词（一元先记 0，合成后再填），返回它的编号。
    pub(crate) fn push_word(&mut self, text: &str) -> u32 {
        let id = self.words.len() as u32;
        self.words.push(text.to_owned());
        self.ids.insert(text.to_owned(), id);
        self.unigram.push(0);
        id
    }

    /// 写 `lm-unigram.tsv` / `lm-bigram.tsv` / `lm-trigram.tsv`：原有二元原样照抄，`extra` 是合成的行；
    /// 一元只写次数大于 0 的词（与 `bigram` 一致）；三元原样写回（模型里没有三元时不写这张表）。
    pub(crate) fn write_tsv(
        &self,
        extra: &[(u64, u32)],
        out_dir: &Path,
    ) -> Result<(), ConvertError> {
        let bigram_path = out_dir.join("lm-bigram.tsv");
        let mut writer = BufWriter::new(File::create(&bigram_path)?);
        writeln!(
            writer,
            "# 由 glimmer-dict-convert supplement 在已有 lm.qj 上补合成计数写回。前词\\t后词\\t计数"
        )?;
        for &(first, second, count) in &self.rows {
            writeln!(
                writer,
                "{}\t{}\t{count}",
                self.words[first as usize], self.words[second as usize]
            )?;
        }
        for (key, count) in extra {
            let first = &self.words[(key >> 32) as usize];
            let second = &self.words[(key & 0xFFFF_FFFF) as usize];
            writeln!(writer, "{first}\t{second}\t{count}")?;
        }
        writer.flush()?;

        let unigram_path = out_dir.join("lm-unigram.tsv");
        let mut writer = BufWriter::new(File::create(&unigram_path)?);
        writeln!(
            writer,
            "# 由 glimmer-dict-convert supplement 在已有 lm.qj 上补合成计数写回。词\\t计数；<s> 是句首标记"
        )?;
        let mut written = 0usize;
        for (word, count) in self.words.iter().zip(&self.unigram) {
            if *count > 0 {
                writeln!(writer, "{word}\t{count}")?;
                written += 1;
            }
        }
        writer.flush()?;

        if !self.triples.is_empty() {
            let trigram_path = out_dir.join("lm-trigram.tsv");
            let mut writer = BufWriter::new(File::create(&trigram_path)?);
            writeln!(
                writer,
                "# 由 glimmer-dict-convert supplement 原样写回。前前词\t前词\t后词\t计数"
            )?;
            for &(earlier, previous, word, count) in &self.triples {
                writeln!(
                    writer,
                    "{}\t{}\t{}\t{count}",
                    self.words[earlier as usize],
                    self.words[previous as usize],
                    self.words[word as usize]
                )?;
            }
            writer.flush()?;
            tracing::info!(path = %trigram_path.display(), trigrams = self.triples.len(), "三元表已原样写回");
        }
        tracing::info!(
            unigram = %unigram_path.display(),
            words = written,
            bigram = %bigram_path.display(),
            bigrams = self.rows.len() + extra.len(),
            "语言模型 TSV 已写出"
        );
        Ok(())
    }
}
