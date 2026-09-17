//! 英文词表：精确查询、前缀补全及领域词库中的英文名称。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::DictionaryError;

/// 英文词表：按小写编码查原样大小写的词，也能按前缀补全。
/// 文件格式 TSV：`词\t编码[\t词频]`（编码缺省为词的小写，词频缺省 0）。
#[derive(Debug, Default)]
pub struct WordList {
    /// 小写编码 → 在 `entries` 里的下标。
    by_code: HashMap<String, usize>,

    /// (编码, 原样词, 词频)，按编码字节序排好，前缀补全二分定位。
    entries: Vec<(String, String, u32)>,
}

impl WordList {
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut by_code: HashMap<String, usize> = HashMap::new();
        let mut entries: Vec<(String, String, u32)> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let word = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: index + 1,
                    reason: "missing word",
                })?;
            let code = fields
                .next()
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_lowercase)
                .unwrap_or_else(|| word.to_ascii_lowercase());
            let frequency = fields
                .next()
                .map(|s| s.trim().parse::<u32>())
                .transpose()
                .map_err(|_| DictionaryError::Line {
                    line: index + 1,
                    reason: "frequency is not a non-negative integer",
                })?
                .unwrap_or(0);
            // 同一编码取第一个（词表按常用度排序时即最常用的写法）
            if by_code.contains_key(&code) {
                continue;
            }
            by_code.insert(code.clone(), entries.len());
            entries.push((code, word.to_owned(), frequency));
        }
        // 按编码排序后重建下标
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (index, (code, _, _)) in entries.iter().enumerate() {
            by_code.insert(code.clone(), index);
        }
        tracing::debug!(words = entries.len(), "英文词表加载完成");
        Ok(Self { by_code, entries })
    }

    /// 从附加词库提取显式别名：`@` 后为小写 ASCII 字母数字码，
    /// 例如 `C++` → `@cpp`、`Git分支` → `@gitfenzhi`。输出保持源词形；
    /// 未标记的普通拼音词与旧式 AI 混合词不进入英文表。
    /// 同一编码保留最先加载的词库，关闭或移除词库后重新构建即可撤销补全。
    pub fn from_dictionaries(dictionaries: &[crate::Dictionary]) -> Self {
        let mut list = Self::default();
        for entry in dictionaries.iter().flat_map(crate::Dictionary::entries) {
            let Some(code) = entry.pinyin.strip_prefix('@') else {
                continue;
            };
            if !valid_alias_code(code)
                || entry.text.is_empty()
                || entry.text.chars().any(char::is_control)
            {
                continue;
            }
            if let Some(&index) = list.by_code.get(code) {
                if list.entries[index].1 != entry.text {
                    // 导入词库可能没有经过生成器；保留既有加载顺序，同时明确报告冲突。
                    // 不记录词形或输入码，避免把项目内部名称写入运行日志。
                    tracing::warn!("领域词库存在冲突的别名编码，保留先加载的词条");
                }
                continue;
            }
            let code = code.to_owned();
            list.by_code.insert(code.clone(), list.entries.len());
            list.entries
                .push((code, entry.text.to_owned(), entry.frequency));
        }
        list.entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (index, (code, _, _)) in list.entries.iter().enumerate() {
            list.by_code.insert(code.clone(), index);
        }
        list
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// 输入（已小写）对应的英文词。
    pub fn get(&self, code: &str) -> Option<&str> {
        self.by_code
            .get(code)
            .map(|&index| self.entries[index].1.as_str())
    }

    /// 输入（已小写）对应的词频；词表里没有为 `None`。
    pub fn frequency(&self, code: &str) -> Option<u32> {
        self.by_code.get(code).map(|&index| self.entries[index].2)
    }

    /// 以 `prefix` 开头（不含正好相等的）的词里词频最高的 `limit` 个，按词频降序。
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<&str> {
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        let start = self
            .entries
            .partition_point(|(code, _, _)| code.as_str() < prefix);
        let mut hits: Vec<&(String, String, u32)> = self.entries[start..]
            .iter()
            .take_while(|(code, _, _)| code.starts_with(prefix))
            .filter(|(code, _, _)| code != prefix)
            .collect();
        hits.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        let mut seen = HashSet::new();
        hits.into_iter()
            .map(|(_, word, _)| word.as_str())
            .filter(|word| seen.insert(*word))
            .take(limit)
            .collect()
    }

    /// 全部词目：(小写编码, 原样词, 词频)，按编码字节序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, u32)> {
        self.entries
            .iter()
            .map(|(code, word, frequency)| (code.as_str(), word.as_str(), *frequency))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests;

/// 领域别名须以字母开头，只含小写 ASCII 字母数字，长度不超过 128 字节。
/// 不接受标点或空白，以免与选词、命令和普通拼音音节键混淆。
pub fn valid_alias_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 128
        && code.as_bytes()[0].is_ascii_lowercase()
        && code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}
