//! 音标表（`tools/corpus/ipa_en.py` 从 ipa-dict 换写出的 `data/ipa/en_US-textbook.txt`，`词\t/音标/`）装进内存，按小写词查。
//! 这里不改写音标，写法风格全在那个脚本里定。

use std::collections::HashMap;
use std::path::Path;

use crate::error::GlossError;

/// 英文词 → 音标（不带斜线，第一种读法）。
#[derive(Debug, Default)]
pub struct IpaDict {
    entries: HashMap<String, String>,
}

impl IpaDict {
    pub fn load(path: &Path) -> Result<Self, GlossError> {
        Ok(Self::parse(&std::fs::read_to_string(path)?))
    }

    /// 一行 `词\t/音标/, /音标/`；只留第一种读法，去掉斜线，坏行跳过。
    pub fn parse(source: &str) -> Self {
        let mut entries = HashMap::new();
        for line in source.lines() {
            let Some((word, readings)) = line.split_once('\t') else {
                continue;
            };
            let Some(first) = readings.split(',').next() else {
                continue;
            };
            let ipa = first.trim().trim_matches('/');
            if !ipa.is_empty() {
                entries.insert(word.trim().to_ascii_lowercase(), ipa.to_owned());
            }
        }
        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 查一个译词：整体命中直接返回；否则按空格与连字符拆成词逐个查，全命中才拼成一条（词间空格）。
    pub fn lookup(&self, phrase: &str) -> Option<String> {
        let key = phrase.trim().to_ascii_lowercase();
        if let Some(ipa) = self.entries.get(&key) {
            return Some(ipa.clone());
        }
        let words: Vec<&str> = key.split([' ', '-']).filter(|w| !w.is_empty()).collect();
        if words.len() < 2 {
            return None;
        }
        let parts = words
            .iter()
            .map(|w| self.entries.get(*w).map(String::as_str))
            .collect::<Option<Vec<_>>>()?;
        Some(parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::IpaDict;

    #[test]
    fn parses_first_reading_verbatim() {
        let dict = IpaDict::parse("Run\t/rʌn/\nexploit\t/ˈeksˌplɔɪt/, /ˌeksˈplɔɪt/\nbad line\n");
        assert_eq!(dict.len(), 2);
        assert_eq!(dict.lookup("run").as_deref(), Some("rʌn"));
        assert_eq!(dict.lookup("Exploit").as_deref(), Some("ˈeksˌplɔɪt"));
    }

    #[test]
    fn phrases_need_every_word() {
        let dict = IpaDict::parse("ice\t/aɪs/\ncream\t/kriːm/\n");
        assert_eq!(dict.lookup("ice cream").as_deref(), Some("aɪs kriːm"));
        assert_eq!(dict.lookup("ice-cream").as_deref(), Some("aɪs kriːm"));
        assert_eq!(dict.lookup("ice cube"), None);
        assert_eq!(dict.lookup("cube"), None);
    }
}
