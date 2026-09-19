//! 词图上的最优路径：bigram Viterbi + 束搜索。
//!
//! 状态只按前一个词分（束宽内），个人三元要的前二词取前驱节点的回指（它那条最优路径上的前一个词）：
//! 不扩状态，代价是三元上下文是近似的，个人数据量下够用。

use glimmer_dictionary::{Dictionary, Match, SyllablePattern};

use super::{
    ABBREVIATED_SPAN_CANDIDATES, BEAM_WIDTH, Context, Conversion, LanguageModel,
    MAX_WORD_SYLLABLES, MIN_PARTIAL_LETTERS, PROTECTED_MIN_COST, Personal, SPAN_CANDIDATES, Search,
    SentenceWord, SpanCache, SpanWord, fallback_log_prob, transition_log_prob,
};
use crate::ranking::weight_bonus;

/// 词库里没有的孤立音节（罕见音节没有单字）按这个 log 概率兜底，让路径总能走通。
const UNKNOWN_LOG_PROB: f64 = -30.0;

/// 一条部分路径的末尾节点。
struct Node {
    /// 这个词从第几个音节开始。
    start: usize,

    /// 词。
    text: String,

    /// 词的音节。
    syllables: Vec<String>,

    /// 到此为止的累计得分。
    score: f64,

    /// 累计得分里静态模型的部分（见 `Conversion::static_score`）。
    static_score: f64,

    /// 前驱在 `nodes[start]` 里的下标；`start == 0` 时无意义。
    back: usize,

    /// 是占位音节。
    placeholder: bool,

    /// 到此为止路径上模糊音 / 敲错变体的代价之和（已从 `score` 里扣掉，另记一份给调用方判断路径是不是原样）。
    penalty: f64,
}

/// 把音节序列转成最可能的词序列。`positions` 每个位置是若干写法（第一种是敲的，其余是模糊音 / 敲错变体），
/// `cost(位置, 命中的音节)` 是那个位置命中这种写法要扣的分（敲的原样 0），
/// `weight` 是用户选择次数，`personal` 是个人 n-gram 与插值参数（没有个人数据就传 [`Personal::NONE`]），`cache` 是格子候选的缓存
/// （调用方保证它与词库、`weight`、`personal`、`cost` 一致，这些一变就清）。
///
/// 简拼位置（`w x q`）按前缀取词：每个格子的候选会多得多，由语言模型在路径上分辨。
/// 全拼句子末尾的前缀太短时不算它（多半是没打完的音节）；前面已有简拼的句子里末尾单字母就是一个音节。
pub fn convert(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Option<Conversion> {
    convert_paths(
        dictionaries,
        positions,
        Search::BEST,
        model,
        personal,
        weight,
        cost,
        cache,
    )
    .into_iter()
    .next()
}

/// 得分最高的前 `search.paths` 条路径（最多束宽条，按得分降序，文本相同的只留一条）：给重打分用。
///
/// `search.keep_partial` 为真时全拼句子末尾的单字母也当一个音节读（`huo z…` → 或者）：给「整段拼音读法」与别的读法比分用，
/// 比分要两边覆盖同样多的字母。`search.initial` 是这段拼音之前的上文，第一个词的转移按它算；
/// `search.protected_extra` 见 [`typed_word_reach`]。
#[allow(clippy::too_many_arguments)]
pub fn convert_paths(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    search: Search<'_>,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Vec<Conversion> {
    let Some((last, head)) = positions.split_last() else {
        return Vec::new();
    };
    let Some(&last) = last.first() else {
        return Vec::new();
    };
    let abbreviated_head = head.iter().any(|p| p.first().is_none_or(|t| !t.complete));
    let k = search.paths;
    let positions = if search.keep_partial
        || last.complete
        || abbreviated_head
        || last.text.len() >= MIN_PARTIAL_LETTERS
    {
        positions
    } else {
        head
    };
    let n = positions.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }
    let total: f64 = dictionaries
        .iter()
        .map(|d| d.total_frequency() as f64)
        .sum::<f64>()
        .max(1.0);
    let log_total = total.ln();

    let typed_reach = typed_word_reach(
        dictionaries,
        positions,
        search.protected_extra,
        personal,
        &weight,
        &cost,
        cache,
    );

    // nodes[i]：覆盖前 i 个音节、以某个词结尾的部分路径；nodes[0] 是虚拟起点
    let mut nodes: Vec<Vec<Node>> = (0..=n).map(|_| Vec::new()).collect();
    nodes[0].push(Node {
        start: 0,
        text: String::new(),
        syllables: Vec::new(),
        score: 0.0,
        static_score: 0.0,
        back: 0,
        placeholder: false,
        penalty: 0.0,
    });
    for start in 0..n {
        prune(&mut nodes[start]);
        if nodes[start].is_empty() {
            continue;
        }
        let mut any = false;
        for end in start + 1..=n.min(start + MAX_WORD_SYLLABLES) {
            let span = &positions[start..end];
            let hits = cache.get_or_insert_with(SpanCache::key(span), || {
                span_candidates(dictionaries, span, start, personal, &weight, &cost)
            });
            if hits.is_empty() {
                continue;
            }
            any = true;
            for hit in hits.iter() {
                let bonus = weight_bonus(weight(&hit.text));
                let fallback = fallback_log_prob(hit.frequency, log_total);
                let (score, back) = best_predecessor(
                    &nodes,
                    start,
                    search.initial,
                    &hit.text,
                    model,
                    personal,
                    fallback,
                );
                let previous = &nodes[start][back];
                // 原样成词保护：这条猜敲错的边整个落在一个原样读出的多音节词里面，多扣一份
                let guarded = hit.penalty > 0.0
                    && typed_reach[start] >= end
                    && hit.syllables.iter().enumerate().any(|(offset, syllable)| {
                        cost(start + offset, syllable) >= PROTECTED_MIN_COST
                    });
                let hit_penalty = if guarded {
                    hit.penalty + search.protected_extra
                } else {
                    hit.penalty
                };
                let penalty = previous.penalty + hit_penalty;
                let static_step = if start > 0 {
                    model
                        .log_prob(Some(previous.text.as_str()), &hit.text)
                        .unwrap_or(fallback)
                } else {
                    initial_log_prob(model, personal, search.initial, &hit.text, fallback)
                };
                let static_score = previous.static_score + static_step;
                nodes[end].push(Node {
                    start,
                    text: hit.text.clone(),
                    syllables: hit.syllables.clone(),
                    score: score + bonus - hit_penalty - personal.interpolation.word_penalty,
                    static_score,
                    back,
                    placeholder: false,
                    penalty,
                });
            }
        }
        // 这个音节连单字都查不到：用音节本身占位，别让整句断掉
        if !any {
            let text = positions[start][0].text;
            let (score, back) = best_predecessor(
                &nodes,
                start,
                search.initial,
                text,
                &NoModel,
                Personal::NONE,
                UNKNOWN_LOG_PROB,
            );
            let penalty = nodes[start][back].penalty;
            let static_score = nodes[start][back].static_score + UNKNOWN_LOG_PROB;
            nodes[start + 1].push(Node {
                start,
                text: text.to_owned(),
                syllables: vec![text.to_owned()],
                score,
                static_score,
                back,
                placeholder: true,
                penalty,
            });
        }
    }
    prune(&mut nodes[n]);
    let mut paths: Vec<Conversion> = Vec::with_capacity(k.min(nodes[n].len()));
    for index in 0..nodes[n].len() {
        if paths.len() >= k {
            break;
        }
        let conversion = backtrack(&nodes, n, index);
        if !paths.iter().any(|p| p.text == conversion.text) {
            paths.push(conversion);
        }
    }
    paths
}

