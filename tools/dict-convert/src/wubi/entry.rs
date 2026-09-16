//! 码表表体的一行：`text\tcode\tweight[\tstem]`（列顺序以头部 `columns` 为准）。

use super::header::Header;

/// 编码最长几位：五笔 86 / 98 都是四码上屏，更长的码 Core 永远敲不到。
pub const MAX_CODE_LEN: usize = 4;

/// 一条码表记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// 字或词。
    pub text: String,

    /// 编码，只含 `a`–`z`。
    pub code: String,

    /// 词频（Rime 的 weight 列）；缺省 0。
    pub weight: u32,

    /// 构词码（Rime 的 stem 列），只有一级简码那几行有。
    pub stem: Option<String>,
}

/// 一行为什么没解析成记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// 缺 text 或 code 列，或编码含 `a`–`z` 以外的字符 / 超过 [`MAX_CODE_LEN`]。
    Invalid,

    /// 编码含 `z`：`z` 键留给反查与通配，这些条目（符号表）敲不到。
    ZCode,
}

impl Entry {
    /// 按头部的列顺序解析一行；`#` 注释与空行由调用方先跳过。
    pub fn parse(line: &str, header: &Header) -> Result<Self, Rejection> {
        let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
        let field = |name: &str| header.column(name).and_then(|i| fields.get(i).copied());
        let text = field("text")
            .filter(|t| !t.is_empty())
            .ok_or(Rejection::Invalid)?;
        let code = field("code")
            .filter(|c| !c.is_empty())
            .ok_or(Rejection::Invalid)?;
        if code.contains('z') {
            return Err(Rejection::ZCode);
        }
        if code.len() > MAX_CODE_LEN || !code.bytes().all(|b| b.is_ascii_lowercase()) {
            return Err(Rejection::Invalid);
        }
        let weight = field("weight")
            .filter(|w| !w.is_empty())
            .map(parse_weight)
            .unwrap_or(0);
        let stem = field("stem").filter(|s| !s.is_empty()).map(str::to_owned);
        Ok(Self {
            text: text.to_owned(),
            code: code.to_owned(),
            weight,
            stem,
        })
    }
}

/// Rime 的 weight 可以写成 `1e5` 这种浮点；解析失败给 0，超出 u32 截到上限。
fn parse_weight(raw: &str) -> u32 {
    raw.parse::<f64>()
        .ok()
        .filter(|w| w.is_finite() && *w >= 0.0)
        .map_or(0, |w| w.min(f64::from(u32::MAX)) as u32)
}
