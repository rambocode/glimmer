//! 反查表：汉字 → 全码。拼音反查给候选注五笔码、自动造词按字取码都靠它。

use std::collections::HashMap;

use glimmer_dictionary::Dictionary;

/// 单字的全码表，从码表的单字条目建：同一个字取最长的码（全码），等长取词频高的。
#[derive(Debug, Default, Clone)]
pub struct Reverse {
    /// 字 → 全码。
    codes: HashMap<char, String>,
}

impl Reverse {
    /// 扫一遍码表建表。只收单字条目；多字词的码由造词规则从单字全码算，不进这里。
    pub fn build(dictionary: &Dictionary) -> Self {
        let mut best: HashMap<char, (&str, u32)> = HashMap::new();
        for entry in dictionary.entries() {
            let mut chars = entry.text.chars();
            let (Some(ch), None) = (chars.next(), chars.next()) else {
                continue;
            };
            // 码表里编码整个是一个音节，不会带空格；万一带了（手改的 TSV），按第一段算
            let code = entry.pinyin.split(' ').next().unwrap_or_default();
            let replace = match best.get(&ch) {
                None => true,
                Some((current, frequency)) => {
                    code.len() > current.len()
                        || (code.len() == current.len() && entry.frequency > *frequency)
                }
            };
            if replace {
                best.insert(ch, (code, entry.frequency));
            }
        }
        Self {
            codes: best
                .into_iter()
                .map(|(ch, (code, _))| (ch, code.to_owned()))
                .collect(),
        }
    }

    /// 这个字的全码；码表里没有这个字为 `None`。
    pub fn code(&self, ch: char) -> Option<&str> {
        self.codes.get(&ch).map(String::as_str)
    }

    /// 表里有几个字。
    pub fn len(&self) -> usize {
        self.codes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.codes.is_empty()
    }
}