/// 从 `nodes[position][index]` 回溯出整条路径。
fn backtrack(nodes: &[Vec<Node>], mut position: usize, mut index: usize) -> Conversion {
    let score = nodes[position][index].score;
    let static_score = nodes[position][index].static_score;
    let penalty = nodes[position][index].penalty;
    let mut words: Vec<SentenceWord> = Vec::new();
    while position > 0 {
        let node = &nodes[position][index];
        words.push(SentenceWord {
            text: node.text.clone(),
            syllables: node.syllables.clone(),
            placeholder: node.placeholder,
        });
        position = node.start;
        index = node.back;
    }
    words.reverse();
    let mut text = String::new();
    let mut syllables = Vec::new();
    for word in &words {
        text.push_str(&word.text);
        syllables.extend(word.syllables.iter().cloned());
    }
    Conversion {
        text,
        syllables,
        words,
        score,
        static_score,
        penalty,
    }
}

/// 占位音节不问语言模型。
struct NoModel;

impl LanguageModel for NoModel {
    fn log_prob(&self, _previous: Option<&str>, _word: &str) -> Option<f64> {
        None
    }
}

/// 一个格子里的候选词：所有词库的精确命中，按词频（加用户选择次数与个人出现次数，替代写法命中的按代价打折）取前几个。
/// 个人次数只在这里保证用户常用的同音词进得了格子，不进路径打分（那是 n-gram 的事）；
/// 打折让敲错变体命中的词只在原样命中不够多时才进格子，而常用词（关系）即使打折也留得住。
/// 格子里有简拼位置时命中的是一大片不同读音的词，多留一些让语言模型去挑。
fn span_candidates(
    dictionaries: &[&Dictionary],
    span: &[Vec<SyllablePattern<'_>>],
    start: usize,
    personal: Personal<'_>,
    weight: &impl Fn(&str) -> u32,
    cost: &impl Fn(usize, &str) -> f64,
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
        SPAN_CANDIDATES
    });
    scored
        .into_iter()
        .map(|(_, penalty, hit)| SpanWord {
            text: hit.text.to_owned(),
            syllables: hit.syllables().map(str::to_owned).collect(),
            frequency: hit.frequency,
            penalty,
        })
        .collect()
}

