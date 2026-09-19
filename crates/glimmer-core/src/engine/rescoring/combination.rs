//! 重排第二轮：把几条路径各自赢的局部替换拼成一条新路径。
//!
//! 前 k 条路径大多只比最优路径改了一处。长句里常常有两三处各自独立的同音错，「两处都改对」的那条路径
//! 在词级模型下要同时吃两份代价，排不进前 k 条；但它的零件都在：一条路径改对了前半句，另一条改对了后半句。
//! 第一轮重排之后，相对最优路径净赚、彼此不重叠的替换拼在一起，再让模型打一次分，赢了才用。

use crate::sentence::{Conversion, SentenceWord};

/// 一条路径相对基准路径改动的那一段：音节区间与换上去的词。
struct Replacement<'a> {
    /// 音节区间 `[start, end)`。
    start: usize,

    end: usize,

    /// 换上去的词。
    words: &'a [SentenceWord],

    /// 这条路径来自 `candidates` 的第几条。
    source: usize,
}

/// `base` 是词级模型的最优路径，`candidates` 是别的路径与它们重排后比 `base` 多赚的分。
/// 净赚（> 0）且互不重叠的替换按赚得多的先挑；挑出两处以上才拼，拼不出新东西返回 `None`。
///
/// 拼出来的路径得分按可加估：`base` 的分加上各处替换各自带来的差。替换不相邻时词级模型的分确实可加；
/// 相邻时差一个接缝上的二元分，由随后的神经分兜着。
pub(super) fn combine(base: &Conversion, candidates: &[(&Conversion, f64)]) -> Option<Conversion> {
    let mut replacements: Vec<(f64, Replacement<'_>)> = candidates
        .iter()
        .enumerate()
        .filter(|(_, (_, gain))| *gain > 0.0)
        .filter_map(|(source, (path, gain))| Some((*gain, replacement(base, path, source)?)))
        .collect();
    replacements.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut chosen: Vec<Replacement<'_>> = Vec::new();
    for (_, candidate) in replacements {
        let overlaps = chosen
            .iter()
            .any(|taken| candidate.start < taken.end && taken.start < candidate.end);
        if !overlaps {
            chosen.push(candidate);
        }
    }
    if chosen.len() < 2 {
        return None;
    }
    chosen.sort_by_key(|r| r.start);
    let mut words: Vec<SentenceWord> = Vec::new();
    let mut position = 0;
    let mut pending = chosen.iter().peekable();
    for word in &base.words {
        let end = position + word.syllables.len();
        match pending.peek() {
            Some(next) if position == next.start => {
                words.extend(next.words.iter().cloned());
            }
            _ => {}
        }
        let replaced = chosen.iter().any(|r| position >= r.start && end <= r.end);
        if !replaced {
            words.push(word.clone());
        }
        if pending.peek().is_some_and(|next| end >= next.end) {
            pending.next();
        }
        position = end;
    }
    let mut combined = Conversion {
        text: words.iter().map(|w| w.text.as_str()).collect(),
        syllables: words
            .iter()
            .flat_map(|w| w.syllables.iter().cloned())
            .collect(),
        words,
        score: base.score,
        static_score: base.static_score,
        penalty: base.penalty,
    };
    for replacement in &chosen {
        let path = candidates[replacement.source].0;
        combined.score += path.score - base.score;
        combined.static_score += path.static_score - base.static_score;
        combined.penalty += path.penalty - base.penalty;
    }
    Some(combined)
}

/// `path` 相对 `base` 改动的那一段：去掉两头相同的词，剩下的音节区间与 `path` 在这段里的词。
/// 两条路径一样（没有改动）返回 `None`。
fn replacement<'a>(
    base: &Conversion,
    path: &'a Conversion,
    source: usize,
) -> Option<Replacement<'a>> {
    let same = |a: &SentenceWord, b: &SentenceWord| {
        a.text == b.text && a.syllables.len() == b.syllables.len()
    };
    let prefix = base
        .words
        .iter()
        .zip(&path.words)
        .take_while(|(a, b)| same(a, b))
        .count();
    if prefix == base.words.len() && prefix == path.words.len() {
        return None;
    }
    let suffix = base.words[prefix..]
        .iter()
        .rev()
        .zip(path.words[prefix..].iter().rev())
        .take_while(|(a, b)| same(a, b))
        .count();
    let start: usize = base.words[..prefix].iter().map(|w| w.syllables.len()).sum();
    let middle = &path.words[prefix..path.words.len() - suffix];
    let end = start + middle.iter().map(|w| w.syllables.len()).sum::<usize>();
    Some(Replacement {
        start,
        end,
        words: middle,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(words: &[(&str, usize)], score: f64) -> Conversion {
        let words: Vec<SentenceWord> = words
            .iter()
            .map(|(text, syllables)| SentenceWord {
                text: (*text).to_owned(),
                syllables: vec!["x".to_owned(); *syllables],
                placeholder: false,
            })
            .collect();
        Conversion {
            text: words.iter().map(|w| w.text.as_str()).collect(),
            syllables: words.iter().flat_map(|w| w.syllables.clone()).collect(),
            words,
            score,
            static_score: score,
            penalty: 0.0,
        }
    }

    #[test]
    fn joins_winning_replacements_that_do_not_overlap() {
        let base = path(&[("次", 1), ("哭", 1), ("的", 1), ("声称", 2)], -20.0);
        let front = path(&[("词库", 2), ("的", 1), ("声称", 2)], -22.0);
        let back = path(&[("次", 1), ("哭", 1), ("的", 1), ("生成", 2)], -21.0);
        let loser = path(&[("次", 1), ("哭", 1), ("得", 1), ("声称", 2)], -23.0);
        let combined = combine(&base, &[(&front, 3.0), (&back, 2.0), (&loser, -1.0)]).unwrap();
        assert_eq!(combined.text, "词库的生成");
        assert_eq!(combined.words.len(), 3);
        assert_eq!(combined.syllables.len(), 5);
        // 可加估分：−20 + (−22 + 20) + (−21 + 20)
        assert_eq!(combined.score, -23.0);
        // 只有一处赢、或两处重叠：不拼
        assert!(combine(&base, &[(&front, 3.0), (&loser, -1.0)]).is_none());
        let overlapping = path(&[("此", 1), ("库", 1), ("的", 1), ("声称", 2)], -22.5);
        assert!(combine(&base, &[(&front, 3.0), (&overlapping, 1.0)]).is_none());
    }
}
