//! `wubi` 子命令：Rime 五笔码表（`wubi86.dict.yaml`）→ 微明词库 TSV `词\t编码\t词频`，编码整个当一个音节。
//!
//! 流程：解析头部与表体（`table`）→ 跳过非法码与含 `z` 的码（`entry`）→ 缺省按常用字集过滤（`charset`）→ 同（词，码）取最大词频 →
//! 按编码排序写出；顺带建单字全码表核对 stem 列（`full_codes`），给 Core 的反查表验数据质量。

mod charset;
mod encoder_rule;
mod entry;
mod full_codes;
mod header;
mod table;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::ConvertError;
use charset::Charset;
use entry::Entry;
use full_codes::FullCodes;
use table::Table;

/// 转换一份 Rime 码表。`extended` 为真跳过字符集过滤；`charset` 是并入常用字集的主词库（TSV 或 `.qj`）。
pub fn convert(
    input: &Path,
    output: &Path,
    extended: bool,
    charset: Option<&Path>,
) -> Result<(), ConvertError> {
    let table = Table::from_path(input)?;
    tracing::info!(
        path = %input.display(),
        name = table.header.name.as_deref().unwrap_or(""),
        version = table.header.version.as_deref().unwrap_or(""),
        columns = table.header.columns.join(","),
        rules = table.header.rules.len(),
        lines = table.lines,
        entries = table.entries.len(),
        comments = table.comments,
        invalid = table.invalid,
        z_codes = table.z_codes,
        "码表已读取"
    );
    let charset = if extended {
        None
    } else {
        Some(match charset {
            Some(path) => Charset::with_dictionary(path)?,
            None => Charset::gb2312(),
        })
    };
    let (entries, filtered, merged) = select(&table.entries, charset.as_ref());
    FullCodes::collect(&entries).report(&entries);
    write(&entries, output)?;
    tracing::info!(
        path = %output.display(),
        read = table.entries.len(),
        written = entries.len(),
        filtered,
        merged,
        invalid = table.invalid,
        z_codes = table.z_codes,
        "写出完成"
    );
    Ok(())
}

/// 字符集过滤 + 同（词，码）合并取最大词频 + 按（编码，词频降序，词）排序。返回（条目，过滤掉的条数，合并掉的条数）。
fn select(entries: &[Entry], charset: Option<&Charset>) -> (Vec<Entry>, usize, usize) {
    let mut filtered = 0;
    let mut merged: HashMap<(&str, &str), Entry> = HashMap::new();
    for entry in entries {
        if charset.is_some_and(|set| !set.accepts(&entry.text)) {
            filtered += 1;
            continue;
        }
        let key = (entry.text.as_str(), entry.code.as_str());
        match merged.get_mut(&key) {
            Some(kept) => {
                kept.weight = kept.weight.max(entry.weight);
                // stem 只在其中一行有时也保住
                if kept.stem.is_none() {
                    kept.stem.clone_from(&entry.stem);
                }
            }
            None => {
                merged.insert(key, entry.clone());
            }
        }
    }
    let merged_count = entries.len() - filtered - merged.len();
    let mut selected: Vec<Entry> = merged.into_values().collect();
    selected.sort_by(|a, b| {
        a.code
            .cmp(&b.code)
            .then(b.weight.cmp(&a.weight))
            .then(a.text.cmp(&b.text))
    });
    (selected, filtered, merged_count)
}

/// 写微明 TSV。
fn write(entries: &[Entry], output: &Path) -> Result<(), ConvertError> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        file,
        "# 由 glimmer-dict-convert wubi 生成。词\\t编码\\t词频（Rime weight）"
    )?;
    for entry in entries {
        writeln!(file, "{}\t{}\t{}", entry.text, entry.code, entry.weight)?;
    }
    file.flush()?;
    Ok(())
}
