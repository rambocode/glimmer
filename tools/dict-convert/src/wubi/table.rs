//! 整个 Rime 码表文件：头部 + 表体记录 + 跳过计数。

use std::path::Path;

use super::entry::{Entry, Rejection};
use super::header::Header;
use crate::error::ConvertError;

/// 解析完的码表。
#[derive(Debug, Default)]
pub struct Table {
    /// YAML 头。
    pub header: Header,

    /// 表体记录，保持文件顺序，未去重。
    pub entries: Vec<Entry>,

    /// 文件总行数（含头部与注释）。
    pub lines: usize,

    /// 跳过的注释行与空行数（头部之前的文件头注释也算）。
    pub comments: usize,

    /// 因缺列 / 非法编码跳过的行数。
    pub invalid: usize,

    /// 因编码含 `z` 跳过的行数。
    pub z_codes: usize,
}

impl Table {
    /// 读文件再解析。
    pub fn from_path(path: &Path) -> Result<Self, ConvertError> {
        Ok(Self::parse(&std::fs::read_to_string(path)?))
    }

    /// 解析整个文件文本。
    ///
    /// 文件分三段：`---` 之前只有注释；`---` 到 `...` 是 YAML 头；之后是表体。先定位头部范围并解析出列序，
    /// 再按列序解析表体。没有 `---` 的文件（少数手写表）从第一条含制表符的行起当表体。
    ///
    /// 开头先剥 UTF-8 BOM：上游码表有的是 Windows 编辑器存的，带 BOM 时第一行会变成 `\u{feff}# Rime …`，
    /// `---` 与 `#` 的判断全部错位，头部会整段读不到。
    pub fn parse(text: &str) -> Self {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
        let mut table = Self {
            lines: lines.len(),
            ..Self::default()
        };
        let mut header_lines: Vec<&str> = Vec::new();
        let mut in_header = false;
        let mut body_start = lines.len();
        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if in_header {
                if trimmed == "..." {
                    body_start = index + 1;
                    break;
                }
                header_lines.push(line);
            } else if trimmed == "---" {
                in_header = true;
            } else if line.contains('\t') {
                body_start = index;
                break;
            } else {
                table.comments += 1;
            }
        }
        table.header = Header::parse(&header_lines);
        for line in &lines[body_start..] {
            table.push(line);
        }
        table
    }

    /// 表体一行按头部列序解析并计数。
    fn push(&mut self, line: &str) {
        if line.trim().is_empty() || line.starts_with('#') {
            self.comments += 1;
            return;
        }
        match Entry::parse(line, &self.header) {
            Ok(entry) => self.entries.push(entry),
            Err(Rejection::Invalid) => {
                if self.invalid < 5 {
                    tracing::warn!(line, "码表行无法解析，跳过");
                }
                self.invalid += 1;
            }
            Err(Rejection::ZCode) => self.z_codes += 1,
        }
    }
}
