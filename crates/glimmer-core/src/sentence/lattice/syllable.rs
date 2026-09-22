//! 拼音的词图：位置是音节，边是正好覆盖那几个音节的词。

use std::sync::Arc;

use glimmer_dictionary::{Dictionary, Match, SyllablePattern};

use super::Lattice;
use crate::sentence::{
    ABBREVIATED_SPAN_CANDIDATES, MAX_WORD_SYLLABLES, PROTECTED_MIN_COST, Personal, ReadingShare,
    Search, SpanCache, SpanWord,
};

/// 拼音的词图。`positions` 每个位置是若干写法（第一种是敲的，其余是模糊音 / 敲错变体），
/// `cost(位置, 命中的音节)` 是那个位置命中这种写法要扣的分（敲的原样 0），`weight` 是用户选择次数。
///
/// 格子查词是最贵的一步，结果放进 `cache`（调用方保证它与词库、`weight`、`personal`、`cost` 一致，这些一变就清）。
pub struct SyllableLattice<'a, W, C> {
    /// 主词库与用户词。
    dictionaries: &'a [&'a Dictionary],

    /// 每个位置的各种写法。
    positions: &'a [Vec<SyllablePattern<'a>>],

    /// 个人 n-gram：只用它的出现次数让用户常用的同音词进得了格子。
    personal: Personal<'a>,

    /// 用户选择次数。
    weight: &'a W,

    /// 替代写法的代价。
    cost: &'a C,

    /// 格子候选的缓存。
    cache: &'a mut SpanCache,

    /// 多音字的读音份额表；`None` 为不扣读音份额。
    reading: Option<&'a ReadingShare>,

    /// 原样成词保护多扣的分（见 [`Self::typed_word_reach`]）；0 为不保护。
    protected_extra: f64,

    /// 每个位置往右最远被「按敲的原样读出的多音节词」盖到哪。
    typed_reach: Vec<usize>,

    /// 词库总词频的对数。
    log_total: f64,
}

