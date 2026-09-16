//! 单字全码表：每个单字取最长的码（Core 的反查表 char → 全码就是这么建的），顺带核对 stem 列、统计没有四码全码的字。

use std::collections::HashMap;

use super::entry::{Entry, MAX_CODE_LEN};

/// 单字 → 全码（最长码，等长取词频高的）。
#[derive(Debug, Default)]
pub struct FullCodes {
    /// 字 → （全码，词频）。
    codes: HashMap<char, (String, u32)>,
}

impl FullCodes {
    /// 从条目里收集所有单字的最长码。
    pub fn collect(entries: &[Entry]) -> Self {
        let mut codes: HashMap<char, (String, u32)> = HashMap::new();
        for entry in entries {
            let mut chars = entry.text.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else {
                continue;
            };
            // 元组比较：先比码长，再比词频
            let better = codes.get(&c).is_none_or(|(code, weight)| {
                (entry.code.len(), entry.weight) > (code.len(), *weight)
            });
            if better {
                codes.insert(c, (entry.code.clone(), entry.weight));
            }
        }
        Self { codes }
    }

    /// 某个字的全码。
    pub fn get(&self, c: char) -> Option<&str> {
        self.codes.get(&c).map(|(code, _)| code.as_str())
    }

    /// 单字数。
    pub fn len(&self) -> usize {
        self.codes.len()
    }

    /// 全码不足四位的字（按字排序，输出稳定）。
    pub fn short(&self) -> Vec<(char, &str)> {
        let mut short: Vec<(char, &str)> = self
            .codes
            .iter()
            .filter(|(_, (code, _))| code.len() < MAX_CODE_LEN)
            .map(|(c, (code, _))| (*c, code.as_str()))
            .collect();
        short.sort_unstable();
        short
    }

    /// 核对 stem 列：构词码必须是该字全码的前缀（一级简码的 stem 是全码前两位）。返回不一致的（字，stem，全码）。
    pub fn check_stems(&self, entries: &[Entry]) -> Vec<(char, String, String)> {
        let mut mismatches = Vec::new();
        for entry in entries {
            let Some(stem) = &entry.stem else {
                continue;
            };
            let mut chars = entry.text.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else {
                continue;
            };
            let full = self.get(c).unwrap_or_default();
            if !full.starts_with(stem.as_str()) {
                mismatches.push((c, stem.clone(), full.to_owned()));
            }
        }
        mismatches
    }

    /// 把统计打到日志：单字数、有四码全码的、没有的（及前 20 个例子）、stem 不一致的。
    pub fn report(&self, entries: &[Entry]) {
        let short = self.short();
        let examples: Vec<String> = short
            .iter()
            .take(20)
            .map(|(c, code)| format!("{c}:{code}"))
            .collect();
        tracing::info!(
            chars = self.len(),
            four_code = self.len() - short.len(),
            short = short.len(),
            examples = examples.join(" "),
            "单字全码统计"
        );
        let mismatches = self.check_stems(entries);
        if !mismatches.is_empty() {
            let examples: Vec<String> = mismatches
                .iter()
                .take(10)
                .map(|(c, stem, full)| format!("{c}:{stem}≠{full}"))
                .collect();
            tracing::warn!(
                count = mismatches.len(),
                examples = examples.join(" "),
                "stem 列与单字全码不一致"
            );
        }
    }
}
