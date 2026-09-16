//! Rime `.dict.yaml` 的 YAML 头：`---` 与 `...` 之间那几行。表体不是合法 YAML，所以不用 YAML 库，按行手工取需要的几个键。

use super::encoder_rule::EncoderRule;

/// 头部里我们关心的几个键；其他键忽略。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// `name:`。
    pub name: Option<String>,

    /// `version:`（去掉引号）。
    pub version: Option<String>,

    /// `columns:` 列表；没写时按 Rime 缺省 `text` / `code` / `weight`。
    pub columns: Vec<String>,

    /// `encoder.rules` 列表；没写为空。
    pub rules: Vec<EncoderRule>,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            name: None,
            version: None,
            columns: ["text", "code", "weight"]
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            rules: Vec::new(),
        }
    }
}

impl Header {
    /// 解析头部各行（不含 `---` / `...` 两行本身）。
    ///
    /// 只认三种结构：顶层 `key: value`、`columns:` 下面的 `- x` 列表、`encoder:` → `rules:` 下面的
    /// `- length_equal: N` / `- length_in_range: [A, B]` + `formula: "…"` 两行一组。缩进只用来判断「还在不在列表里」，
    /// 不做通用 YAML 解析。
    pub fn parse(lines: &[&str]) -> Self {
        let mut header = Self::default();
        let mut columns: Vec<String> = Vec::new();
        let mut section = Section::None;
        // 当前正在拼的规则：先看到 length_*，再看到 formula 才算完整
        let mut pending: Option<(usize, usize)> = None;
        for raw in lines {
            let line = raw.trim_end();
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            let body = line.trim();
            // 顶层键结束上一个列表段
            if indent == 0 {
                section = Section::None;
                pending = None;
                if let Some((key, value)) = body.split_once(':') {
                    let value = unquote(value);
                    match key.trim() {
                        "name" => header.name = non_empty(value),
                        "version" => header.version = non_empty(value),
                        "columns" => section = Section::Columns,
                        "encoder" => section = Section::Encoder,
                        _ => {}
                    }
                }
                continue;
            }
            match section {
                Section::Columns => {
                    if let Some(item) = body.strip_prefix("- ") {
                        columns.push(unquote(item).to_owned());
                    }
                }
                Section::Encoder => {
                    let item = body.strip_prefix("- ").unwrap_or(body);
                    if let Some((key, value)) = item.split_once(':') {
                        let value = unquote(value);
                        match key.trim() {
                            "length_equal" => {
                                pending = value.parse().ok().map(|n| (n, n));
                            }
                            "length_in_range" => {
                                pending = parse_range(value);
                            }
                            "formula" => {
                                if let Some((min_length, max_length)) = pending.take() {
                                    header.rules.push(EncoderRule {
                                        min_length,
                                        max_length,
                                        formula: value.to_owned(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Section::None => {}
            }
        }
        if !columns.is_empty() {
            header.columns = columns;
        }
        header
    }

    /// 某一列在表体里的下标（按 `columns` 顺序）。
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }
}

/// 头部里正在读的列表段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Columns,
    Encoder,
}

/// 去掉首尾空白与成对引号。
fn unquote(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
}

/// 空字符串当没写。
fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

/// `[4, 10]` → `(4, 10)`。
fn parse_range(value: &str) -> Option<(usize, usize)> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let (low, high) = inner.split_once(',')?;
    Some((low.trim().parse().ok()?, high.trim().parse().ok()?))
}
