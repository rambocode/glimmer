//! 五笔的候选：全码命中在前、前缀命中作逐键提示，`z` 开头走拼音反查。
//! 按键后要不要自动上屏在 `auto_commit`，连着打编码出整句在 `sentence`。

mod auto_commit;
mod sentence;

use std::cmp::Reverse;
use std::time::{Duration, Instant};

use glimmer_dictionary::{Match, SyllablePattern};

use super::{Query, join_marked};
use crate::candidate::{Candidate, CandidateKind, CandidateList};
use crate::engine::{Engine, Learner, MAX_CANDIDATES, Timings, is_raw};
use crate::parser::ParseError;
use crate::ranking;
use crate::sentence::fallback_log_prob;
use crate::wubi::{REVERSE_LOOKUP_KEY, is_reverse_lookup};

/// 一条待排序的码表命中。
struct WubiHit<'a> {
    /// 码表命中。
    hit: Match<'a>,

    /// 编码与作用域整个相同（全码命中），否则是前缀命中（逐键提示）。
    full: bool,

    /// 同一编码下用户选过这个词几次（简码固定序时为 0）。
    choice: u32,

    /// 词频归一化的 log 概率加用户选择次数的加分。
    score: f64,

    /// 在码表返回顺序里的序号（查询时 `enumerate` 记下），平手时按它定序。
    index: usize,
}

impl WubiHit<'_> {
    /// 排序键，越小越靠前：全码在前；前缀命中短码在前（fcitx `SortByCodeLength`）；再按同编码下的选择次数、得分、码表顺序。
    ///
    /// 最后一项用码表顺序而不是文本序：98 码表的词频是上游逐行减 1 的名次（`王 1098435`、`五一 1098434`），
    /// 归一化后的 log 概率差在千分位以下，`(score * 1000).round()` 取整后完全打平，
    /// 按文本序排会把「五一」顶到「王」前面。词库 `assemble` 已经把同键条目按词频降序、平手保 TSV 原序排好，
    /// 所以码表顺序就是想要的词频顺序。
    fn key(&self) -> (Reverse<bool>, usize, Reverse<u32>, Reverse<i64>, usize) {
        (
            Reverse(self.full),
            self.hit.pinyin.len(),
            Reverse(self.choice),
            Reverse((self.score * 1000.0).round() as i64),
            self.index,
        )
    }
}

impl Engine {
    /// 五笔查询：直输段原样出；`z` 开头按拼音反查；否则查码表（含用户词），全码命中在前、前缀命中作提示，emoji 按文本照配。
    /// 不做纠错、模糊音、英文混输；整句只在 `[wubi] sentence` 开着时做（[`Self::insert_wubi_sentence`]）。
    pub(super) fn query_wubi(
        &self,
        keys: &str,
        rest: String,
        start: Instant,
    ) -> Result<Query, ParseError> {
        if is_raw(keys, self.modes(), self.shuangpin, self.zhuyin, true) {
            return Ok(self.query_raw(keys, rest, start));
        }
        if keys.starts_with(REVERSE_LOOKUP_KEY) {
            return self.query_reverse_lookup(keys, rest, start);
        }
        let parse = start.elapsed();
        let start = Instant::now();
        let mut items = self.wubi_candidates(keys);
        let lookup = start.elapsed();
        let start = Instant::now();
        // 整句开着：超过四码（或空码）时首选换成整句，拼音行按整句的切法用 `'` 分开
        let typed_display = self.insert_wubi_sentence(keys, &mut items);
        self.insert_emoji(&mut items);
        Ok(Query {
            segmentations: Vec::new(),
            candidates: CandidateList { items },
            tail: keys.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: false,
            typed_display,
            correction: None,
            timings: Timings {
                parse,
                lookup,
                rank: start.elapsed(),
            },
        })
    }

