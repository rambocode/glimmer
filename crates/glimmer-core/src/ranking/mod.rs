//! 候选排序。
//!
//! 词级排序规则：
//! 1. 音节数与输入完全一致的词优先（`kaifa` → 开发 排在 开发者 前）
//! 2. 覆盖输入字母多者优先（`kaif` → 开发者 排在 开 前）
//! 3. 切分里非末尾的简拼音节少者优先（`kaifa` 按 `kai fa` 读的 开放 排在按 `kai f a` 读的 开放啊 前）
//! 4. 最后一个音节完整匹配优先（`kaifa` → 开发 排在 开放 前）
//! 5. 得分：语言模型给的 `log P(词 | 上一个上屏的词)`（个人 bigram 插值，模型不认识的按词库词频兜底并扣分，
//!    见 `sentence::transition_log_prob`），加同一输入串下、同一位置（句首 / 句中）选过的加分（[`choice_bonus`]）
//!    与用户选择总次数的加分（[`weight_bonus`]），减模糊音命中的 ln 2、敲错变体那类敲错的代价
//!    （`correction::TypoKind::cost`，个人敲错表打折）。
//!    这样 `ba` 在「做了」后面出 吧、句首出 把；纯词频排序两处都只能出同一个
//! 6. 敲的原音节优先，词长短者优先，最后按字符串稳定排序保证结果可复现
//!
//! 「同输入串下选过几次」（`mgs` 选过 美国式）以前是压在上下文之上的**硬**规则，一次选择就能锁死首选：
//! 用户的 `youjian` 选过 右键 3 次、邮件 0 次，于是「收到」后面的 `youjian` 也只出 右键，而语言模型里「收到 → 邮件」有 104 次。
//! 改成得分里的一项之后，选择记录仍然压得过静态词频（简拼 `mgs` 这种语言模型没意见的照旧命中），但上下文足够强时能把正确的词纠回来。
//! 位置分桶（句首 / 句中）保留：同一个加分项在两处给同一个词，`ba` 还是会在句首被 吧 占住。
//!
//! 词库静态词频只用于预选（命中太多时先按词频砍到够排的量）与兜底。

mod scored;

use std::collections::HashSet;

pub use scored::{PreselectKey, Scored, SortKey};

/// 用户选择次数的加分系数：加分 = 系数 × ln(1 + min(次数, [`WEIGHT_CAP`]))。
/// 这个加分只负责「同音词里偏向用户常选的那个」，所以既取对数又封顶（最多约 1.5 分，e^1.5 ≈ 4.5 倍），
/// 让上下文（bigram）仍能压过它；不封顶时选过 173 次的 的 会带着 5 分加分把任何含 的 的拆分路径抬到整词之上
/// （`haode` 出 号的 而不是 好的）。用户用得多的词另有个人 bigram 里的一元项撑腰，不靠这里。
pub const WEIGHT_BONUS: f64 = 0.5;

/// 加分里计入的选择次数上限。
pub const WEIGHT_CAP: u32 = 20;

/// 模糊音命中扣的分（词频减半）。敲错变体的代价见 `correction::TypoKind`。
pub const FUZZY_PENALTY: f64 = std::f64::consts::LN_2;

/// 同一输入串、同一位置下选过次数的加分系数：加分 = 系数 × ln(1 + min(次数, [`CHOICE_CAP`]))。
///
/// 比 [`WEIGHT_BONUS`] 大：那个是「这个词你总共选得多」，这个是「这段字母你就是要这个词」，后者证据强得多。
/// 取对数且封顶是因为它要能被上下文压回来：1.5 换算过去是选过 1 次约抵 2.8 倍词频、3 次抵 8 倍、封顶抵 96 倍，
/// 一条像样的 bigram 就能盖过偶尔的一次误选，攒了几十次的习惯则要很强的上文才翻得动。
/// 不封顶时选过上百次的词会在任何上文后面都当首选，等于退回改之前的硬规则。
///
/// 值是回放扫出来的：`choice` 从 0.5 扫到 3、封顶 5 / 10 / 20 / 30，词级命中从 8795 涨到 8822（不设加分是 8766、
/// 改之前的硬规则是 8826），1.5 以上就进平台区。取平台区里最软的一档——回放这把尺子的「对」就是「上次这段字母选的那个词」，
/// 天生偏向硬规则，所以不拿它去挑最大的系数。表见 `docs/notes/constant-sweep.md`。
pub const CHOICE_BONUS: f64 = 1.5;