/// 在 `nodes[start]` 的前驱里挑让 `word` 得分最高的那条，返回 (累计得分, 前驱下标)。
/// 转移概率先问静态模型（不认识就用词库兜底值），再与个人 n-gram 插值；前二词是前驱自己的前驱（回指）。
/// 路径开头接 `initial`（这段拼音之前的上文）：第一个词的上文就是它，第二个词的前二词是它的 `previous`。
fn best_predecessor(
    nodes: &[Vec<Node>],
    start: usize,
    initial: Context<'_>,
    word: &str,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    fallback: f64,
) -> (f64, usize) {
    let mut best = (f64::NEG_INFINITY, 0);
    for (index, previous) in nodes[start].iter().enumerate() {
        let context = if start == 0 {
            initial
        } else {
            Context {
                previous: Some(previous.text.as_str()),
                earlier: if previous.start > 0 {
                    Some(nodes[previous.start][previous.back].text.as_str())
                } else {
                    initial.previous
                },
            }
        };
        let step = if start == 0 {
            initial_step(model, personal, initial, word, fallback)
        } else {
            transition_log_prob(model, personal, context, word, fallback)
        };
        let score = previous.score + step;
        if score > best.0 {
            best = (score, index);
        }
    }
    best
}

/// 一段拼音第一个词的静态 log 概率：「接着上文」与「句首」两种读法按 `initial_weight` 混（见 `INITIAL_CONTEXT_WEIGHT`）。
/// 没有上文就是句首；模型不认识的词两边都用 `fallback`。
fn initial_log_prob(
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    initial: Context<'_>,
    word: &str,
    fallback: f64,
) -> f64 {
    let fresh = model.log_prob(None, word).unwrap_or(fallback);
    let weight = personal.interpolation.initial_weight.clamp(0.0, 1.0);
    let Some(previous) = initial.previous.filter(|_| weight > 0.0) else {
        return fresh;
    };
    let following = model.log_prob(Some(previous), word).unwrap_or(fallback);
    mix(weight, following, fresh)
}

/// 一段拼音第一个词的路径得分：同 [`initial_log_prob`] 的混法，两种读法各自先与个人 n-gram 插值再混。
/// 个人部分不能只按上文算：个人二元里「前词 → 的」这类接续攒得最快，全按接续算 `dizhi` 会出 的只。
fn initial_step(
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    initial: Context<'_>,
    word: &str,
    fallback: f64,
) -> f64 {
    let fresh = transition_log_prob(model, personal, Context::START, word, fallback);
    let weight = personal.interpolation.initial_weight.clamp(0.0, 1.0);
    if initial.previous.is_none() || weight <= 0.0 {
        return fresh;
    }
    let following = transition_log_prob(model, personal, initial, word, fallback);
    mix(weight, following, fresh)
}

/// `ln(w·e^a + (1−w)·e^b)`。
fn mix(weight: f64, a: f64, b: f64) -> f64 {
    (weight * a.exp() + (1.0 - weight) * b.exp())
        .max(f64::MIN_POSITIVE)
        .ln()
}

/// 每个位置往右最远被「按敲的原样读出的多音节词」盖到哪：`reach[i] >= end` 说明 `[i, end)` 整个落在某个原样词里面。
///
/// `anpaiceshirenyuan` 里 `ce shi` 原样就是 测试，这时在 `ce` 上猜敲错（`de` → 的）多半是误伤：
/// 高频虚词的语言模型分盖得过一条敲错边的代价，句子越长这种机会越多。落在原样词里面的敲错边由调用方多扣 `extra`。
/// 只管「里面」：`mei gan xi` 的 没关系 比原样词 美感 长、伸到了外面，原样词解释不了整段，不拦。
/// 没有替代写法或 `extra` 为 0 时不扫；格子候选走同一张缓存，这里查过的主循环直接用。
fn typed_word_reach(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    extra: f64,
    personal: Personal<'_>,
    weight: &impl Fn(&str) -> u32,
    cost: &impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Vec<usize> {
    let n = positions.len();
    let mut reach = vec![0; n];
    if extra <= 0.0 || positions.iter().all(|p| p.len() < 2) {
        return reach;
    }
    for start in 0..n {
        for end in start + 2..=n.min(start + MAX_WORD_SYLLABLES) {
            let span = &positions[start..end];
            let hits = cache.get_or_insert_with(SpanCache::key(span), || {
                span_candidates(dictionaries, span, start, personal, weight, cost)
            });
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
                reach[start] = end;
            }
        }
    }
    // 起点更靠左的原样词也盖得住右边的位置
    for index in 1..n {
        reach[index] = reach[index].max(reach[index - 1]);
    }
    reach
}

/// 按得分降序只留束宽条。
fn prune(nodes: &mut Vec<Node>) {
    nodes.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    nodes.truncate(BEAM_WIDTH);
}

#[cfg(test)]
mod tests;
