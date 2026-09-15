//! 自定义短语配置及共享校验、预览；由 Core 匹配，平台负责编辑与展示。

use serde::{Deserialize, Serialize};

/// 按原始输入码匹配的固定位置文本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPhrase {
    /// 小写英文字母输入码，1–32 个字符。
    pub code: String,

    /// 原样上屏的文本，保留空格和换行。
    pub text: String,

    /// 从 1 开始的固定候选位置，最大 9。
    pub position: usize,

    /// 是否启用；停用规则仍保留位置。
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn enabled() -> bool {
    true
}

/// 保存和加载使用同一校验，不覆盖冲突条目。
pub fn validate_phrases(phrases: &[CustomPhrase]) -> Result<(), String> {
    let mut occupied = std::collections::BTreeSet::new();
    for phrase in phrases {
        if phrase.code.is_empty()
            || phrase.code.len() > 32
            || !phrase.code.bytes().all(|c| c.is_ascii_lowercase())
        {
            return Err("输入码须为 1–32 个小写英文字母".into());
        }
        if !(1..=9).contains(&phrase.position) {
            return Err("候选位置须为 1–9".into());
        }
        if phrase.text.is_empty() {
            return Err("自定义短语不能为空".into());
        }
        if !occupied.insert((&phrase.code, phrase.position)) {
            return Err(format!(
                "输入码 {} 的第 {} 位已被占用，不能保存",
                phrase.code, phrase.position
            ));
        }
    }
    Ok(())
}

impl CustomPhrase {
    /// 单行预览保留 Unicode 字符边界，用可见符号表示换行和制表符。
    pub fn preview(text: &str, max_chars: usize) -> String {
        let mut chars = text.chars().peekable();
        let mut preview = String::new();
        for _ in 0..max_chars {
            let Some(c) = chars.next() else {
                break;
            };
            preview.push(match c {
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    '↵'
                }
                '\n' => '↵',
                '\t' => '⇥',
                _ => c,
            });
        }
        if chars.next().is_some() {
            preview.push('…');
        }
        preview
    }
}
