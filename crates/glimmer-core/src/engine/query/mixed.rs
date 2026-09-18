//! 混输（五笔 + 拼音）：两条管线各出一批候选，编码打全的五笔词在最前、拼音居中、只命中编码前缀的五笔词垫后。

use std::collections::HashSet;
use std::time::Instant;

use super::Query;
use crate::candidate::{Candidate, CandidateList};
use crate::engine::{Engine, MAX_CANDIDATES, Timings, is_raw};
use crate::parser::ParseError;

impl Engine {
    /// 混输的候选：五笔与拼音都查，**编码打全的五笔词在最前，其次拼音，只命中编码前缀的五笔词垫后**。
    ///
    /// 打全的编码是精确的（`ga` 就是 开）；只敲了前缀的五笔词一律排前面的话，`kai` 的首选会变成
    /// 编码 `kaik` 的 中共党员，拼音就没法用了。拼音那条路给不出解析时（`ggll` 切不成音节）不算失败，
    /// 整个查询按五笔的结果走；五笔码最长四位，第 5 个字母起码表查不到东西，自然只剩拼音。
    ///
    /// 混输下 `z` 不再是拼音反查的前缀（`z` 是声母，`zhongguo` 得能打），反查只在只用形码时有。
    pub(super) fn query_mixed(
        &self,
        keys: &str,
        rest: String,
        start: Instant,
    ) -> Result<Query, ParseError> {
        // 直输段按拼音侧的规则判（`no-way` 的 `-` 两边都不是键）：混输下字母既可能是编码也可能是拼音
        if is_raw(keys, self.modes(), self.shuangpin, self.zhuyin, false) {
            return Ok(self.query_raw(keys, rest, start));
        }
        let lookup_started = Instant::now();
        let mut full = self.wubi_candidates(keys);
        // `wubi_candidates` 已把全码命中排在前缀命中之前，从第一条「编码比敲的长」处切开就是两段
        let split_at = full
            .iter()
            .take_while(|c| c.syllables.first().is_some_and(|code| code == keys))
            .count();
        let hints = full.split_off(split_at);
        let lookup = lookup_started.elapsed();

        let mut query = match self.query_pinyin(keys, rest.clone(), start) {
            Ok(query) => query,
            Err(error) => {
                // 拼音读不出来，但五笔有命中：整段按五笔走；两边都空才算查询失败
                if full.is_empty() && hints.is_empty() {
                    return Err(error);
                }
                return Ok(self.code_only_query(keys, rest, full, hints, start, lookup));
            }
        };
        // 同一个词两边都命中时按文本去重，靠前的那条留着
        let mut seen: HashSet<String> = HashSet::new();
        let items: Vec<Candidate> = full
            .into_iter()
            .chain(std::mem::take(&mut query.candidates.items))
            .chain(hints)
            .filter(|candidate| seen.insert(candidate.text.clone()))
            .take(MAX_CANDIDATES)
            .collect();
        query.candidates.items = items;
        query.timings.lookup += lookup;
        Ok(query)
    }

    /// 拼音那条路失败时用的结果：只有五笔候选，显示串是敲的编码本身（没有切分）。
    fn code_only_query(
        &self,
        keys: &str,
        rest: String,
        full: Vec<Candidate>,
        hints: Vec<Candidate>,
        start: Instant,
        lookup: std::time::Duration,
    ) -> Query {
        let mut items = full;
        items.extend(hints);
        items.truncate(MAX_CANDIDATES);
        self.insert_emoji(&mut items);
        Query {
            segmentations: Vec::new(),
            candidates: CandidateList { items },
            tail: keys.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: false,
            typed_display: None,
            correction: None,
            timings: Timings {
                parse: start.elapsed().saturating_sub(lookup),
                lookup,
                rank: std::time::Duration::ZERO,
            },
        }
    }
}
