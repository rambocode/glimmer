//! 从 TSV 解析：一元表按出现顺序编号，二元、三元排序后各分组成 CSR。

use std::collections::HashSet;
use std::path::Path;

use glimmer_format::{Table, Text};

use crate::error::LmError;
use crate::smoothing::Smoothing;
use crate::successor::Successor;
use crate::word_entry::WordEntry;

use super::{NgramModel, build_index, word_text};

impl NgramModel {
    /// 从 TSV 解析。`trigram` 不给就是纯二元模型。
    pub fn from_paths(
        unigram: &Path,
        bigram: &Path,
        trigram: Option<&Path>,
    ) -> Result<Self, LmError> {
        let trigram = match trigram {
            Some(path) => std::fs::read_to_string(path)?,
            None => String::new(),
        };
        Self::parse(
            &std::fs::read_to_string(unigram)?,
            &std::fs::read_to_string(bigram)?,
            &trigram,
        )
    }

    /// 解析三张表的内容（`trigram` 可以是空串）。一元表里没有的词、挂不上二元的三元都跳过。
    pub fn parse(unigram: &str, bigram: &str, trigram: &str) -> Result<Self, LmError> {
        let (words, entries) = parse_unigram(unigram)?;
        let index = build_index(&words, &entries);
        let lookup = |word: &str| {
            glimmer_format::hash::find(&index, word, |id| word_text(&words, &entries[id as usize]))
        };
        let mut pairs: Vec<(u32, u32, u32)> = Vec::new();
        for (line_number, raw) in bigram.lines().enumerate() {
            let Some((words, count)) = fields(raw, 2, "lm-bigram.tsv", line_number)? else {
                continue;
            };
            // 一元表里没有的词直接跳过：没有 c(v) 也算不出条件概率
            let (Some(first), Some(second)) = (lookup(words[0]), lookup(words[1])) else {
                continue;
            };
            pairs.push((first, second, count));
        }
        pairs.sort_unstable_by_key(|&(v, w, _)| (v, w));
        pairs.dedup_by_key(|&mut (v, w, _)| (v, w));
        let (offsets, successors) = group(&pairs, entries.len());
        let (trigram_offsets, trigram_successors) =
            parse_trigram(trigram, &offsets, &successors, lookup)?;
        let mut model = Self {
            words: Text::Owned(words),
            entries: Table::Owned(entries),
            index: Table::Owned(index),
            offsets: Table::Owned(offsets),
            successors: Table::Owned(successors),
            trigram_offsets: Table::Owned(trigram_offsets),
            trigram_successors: Table::Owned(trigram_successors),
            context_sums: Table::default(),
            continuations: Table::default(),
            total: 0.0,
            continuation_total: 0.0,
            start: None,
            smoothing: Smoothing::DEFAULT,
            metadata: None,
        };
        model.finish();
        tracing::debug!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            trigrams = model.trigram_count(),
            "语言模型加载完成"
        );
        Ok(model)
    }
}

/// 一元表 → (词 arena, 词表)。重复的词只认第一次。
fn parse_unigram(unigram: &str) -> Result<(String, Vec<WordEntry>), LmError> {
    let mut words = String::new();
    let mut entries: Vec<WordEntry> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for (index, raw) in unigram.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (word, count) = line.split_once('\t').ok_or(LmError::Format {
            file: "lm-unigram.tsv",
            line: index + 1,
            reason: "expected word<TAB>count",
        })?;
        let count: u32 = count.trim().parse().map_err(|_| LmError::Format {
            file: "lm-unigram.tsv",
            line: index + 1,
            reason: "count is not a number",
        })?;
        let text_len = u16::try_from(word.len()).map_err(|_| LmError::Format {
            file: "lm-unigram.tsv",
            line: index + 1,
            reason: "word too long",
        })?;
        if !seen.insert(word) {
            continue;
        }
        entries.push(WordEntry {
            text_start: words.len() as u32,
            count,
            text_len,
            reserved: 0,
        });
        words.push_str(word);
    }
    Ok((words, entries))
}

/// 三元表 → 三元 CSR（段号是二元在 `successors` 里的下标）。三元表为空时返回两个空表。
fn parse_trigram(
    trigram: &str,
    offsets: &[u32],
    successors: &[Successor],
    lookup: impl Fn(&str) -> Option<u32>,
) -> Result<(Vec<u32>, Vec<Successor>), LmError> {
    if trigram.trim().is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    // (u,v) → 它在 successors 里的下标：二元 CSR 段内按后词升序，二分即可
    let slot = |earlier: u32, previous: u32| -> Option<usize> {
        let start = *offsets.get(earlier as usize)? as usize;
        let end = *offsets.get(earlier as usize + 1)? as usize;
        successors[start..end]
            .binary_search_by_key(&previous, |s| s.word)
            .ok()
            .map(|index| start + index)
    };
    let mut rows: Vec<(u32, u32, u32)> = Vec::new();
    let mut orphans = 0usize;
    for (line_number, raw) in trigram.lines().enumerate() {
        let Some((words, count)) = fields(raw, 3, "lm-trigram.tsv", line_number)? else {
            continue;
        };
        let (Some(earlier), Some(previous), Some(word)) =
            (lookup(words[0]), lookup(words[1]), lookup(words[2]))
        else {
            orphans += 1;
            continue;
        };
        // 二元表砍过，(u,v) 自己可能不在表里：没有段可挂，丢掉（打分时这条上下文本来也退到二元）
        let Some(slot) = slot(earlier, previous) else {
            orphans += 1;
            continue;
        };
        rows.push((slot as u32, word, count));
    }
    if rows.is_empty() {
        // 一条都没挂上：别留一张全零的段偏移表，那是每条二元 4 字节
        return Ok((Vec::new(), Vec::new()));
    }
    rows.sort_unstable_by_key(|&(slot, word, _)| (slot, word));
    rows.dedup_by_key(|&mut (slot, word, _)| (slot, word));
    if orphans > 0 {
        tracing::debug!(orphans, kept = rows.len(), "三元里挂不上二元的已丢掉");
    }
    Ok(group(&rows, successors.len()))
}

/// 一行 TSV：前 `keys` 列是词、最后一列是计数。空行与 `#` 开头的返回 `None`。
#[allow(clippy::type_complexity)]
fn fields<'a>(
    raw: &'a str,
    keys: usize,
    file: &'static str,
    line_number: usize,
) -> Result<Option<(Vec<&'a str>, u32)>, LmError> {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < keys + 1 {
        return Err(LmError::Format {
            file,
            line: line_number + 1,
            reason: "expected words<TAB>…<TAB>count",
        });
    }
    let count: u32 = parts[keys].trim().parse().map_err(|_| LmError::Format {
        file,
        line: line_number + 1,
        reason: "count is not a number",
    })?;
    Ok(Some((parts[..keys].to_vec(), count)))
}

/// 把按 (段号, 后词) 排好的三元组分成每段一截。
pub(crate) fn group(rows: &[(u32, u32, u32)], groups: usize) -> (Vec<u32>, Vec<Successor>) {
    let mut offsets = vec![0u32; groups + 1];
    let mut successors = Vec::with_capacity(rows.len());
    for &(group, word, count) in rows {
        offsets[group as usize + 1] += 1;
        successors.push(Successor { word, count });
    }
    for index in 1..offsets.len() {
        offsets[index] += offsets[index - 1];
    }
    (offsets, successors)
}
