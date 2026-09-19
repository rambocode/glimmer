//! 五笔的词图：位置是编码的字母，边是编码正好等于那几个字母的词。

use std::sync::Arc;

use glimmer_dictionary::{Dictionary, SyllablePattern};

use super::Lattice;
use crate::sentence::{Personal, SPAN_CANDIDATES, SpanCache, SpanWord};
use crate::wubi::{MAX_CODE_LEN, is_code_key};

/// 末尾那条边按前缀命中（编码还没打完）时扣多少分：敲的字母本身就是一条简码时让简码赢，
/// 没有简码（或简码的词接不上前文）时才拿「打完会是什么」来顶，首选不至于每一键都跳成单字。
/// 2026-09-19 在 9807 句全码评测上扫过 1 / 3 / 6 / 30：首选 86 版 79.5% / 79.6% / 79.7% / 79.7%、98 版 75.9% / 76.3% / 76.3% / 76.3%，
/// 扣得少了前缀词会抢走打全了的简码；6 起就平了，取 6（再大前缀词在打到一半时几乎出不来）。
pub const TAIL_PREFIX_PENALTY: f64 = 6.0;

/// 缓存键的前缀：混输下拼音与五笔共用一张 [`SpanCache`]，编码 `a` 与音节 `a` 会撞键。
const CACHE_PREFIX: char = '\u{1}';

/// 五笔的词图。连着打的一串编码（`wqvbkhlg`）没有切分，每 1 到 [`MAX_CODE_LEN`] 个字母都可能是一条编码，
/// 全部放进词图让语言模型挑；每个字母都有一级简码，所以路径总走得通。
pub struct CodeLattice<'a, W> {
    /// 码表与用户词。
    dictionaries: &'a [&'a Dictionary],

    /// 连着打的编码，全是编码键（ASCII 小写字母），所以按字节切就是按键切。
    keys: &'a str,

    /// 个人 n-gram：只用它的出现次数让用户常用的重码词进得了格子。
    personal: Personal<'a>,

    /// 用户选择次数。
    weight: &'a W,

    /// 格子候选的缓存。
    cache: &'a mut SpanCache,

    /// 词库总词频的对数。
    log_total: f64,
}

impl<'a, W> CodeLattice<'a, W>
where
    W: Fn(&str) -> u32,
{
    /// 建词图；`keys` 里有不是编码键的字符时为 `None`（按字节切会切坏）。
    pub fn new(
        dictionaries: &'a [&'a Dictionary],
        keys: &'a str,
        personal: Personal<'a>,
        weight: &'a W,
        cache: &'a mut SpanCache,
    ) -> Option<Self> {
        if !keys.chars().all(is_code_key) {
            return None;
        }
        let total: f64 = dictionaries
            .iter()
            .map(|d| d.total_frequency() as f64)
            .sum::<f64>()
            .max(1.0);
        Some(Self {
            dictionaries,
            keys,
            personal,
            weight,
            cache,
            log_total: total.ln(),
        })
    }

    /// 一个格子里的候选词：编码等于 `code` 的词按词频（加用户选择次数与个人出现次数）取前几个。
    /// `tail` 为真（句末、不满四码）时再收以 `code` 为前缀的词，按 [`TAIL_PREFIX_PENALTY`] 打折，另留同样多个位置。
    fn span_candidates(
        dictionaries: &[&Dictionary],
        code: &str,
        tail: bool,
        personal: Personal<'_>,
        weight: &W,
    ) -> Vec<SpanWord> {
        let pattern = [if tail {
            SyllablePattern::prefix(code)
        } else {
            SyllablePattern::complete(code)
        }];
        let mut scored: Vec<(f64, SpanWord)> = dictionaries
            .iter()
            .flat_map(|d| d.lookup_pattern(&pattern))
            // 前缀模式也会带出更长的多音节条目；码表一条编码就是一个音节，用户词也一样，多音节的不是五笔词
            .filter(|m| m.exact)
            .map(|m| {
                let penalty = if m.pinyin == code {
                    0.0
                } else {
                    TAIL_PREFIX_PENALTY
                };
                let seen = weight(m.text) + personal.count(m.text);
                let score = f64::from(m.frequency) * (1.0 + f64::from(seen)) * (-penalty).exp();
                let word = SpanWord {
                    text: m.text.to_owned(),
                    // 记敲的那段字母，不记词的完整编码：前缀命中的编码比敲的长，路径上各词的宽度要加起来正好是整串
                    syllables: vec![code.to_owned()],
                    frequency: m.frequency,
                    penalty,
                };
                (score, word)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        // 同一个词既有简码又有全码（前缀命中）时留靠前的那条
        let mut seen: Vec<&str> = Vec::new();
        let mut exact = 0;
        let mut prefixed = 0;
        let mut kept = Vec::new();
        for (index, (_, word)) in scored.iter().enumerate() {
            if seen.contains(&word.text.as_str()) {
                continue;
            }
            let count = if word.penalty == 0.0 {
                &mut exact
            } else {
                &mut prefixed
            };
            if *count >= SPAN_CANDIDATES {
                continue;
            }
            *count += 1;
            seen.push(&word.text);
            kept.push(index);
        }
        kept.into_iter().map(|i| scored[i].1.clone()).collect()
    }
}

impl<W> Lattice for CodeLattice<'_, W>
where
    W: Fn(&str) -> u32,
{
    fn positions(&self) -> usize {
        self.keys.len()
    }

    fn max_span(&self) -> usize {
        MAX_CODE_LEN
    }

    fn log_total(&self) -> f64 {
        self.log_total
    }

    fn words(&mut self, start: usize, end: usize) -> Arc<[SpanWord]> {
        let code = &self.keys[start..end];
        let tail = end == self.keys.len() && code.len() < MAX_CODE_LEN;
        let mut key = String::with_capacity(code.len() + 4);
        key.push(CACHE_PREFIX);
        key.push_str(code);
        if tail {
            key.push('…');
        }
        let (dictionaries, personal, weight) = (self.dictionaries, self.personal, self.weight);
        self.cache.get_or_insert_with(key, || {
            Self::span_candidates(dictionaries, code, tail, personal, weight)
        })
    }

    /// 五笔没有猜敲错的边，不多扣。
    fn extra_penalty(&self, _start: usize, _end: usize, _word: &SpanWord) -> f64 {
        0.0
    }

    fn placeholder(&self, start: usize) -> &str {
        &self.keys[start..start + 1]
    }
}
