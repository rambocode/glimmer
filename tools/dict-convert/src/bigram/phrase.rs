//! 短语层的合成计数：短语不参与分词，统计完按成分的一元 / 二元估它自己的计数（算法见 `super` 的模块注释）。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::ConvertError;

use super::Vocabulary;

/// 把 `path` 里的短语从分词词表摘掉（编号留着给合成的 token 用），返回每条短语的 (编号, 成分词编号)。
/// 切不成两个以上词库词的短语跳过。
pub(super) fn phrase_components(
    path: &Path,
    vocabulary: &mut Vocabulary,
) -> Result<Vec<(u32, Vec<u32>)>, ConvertError> {
    let texts: Vec<String> = std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next().map(str::to_owned))
        .collect();
    let ids: Vec<Option<u32>> = texts
        .iter()
        .map(|t| vocabulary.ids.get(t).copied())
        .collect();
    for text in &texts {
        vocabulary.ids.remove(text);
    }
    let mut parts = Vec::new();
    let mut tokens = Vec::new();
    let mut skipped = 0usize;
    for (text, id) in texts.iter().zip(ids) {
        let Some(id) = id else {
            skipped += 1;
            continue;
        };
        vocabulary.segment(text, &mut tokens);
        let components: Option<Vec<u32>> = tokens.iter().copied().collect();
        match components {
            Some(components) if components.len() >= 2 => parts.push((id, components)),
            _ => skipped += 1,
        }
    }
    tracing::info!(path = %path.display(), phrases = parts.len(), skipped, "短语已从分词词表摘掉，统计完再合成计数");
    Ok(parts)
}

/// 基础词库（`dict`，不含 `dicts/` 领域词库）里语料一次都没切出来的多字词：一行 在语料里总被切成 一 / 行
/// （词库词频是底值，拼不过两个高频单字），一元表里就没有它，词库下一轮也只能给底值，永远翻不了身。
/// 这些词和短语层一样处理：从分词词表摘掉，按成分合成计数（`phrases` 里的短语已摘掉、计数本来就是 0，跳过）。
/// 领域词库的词不合成：它们按语料次数决定留在基础词库还是拆出去，合成的次数会把一堆领域词拉回基础词库。
pub(super) fn unseen_components(
    dict: &Path,
    vocabulary: &mut Vocabulary,
    unigram: &[u64],
    phrases: &[(u32, Vec<u32>)],
) -> Result<Vec<(u32, Vec<u32>)>, ConvertError> {
    let phrase_ids: HashSet<u32> = phrases.iter().map(|(id, _)| *id).collect();
    let mut unseen: Vec<(String, u32)> = Vec::new();
    let mut seen_texts: HashSet<String> = HashSet::new();
    for line in std::fs::read_to_string(dict)?.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(text) = line.split('\t').next() else {
            continue;
        };
        if text.chars().count() < 2 || !seen_texts.insert(text.to_owned()) {
            continue;
        }
        if let Some(&id) = vocabulary.ids.get(text)
            && unigram[id as usize] == 0
            && !phrase_ids.contains(&id)
        {
            unseen.push((text.to_owned(), id));
        }
    }
    // 先全部摘掉再切：不然切 一行 时词表里还有 一行 自己
    for (text, _) in &unseen {
        vocabulary.ids.remove(text);
    }
    let mut parts = Vec::new();
    let mut tokens = Vec::new();
    for (text, id) in &unseen {
        vocabulary.segment(text, &mut tokens);
        let components: Option<Vec<u32>> = tokens.iter().copied().collect();
        if let Some(components) = components
            && components.len() >= 2
        {
            parts.push((*id, components));
        }
    }
    tracing::info!(
        unseen = unseen.len(),
        synthesizable = parts.len(),
        "语料没切出来的词库词按成分合成计数"
    );
    Ok(parts)
}

/// 短语自身的合成次数：两词就是成分二元 c(a,b)，更长的按链式 c(a,b)·c(b,c)/c(b) 估（算法见模块注释）。
pub(crate) fn phrase_count(parts: &[u32], unigram: &[u64], bigram: &HashMap<u64, u32>) -> f64 {
    let pair = |a: u32, b: u32| {
        f64::from(
            bigram
                .get(&((u64::from(a) << 32) | u64::from(b)))
                .copied()
                .unwrap_or(0),
        )
    };
    let mut count = pair(parts[0], parts[1]);
    for window in parts.windows(2).skip(1) {
        let middle = unigram[window[0] as usize] as f64;
        if middle <= 0.0 {
            return 0.0;
        }
        count *= pair(window[0], window[1]) / middle;
    }
    count
}

/// 给短语合成一元（写进 `unigram`）与前后接的二元计数（返回，算法见模块注释）。
pub(crate) fn synthesize_phrases(
    phrases: &[(u32, Vec<u32>)],
    unigram: &mut [u64],
    bigram: &HashMap<u64, u32>,
    min_count: u32,
) -> Vec<(u64, u32)> {
    let key = |a: u32, b: u32| (u64::from(a) << 32) | u64::from(b);
    // 按后词 / 前词索引一遍二元表，合成时按成分查前接与后接
    let mut by_second: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    let mut by_first: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    for (&k, &count) in bigram.iter() {
        let (first, second) = ((k >> 32) as u32, (k & 0xFFFF_FFFF) as u32);
        by_second.entry(second).or_default().push((first, count));
        by_first.entry(first).or_default().push((second, count));
    }
    let mut added = 0usize;
    let mut rows: Vec<(u64, u32)> = Vec::new();
    for (id, parts) in phrases {
        let (first, last) = (parts[0], parts[parts.len() - 1]);
        let count = phrase_count(parts, unigram, bigram);
        if count < f64::from(min_count) {
            continue;
        }
        unigram[*id as usize] = count.round() as u64;
        added += 1;
        let first_total = unigram[first as usize].max(1) as f64;
        let last_total = unigram[last as usize].max(1) as f64;
        for (previous, c) in by_second.get(&first).into_iter().flatten() {
            let synthesized = f64::from(*c) * count / first_total;
            if synthesized >= f64::from(min_count) {
                rows.push((key(*previous, *id), synthesized.round() as u32));
            }
        }
        for (next, c) in by_first.get(&last).into_iter().flatten() {
            let synthesized = count * f64::from(*c) / last_total;
            if synthesized >= f64::from(min_count) {
                rows.push((key(*id, *next), synthesized.round() as u32));
            }
        }
    }
    tracing::info!(phrases = added, bigrams = rows.len(), "短语计数已合成");
    rows
}
