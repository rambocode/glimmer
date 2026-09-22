use std::collections::HashMap;

use glimmer_dictionary::Dictionary;

/// 多音字的读音份额表：一个词按某个读音命中时，该读音占这个词全部读音的多少。
///
/// 语言模型只看字不看音：`P(没 | 上文)` 是 没 两个读音（mei 92 万、mo 7 千）合起来的概率，
/// 而词库早已按读音把词频分开了。`mohuiyu` 里 没(mo) 拿着 没(mei) 的分去跟 莫 / 末 比，自然赢。
/// 减掉 `ln(该读音词频 / 该词全部读音词频之和)` 就把模型给的那份还原成这个读音的（没(mo) 约 −4.85）。
///
/// 词库里 871 个词有两个以上读音（没 / 会 / 了 / 和 / 大 / 的 / 给 / 呢 / 么 …），表里只存这些，
/// 单读音词查不到就是不扣。多字词同样适用：银行 / 长江 / 重庆 这类整词只有一个读音，份额是 1。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReadingShare {
    /// 多读音词 → 全部读音的词频之和。
    totals: HashMap<Box<str>, u32>,
}

impl ReadingShare {
    /// 扫一遍全部词库建表。词库没有「按汉字查读音」的索引，只能整表过一遍（9 万条词目几毫秒），
    /// 所以只在装词库时建一次，不是每次查询都算。
    ///
    /// 一个词在几个词库里都有时词频相加、读音合并；判断「有没有第二个读音」只记第一个读音再逐条比，
    /// 免得给 9 万个词各开一个集合。
    pub fn build(dictionaries: &[&Dictionary]) -> Self {
        let mut seen: HashMap<&str, (u64, &str, bool)> = HashMap::new();
        for dictionary in dictionaries {
            for entry in dictionary.entries() {
                match seen.get_mut(entry.text) {
                    Some((total, first, multi)) => {
                        *total += u64::from(entry.frequency);
                        *multi |= *first != entry.pinyin;
                    }
                    None => {
                        seen.insert(
                            entry.text,
                            (u64::from(entry.frequency), entry.pinyin, false),
                        );
                    }
                }
            }
        }
        let totals = seen
            .into_iter()
            .filter(|(_, (_, _, multi))| *multi)
            .map(|(text, (total, _, _))| {
                (Box::from(text), u32::try_from(total).unwrap_or(u32::MAX))
            })
            .collect();
        Self { totals }
    }

    /// 这条词目按它自己的读音命中时要扣的分（非负，0 为不扣）。
    /// `frequency` 是这条词目的词频（词库本来就按读音分开记），`weight` 是系数（0 关掉），`cap` 是封顶。
    ///
    /// 封顶是必须的：词库里有词频记成个位数的冷门读音，不封顶一条边就能扣掉二三十分，
    /// 比任何语言模型分都大，等于把那个读音从词图里删掉。
    pub fn penalty(&self, text: &str, frequency: u32, weight: f64, cap: f64) -> f64 {
        let cap = cap.max(0.0);
        if weight <= 0.0 || cap <= 0.0 {
            return 0.0;
        }
        let Some(&total) = self.totals.get(text) else {
            return 0.0;
        };
        if frequency >= total {
            return 0.0;
        }
        // 词频记成 0 的读音份额是 0，−ln(0) 是无穷，直接按封顶算
        let share = f64::from(frequency) / f64::from(total);
        let raw = if share > 0.0 { -share.ln() } else { cap };
        (weight * raw).min(cap)
    }

    /// 表里有几个多读音词。
    pub fn len(&self) -> usize {
        self.totals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.totals.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use glimmer_dictionary::Dictionary;

    use super::ReadingShare;

    const WORDS: &str = "没\tmei\t900\n没\tmo\t100\n莫\tmo\t500\n银行\tyin hang\t700\n";

    fn share() -> ReadingShare {
        let dictionary = Dictionary::parse(WORDS).unwrap();
        ReadingShare::build(&[&dictionary])
    }

    #[test]
    fn only_multi_reading_words_are_kept() {
        let share = share();
        assert_eq!(share.len(), 1);
        assert_eq!(share.penalty("莫", 500, 1.0, 6.0), 0.0);
        assert_eq!(share.penalty("银行", 700, 1.0, 6.0), 0.0);
    }

    #[test]
    fn rare_reading_pays_the_log_of_its_share() {
        let share = share();
        let expected = -(0.1f64).ln();
        assert!((share.penalty("没", 100, 1.0, 6.0) - expected).abs() < 1e-9);
        // 常见读音那一条也按份额扣，只是几乎为 0
        let common = -(0.9f64).ln();
        assert!((share.penalty("没", 900, 1.0, 6.0) - common).abs() < 1e-9);
    }

    #[test]
    fn weight_scales_and_cap_limits() {
        let share = share();
        assert_eq!(share.penalty("没", 100, 0.0, 6.0), 0.0);
        let half = share.penalty("没", 100, 0.5, 6.0);
        assert!((half - -(0.1f64).ln() / 2.0).abs() < 1e-9);
        assert_eq!(share.penalty("没", 100, 1.0, 1.0), 1.0);
    }

    #[test]
    fn several_dictionaries_share_one_total() {
        let main = Dictionary::parse("没\tmei\t900\n").unwrap();
        let extra = Dictionary::parse("没\tmo\t100\n").unwrap();
        let share = ReadingShare::build(&[&main, &extra]);
        assert_eq!(share.len(), 1);
        assert!((share.penalty("没", 100, 1.0, 6.0) - -(0.1f64).ln()).abs() < 1e-9);
    }
}