/// 加分里计入的同输入串选择次数上限，见 [`CHOICE_BONUS`]；与 [`WEIGHT_CAP`] 取同一个数。
pub const CHOICE_CAP: u32 = 20;

/// 用户选择次数换算成得分加成，词级排序与整句路径共用，见 [`WEIGHT_BONUS`]。
pub fn weight_bonus(count: u32) -> f64 {
    WEIGHT_BONUS * (1.0 + f64::from(count.min(WEIGHT_CAP))).ln()
}

/// 同输入串、同位置下的选择次数换算成得分加成，见 [`CHOICE_BONUS`]。
/// 系数与封顶从外面给（`Interpolation` 里可调，回放扫参用），缺省是 [`CHOICE_BONUS`] / [`CHOICE_CAP`]。
pub fn choice_bonus(count: u32, coefficient: f64, cap: u32) -> f64 {
    coefficient * (1.0 + f64::from(count.min(cap))).ln()
}

/// 排序并按词文本去重（同一个词可能被多种切分命中，保留得分最高的一条），最多留 `limit` 条。
/// `context` 给每条命中算上下文那部分的得分（`log P(词 | 上文)` 加同输入串选过的加分 [`choice_bonus`]），
/// 只对预选后剩下的那些调用；词自身的加分与扣分（[`weight_bonus`]、模糊音 / 敲错代价）由这里补上。
///
/// 排序键先算好再排：单字母简拼能命中两万条，比较器里每次数字符数会让排序占掉几十毫秒；去重也只做到够数为止。
pub fn rank(items: &mut Vec<Scored<'_>>, limit: usize, context: impl Fn(&Scored<'_>) -> f64) {
    // 远超上限时先按结构键 + 词频线性选出前面一段：同一个词会被多种切分命中，多选一倍留给去重（结果仍可能略少于上限，无妨）
    let preselect = limit.saturating_mul(2);
    if items.len() > preselect.saturating_mul(2) {
        let mut keyed: Vec<(PreselectKey, Scored<'_>)> = items
            .drain(..)
            .map(|item| (item.preselect_key(), item))
            .collect();
        keyed.select_nth_unstable_by(preselect, |a, b| b.0.cmp(&a.0));
        keyed.truncate(preselect);
        items.extend(keyed.into_iter().map(|(_, item)| item));
    }
    let mut keyed: Vec<(SortKey<'_>, Scored<'_>)> = items
        .drain(..)
        .map(|item| {
            let score = context(&item) + weight_bonus(item.weight) - item.penalty;
            (item.key(score), item)
        })
        .collect();
    keyed.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let mut seen: HashSet<&str> = HashSet::with_capacity(limit.min(keyed.len()));
    items.extend(
        keyed
            .into_iter()
            .map(|(_, item)| item)
            .filter(|item| seen.insert(item.hit.text))
            .take(limit),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use glimmer_dictionary::Match;

    fn hit<'a>(text: &'a str, pinyin: &'a str, frequency: u32, exact: bool) -> Match<'a> {
        Match {
            text,
            pinyin,
            frequency,
            exact,
        }
    }

    #[test]
    fn exact_beats_frequency_and_weight_beats_static_frequency() {
        let mut items = vec![
            Scored {
                hit: hit("开发者", "kai fa zhe", 99999, false),
                full_last: true,
                coverage: 5,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
            Scored {
                hit: hit("开放", "kai fang", 20000, true),
                full_last: false,
                coverage: 5,
                abbreviated: 0,
                weight: 5,
                penalty: 0.0,
            },
            Scored {
                hit: hit("开发", "kai fa", 9000, true),
                full_last: true,
                coverage: 5,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
        ];
        rank(&mut items, usize::MAX, |_| 0.0);
        let texts: Vec<&str> = items.iter().map(|s| s.hit.text).collect();
        assert_eq!(texts, ["开发", "开放", "开发者"]);
    }

    #[test]
    fn context_score_orders_within_the_same_structure() {
        let mut items = vec![
            Scored {
                hit: hit("把", "ba", 3_000_000, true),
                full_last: true,
                coverage: 2,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
            Scored {
                hit: hit("吧", "ba", 2_000_000, true),
                full_last: true,
                coverage: 2,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
        ];
        // 上下文说 吧 更像：词频高的 把 让位
        let by_context = |s: &Scored<'_>| if s.hit.text == "吧" { -1.0 } else { -6.0 };
        rank(&mut items, usize::MAX, by_context);
        let texts: Vec<&str> = items.iter().map(|s| s.hit.text).collect();
        assert_eq!(texts, ["吧", "把"]);
        // 同输入串下选过 把：加分是软的，压不过 6 nat 的上下文差（以前这一级是硬规则，选过一次就锁死首选）
        let with_choice = |gap: f64, count: u32| {
            move |s: &Scored<'_>| {
                let choice = u32::from(s.hit.text == "把") * count;
                choice_bonus(choice, CHOICE_BONUS, CHOICE_CAP)
                    + if s.hit.text == "吧" {
                        -1.0
                    } else {
                        -1.0 - gap
                    }
            }
        };
        rank(&mut items, usize::MAX, with_choice(6.0, CHOICE_CAP));
        assert_eq!(items[0].hit.text, "吧");
        // 上下文只差半分时选过几次就翻得过来
        rank(&mut items, usize::MAX, with_choice(0.5, 3));
        assert_eq!(items[0].hit.text, "把");
        // 同分时用户选过的、非模糊音的靠前
        for item in &mut items {
            item.weight = u32::from(item.hit.text == "把") * 3;
        }
        rank(&mut items, usize::MAX, |_| -2.0);
        assert_eq!(items[0].hit.text, "把");
        for item in &mut items {
            item.weight = 3;
            item.penalty = if item.hit.text == "把" {
                FUZZY_PENALTY
            } else {
                0.0
            };
        }
        rank(&mut items, usize::MAX, |_| -2.0);
        assert_eq!(items[0].hit.text, "吧");
    }

    #[test]
    fn fewer_abbreviated_syllables_win_at_equal_coverage() {
        let mut items = vec![
            Scored {
                hit: hit("开放啊", "kai fang a", 132, true),
                full_last: true,
                coverage: 5,
                abbreviated: 1,
                weight: 0,
                penalty: 0.0,
            },
            Scored {
                hit: hit("开放", "kai fang", 500_000, true),
                full_last: false,
                coverage: 5,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
        ];
        rank(&mut items, usize::MAX, |_| 0.0);
        assert_eq!(items[0].hit.text, "开放");
    }

    #[test]
    fn weight_bonus_is_logarithmic_and_capped() {
        assert_eq!(weight_bonus(0), 0.0);
        assert!(weight_bonus(1) < weight_bonus(10));
        assert_eq!(weight_bonus(WEIGHT_CAP), weight_bonus(WEIGHT_CAP * 50));
        assert!(weight_bonus(u32::MAX) < 1.6);
    }

    #[test]
    fn choice_bonus_is_logarithmic_and_capped() {
        assert_eq!(choice_bonus(0, CHOICE_BONUS, CHOICE_CAP), 0.0);
        assert!(
            choice_bonus(1, CHOICE_BONUS, CHOICE_CAP) < choice_bonus(5, CHOICE_BONUS, CHOICE_CAP)
        );
        assert_eq!(
            choice_bonus(CHOICE_CAP, CHOICE_BONUS, CHOICE_CAP),
            choice_bonus(u32::MAX, CHOICE_BONUS, CHOICE_CAP)
        );
        // 选过的加分比「这个词总共选得多」的加分重
        assert!(choice_bonus(3, CHOICE_BONUS, CHOICE_CAP) > weight_bonus(3));
    }

    #[test]
    fn deduplicates_by_text_keeping_best() {
        let mut items = vec![
            Scored {
                hit: hit("西安", "xi an", 4000, false),
                full_last: true,
                coverage: 4,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
            Scored {
                hit: hit("西安", "xi an", 4000, true),
                full_last: true,
                coverage: 4,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
        ];
        rank(&mut items, usize::MAX, |_| 0.0);
        assert_eq!(items.len(), 1);
        assert!(items[0].hit.exact);
    }
}