impl<'a, W, C> SyllableLattice<'a, W, C>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    /// 建词图：总词频与原样成词的范围当场算好（后者查过的格子进缓存，主循环直接用）。
    /// `search` 只取词图这一层用得上的两项（读音份额表、原样成词保护），其余是 Viterbi 的事。
    pub fn new(
        dictionaries: &'a [&'a Dictionary],
        positions: &'a [Vec<SyllablePattern<'a>>],
        personal: Personal<'a>,
        weight: &'a W,
        cost: &'a C,
        cache: &'a mut SpanCache,
        search: Search<'a>,
    ) -> Self {
        let total: f64 = dictionaries
            .iter()
            .map(|d| d.total_frequency() as f64)
            .sum::<f64>()
            .max(1.0);
        let mut lattice = Self {
            dictionaries,
            positions,
            personal,
            weight,
            cost,
            cache,
            reading: search.reading,
            protected_extra: search.protected_extra,
            typed_reach: Vec::new(),
            log_total: total.ln(),
        };
        lattice.typed_reach = lattice.typed_word_reach();
        lattice
    }

    /// 每个位置往右最远被「按敲的原样读出的多音节词」盖到哪：`reach[i] >= end` 说明 `[i, end)` 整个落在某个原样词里面。
    ///
    /// `anpaiceshirenyuan` 里 `ce shi` 原样就是 测试，这时在 `ce` 上猜敲错（`de` → 的）多半是误伤：
    /// 高频虚词的语言模型分盖得过一条敲错边的代价，句子越长这种机会越多。落在原样词里面的敲错边多扣 `protected_extra`。
    /// 只管「里面」：`mei gan xi` 的 没关系 比原样词 美感 长、伸到了外面，原样词解释不了整段，不拦。
    /// 没有替代写法或 `protected_extra` 为 0 时不扫。
    fn typed_word_reach(&mut self) -> Vec<usize> {
        let n = self.positions.len();
        let mut reach = vec![0; n];
        if self.protected_extra <= 0.0 || self.positions.iter().all(|p| p.len() < 2) {
            return reach;
        }
        for (start, furthest) in reach.iter_mut().enumerate() {
            for end in start + 2..=n.min(start + MAX_WORD_SYLLABLES) {
                let span = &self.positions[start..end];
                let hits = self.words(start, end);
                // 只认每个音节都与敲的完整音节一字不差的词：简拼 / 前缀位置上的命中不算「原样成词」
                let typed = hits.iter().any(|hit| {
                    hit.penalty == 0.0
                        && hit.syllables.len() == span.len()
                        && hit.syllables.iter().zip(span).all(|(syllable, forms)| {
                            forms
                                .first()
                                .is_some_and(|t| t.complete && t.text == syllable)
                        })
                });
                if typed {
                    *furthest = end;
                }
            }
        }
        // 起点更靠左的原样词也盖得住右边的位置
        for index in 1..n {
            reach[index] = reach[index].max(reach[index - 1]);
        }
        reach
    }

    /// 一个格子里的候选词：所有词库的精确命中，按词频（加用户选择次数与个人出现次数，替代写法命中的按代价打折）取前几个。
    /// 个人次数只在这里保证用户常用的同音词进得了格子，不进路径打分（那是 n-gram 的事）；
    /// 打折让敲错变体命中的词只在原样命中不够多时才进格子，而常用词（关系）即使打折也留得住。
    /// 格子里有简拼位置时命中的是一大片不同读音的词，多留一些让语言模型去挑。
    ///
    /// 挑候选**不**看读音份额：这里用的 `m.frequency` 是词库按读音分开记的那一条（没(mo) 是 7 千不是 92 万），
    /// 本来就只算这个读音，再扣一次就把冷门读音从格子里删掉了。份额是给语言模型那一步用的，
    /// 算好存进 [`SpanWord::reading`]，路径打分时才减。
    fn span_candidates(
        dictionaries: &[&Dictionary],
        span: &[Vec<SyllablePattern<'_>>],
        start: usize,
        personal: Personal<'_>,
        weight: &W,
        cost: &C,
        reading: Option<&ReadingShare>,
    ) -> Vec<SpanWord> {
        let alternatives = span.iter().any(|p| p.len() > 1);
        let penalty_of = |m: &Match<'_>| {
            if !alternatives {
                return 0.0;
            }
            m.syllables()
                .enumerate()
                .map(|(index, syllable)| cost(start + index, syllable))
                .sum::<f64>()
        };
        // 得分先算好再排：单字母简拼的格子能命中几千条，比较器里每次查两张表会让排序占掉十几毫秒
        let mut scored: Vec<(f64, f64, Match<'_>)> = dictionaries
            .iter()
            .flat_map(|d| d.lookup_exact_alt(span))
            .map(|m| {
                let seen = weight(m.text) + personal.count(m.text);
                let penalty = penalty_of(&m);
                let score = f64::from(m.frequency) * (1.0 + f64::from(seen)) * (-penalty).exp();
                (score, penalty, m)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.dedup_by(|a, b| a.2.text == b.2.text);
        let abbreviated = span.iter().any(|p| p.iter().any(|t| !t.complete));
        scored.truncate(if abbreviated {
            ABBREVIATED_SPAN_CANDIDATES
        } else {
            personal.interpolation.span_candidates
        });
        let interpolation = personal.interpolation;
        scored
            .into_iter()
            .map(|(_, penalty, hit)| SpanWord {
                text: hit.text.to_owned(),
                syllables: hit.syllables().map(str::to_owned).collect(),
                frequency: hit.frequency,
                penalty,
                reading: reading.map_or(0.0, |share| {
                    share.penalty(
                        hit.text,
                        hit.frequency,
                        interpolation.reading_weight,
                        interpolation.reading_cap,
                    )
                }),
            })
            .collect()
    }
}

impl<W, C> Lattice for SyllableLattice<'_, W, C>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    fn positions(&self) -> usize {
        self.positions.len()
    }

    fn max_span(&self) -> usize {
        MAX_WORD_SYLLABLES
    }

    fn log_total(&self) -> f64 {
        self.log_total
    }

    fn words(&mut self, start: usize, end: usize) -> Arc<[SpanWord]> {
        let span = &self.positions[start..end];
        let (dictionaries, personal, weight, cost, reading) = (
            self.dictionaries,
            self.personal,
            self.weight,
            self.cost,
            self.reading,
        );
        self.cache.get_or_insert_with(SpanCache::key(span), || {
            Self::span_candidates(dictionaries, span, start, personal, weight, cost, reading)
        })
    }

    /// 原样成词保护：这条猜敲错的边整个落在一个原样读出的多音节词里面，多扣一份。
    fn extra_penalty(&self, start: usize, end: usize, word: &SpanWord) -> f64 {
        let guarded = word.penalty > 0.0
            && self.typed_reach[start] >= end
            && word.syllables.iter().enumerate().any(|(offset, syllable)| {
                (self.cost)(start + offset, syllable) >= PROTECTED_MIN_COST
            });
        if guarded { self.protected_extra } else { 0.0 }
    }

    fn placeholder(&self, start: usize) -> &str {
        self.positions[start][0].text
    }
}
