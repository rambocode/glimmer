//! `wubi` 子命令：Rime 五笔码表（86 / 98 / 新世纪，四列同一格式）→ 微明词库 TSV `词\t编码\t词频`，编码整个当一个音节。
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
    let mut table = Table::from_path(input)?;
    let in_code_order_weights = apply_in_code_order_weights(&mut table.entries);
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
        in_code_order_weights,
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

/// 上游没给词频列的表，按**同一编码下的出现顺序**补词频：第 r 条（r 从 0 起）给 `1_000_000 / (r + 1)`，最小 1。改了返回真。
///
/// 为什么要补：新世纪码表 11.1 万行里有 11.1 万行只有 `词\t编码` 两列，weight 全是 0。Core 排同码候选时
/// 词频打平就退到按文本比大小（`glimmer-core` 的 `WubiHit::key`），候选顺序变成 Unicode 码位序，跟上游排好的常用度完全无关。
///
/// 为什么按编码分组而不是整表线性递减，两条原因缺一不可：
///
/// 1. Core 的分数是 `ln(词频 / 总频)` 再 `(score * 1000).round()` 取整比较。整表线性递减时相邻两条只差 1，
///    对数之后的差远小于千分之一，取整后完全打平，等于没补。名次倒数（`1/(r+1)`）让前几名的对数差拉开到取整能分辨。
/// 2. 上游文件除了开头 25 行一级简码，整体是**按编码排序**的，「文件顺序 = 常用度」只在同一编码组内成立；
///    跨编码组比大小没有意义，所以每组各自从 `1_000_000` 起算，组与组之间不可比。
///
/// 判据是「一半以上的行没有 weight 列」，整表一起换，不跟真词频混：86（99.9% 有 weight）与 98（全有）一行不动。
fn apply_in_code_order_weights(entries: &mut [Entry]) -> bool {
    let weighted = entries.iter().filter(|entry| entry.weighted).count();
    if entries.is_empty() || weighted * 2 >= entries.len() {
        return false;
    }
    // 先只读地数出每条在自己编码组里的名次，`seen` 借着 entries 的 &str；块结束借用释放，再改 weight
    let ranks: Vec<u32> = {
        let mut seen: HashMap<&str, u32> = HashMap::new();
        entries
            .iter()
            .map(|entry| {
                let next = seen.entry(entry.code.as_str()).or_insert(0);
                let rank = *next;
                *next += 1;
                rank
            })
            .collect()
    };
    for (entry, rank) in entries.iter_mut().zip(ranks) {
        entry.weight = (1_000_000 / (rank + 1)).max(1);
    }
    true
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
