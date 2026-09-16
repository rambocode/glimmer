//! 常用字集：GB2312 一二级汉字 ∪ 主词库里出现过的字。缺省只保留每个字都在这个集合里的条目，生僻字与扩展区字留给「增广字集」。

use std::collections::HashSet;
use std::path::Path;

use glimmer_dictionary::Dictionary;

use crate::error::ConvertError;

/// 字符集过滤器。
#[derive(Debug, Default)]
pub struct Charset {
    /// 主词库里出现过的字（GB2312 之外的补充）。
    extra: HashSet<char>,
}

impl Charset {
    /// 只用 GB2312。
    pub fn gb2312() -> Self {
        Self::default()
    }

    /// GB2312 再并上一本词库（TSV 或 `.qj`）里出现过的所有字。
    pub fn with_dictionary(path: &Path) -> Result<Self, ConvertError> {
        let dictionary = Dictionary::from_path(path)?;
        let extra: HashSet<char> = dictionary
            .entries()
            .flat_map(|entry| entry.text.chars())
            .collect();
        tracing::info!(path = %path.display(), chars = extra.len(), "主词库字集已读取");
        Ok(Self { extra })
    }

    /// 单个字是否在常用字集里。
    pub fn contains(&self, c: char) -> bool {
        is_gb2312(c) || self.extra.contains(&c)
    }

    /// 词的每个字都在常用字集里。
    pub fn accepts(&self, text: &str) -> bool {
        text.chars().all(|c| self.contains(c))
    }
}

/// 是不是 GB2312 一二级汉字：编成 GBK 后落在 0xB0A1–0xF7FE（GBK 与 GB2312 在这一段完全一致；0xD7FA–0xD7FE 是一级区末尾的空位）。
pub fn is_gb2312(c: char) -> bool {
    let mut buffer = [0u8; 4];
    let (bytes, _, had_errors) = encoding_rs::GBK.encode(c.encode_utf8(&mut buffer));
    if had_errors || bytes.len() != 2 {
        return false;
    }
    let (high, low) = (bytes[0], bytes[1]);
    (0xB0..=0xF7).contains(&high) && (0xA1..=0xFE).contains(&low) && !(high == 0xD7 && low >= 0xFA)
}