    /// 拼音反查：`z` 后面的部分当全拼走拼音路径，每个候选右侧注它的五笔码（算不出就不注），上屏吃掉整段作用域。
    /// 只有一个 `z` 时没有候选。
    fn query_reverse_lookup(
        &self,
        keys: &str,
        rest: String,
        start: Instant,
    ) -> Result<Query, ParseError> {
        let Some(scheme) = &self.wubi else {
            return self.query_pinyin(keys, rest, start);
        };
        if !is_reverse_lookup(keys) {
            return Ok(Query {
                tail: keys.to_owned(),
                text: self.composition.text().to_owned(),
                cursor: self.composition.cursor(),
                rest,
                timings: Timings {
                    parse: start.elapsed(),
                    lookup: Duration::ZERO,
                    rank: Duration::ZERO,
                },
                ..Query::default()
            });
        }
        let body = &keys[REVERSE_LOOKUP_KEY.len_utf8()..];
        let mut query = self.query_pinyin(body, rest, start)?;
        for candidate in &mut query.candidates.items {
            match candidate.kind {
                CandidateKind::Chinese | CandidateKind::Sentence | CandidateKind::Cloud => {
                    candidate.reading = scheme.code_of(&candidate.text);
                    candidate.syllables = vec![keys.to_owned()];
                }
                // emoji 按它对应的词消耗拼音，反查里就是整段
                CandidateKind::Emoji => candidate.syllables = vec![keys.to_owned()],
                CandidateKind::English | CandidateKind::Shortcut | CandidateKind::Custom(_) => {}
            }
        }
        // 显示上把 `z` 与切好的拼音连起来：`z'zhong'guo`，光标按字母数对位
        let marked = join_marked(&query.segmentations, &query.tail);
        query.typed_display = Some(format!("{REVERSE_LOOKUP_KEY}'{marked}"));
        Ok(query)
    }

    /// 码表（含用户词）里以 `keys` 为前缀的全部编码，排好序、按文本去重、最多 [`MAX_CANDIDATES`] 条：
    /// 全码命中在前，按词频 + 用户权重（同编码下选过的优先）；作用域不超过 `fixed_order_length` 码时只按码表静态词频
    /// （简码位置固定）；前缀命中短码在前，`hint` 开着时右侧注完整编码。
    pub(in crate::engine) fn wubi_candidates(&self, keys: &str) -> Vec<Candidate> {
        let Some(scheme) = &self.wubi else {
            return Vec::new();
        };
        // 空前缀会匹配整张码表
        if keys.is_empty() {
            return Vec::new();
        }
        let options = scheme.options();
        let fixed = keys.len() <= options.fixed_order_length;
        let dictionaries = self.wubi_dictionaries();
        let total: u64 = dictionaries.iter().map(|d| d.total_frequency()).sum();
        let log_total = (total as f64).max(1.0).ln();
        let pattern = [SyllablePattern::prefix(keys)];
        let position = self.choice_position();
        let mut hits: Vec<WubiHit<'_>> = dictionaries
            .iter()
            .flat_map(|d| d.lookup_pattern(&pattern))
            .enumerate()
            .map(|(index, hit)| {
                let full = hit.pinyin == keys;
                let choice = if full && !fixed {
                    self.learner.choice_weight(keys, hit.text, position)
                } else {
                    0
                };
                let weight = if fixed && full {
                    0
                } else {
                    self.learner.weight(hit.text)
                };
                WubiHit {
                    hit,
                    full,
                    choice,
                    score: fallback_log_prob(hit.frequency, log_total)
                        + ranking::weight_bonus(weight),
                    index,
                }
            })
            .collect();
        // 一键前缀能命中上万条：先按键选出够排的量再排，与拼音路径的预选同理（多留一倍给去重）
        let preselect = MAX_CANDIDATES.saturating_mul(2);
        if hits.len() > preselect.saturating_mul(2) {
            hits.select_nth_unstable_by(preselect, |a, b| a.key().cmp(&b.key()));
            hits.truncate(preselect);
        }
        hits.sort_unstable_by_key(|h| h.key());
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        hits.into_iter()
            .filter(|h| seen.insert(h.hit.text))
            .take(MAX_CANDIDATES)
            .map(|h| Candidate {
                text: h.hit.text.to_owned(),
                kind: CandidateKind::Chinese,
                syllables: vec![h.hit.pinyin.to_owned()],
                // 全码是自己敲的，不用看；提示候选注完整编码
                reading: (!h.full && options.hint).then(|| h.hit.pinyin.to_owned()),
                translation: None,
            })
            .collect()
    }
}
