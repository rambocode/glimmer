//! AI 领域词表的严格校验、去重与打包；人工权重与正文出现次数分列记录。

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

use glimmer_core::parser::is_syllable;
use glimmer_dictionary::Dictionary;
use glimmer_format::Metadata;

use crate::error::ConvertError;

/// 校验所有源文件后生成 AI 领域词库与审计表。任何格式错误均带源文件行号返回；
/// 主词库与其他领域词库的重叠项仍留在源文件中，但不重复写入生成词库。
pub fn convert(
    source: &Path,
    exclude: &[PathBuf],
    corpus: &[PathBuf],
    out: &Path,
) -> Result<(), ConvertError> {
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(source.join("sources.json"))?)?;
    let mut existing = HashSet::new();
    for path in exclude {
        let dict = Dictionary::from_path(path)?;
        existing.extend(dict.entries().map(|e| {
            (
                e.text.to_owned(),
                e.pinyin.strip_prefix('@').unwrap_or(e.pinyin).to_owned(),
            )
        }));
    }
    let mut text = String::new();
    for path in corpus {
        let content = std::fs::read_to_string(path)?;
        // 文件间、段落间不拼接，避免产生不存在的跨边界匹配。
        for line in content.lines().filter(|l| !l.trim_start().starts_with('#')) {
            text.push_str(line);
            text.push('\n');
        }
    }
    let mut rows = BTreeMap::new();
    for file in ["terms.tsv", "names.tsv"] {
        let path = source.join(file);
        for (index, line) in std::fs::read_to_string(&path)?.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            validate(&fields).map_err(|reason| ConvertError::Format {
                path: path.clone(),
                line: index + 1,
                reason,
            })?;
            if !manifest
                .get(fields[6])
                .and_then(|s| s.get("license"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|license| {
                    matches!(
                        license,
                        "GPL-3.0-or-later" | "Apache-2.0" | "CC-BY-SA-4.0" | "name-facts"
                    )
                })
            {
                return Err(ConvertError::Format {
                    path: path.clone(),
                    line: index + 1,
                    reason: "missing source or unsupported source license".into(),
                });
            }
            let key = (fields[0].to_owned(), fields[1].to_owned());
            if rows
                .insert(
                    key,
                    fields.iter().map(|f| (*f).to_owned()).collect::<Vec<_>>(),
                )
                .is_some()
            {
                return Err(ConvertError::Format {
                    path: path.clone(),
                    line: index + 1,
                    reason: "duplicate word and code".into(),
                });
            }
        }
    }
    let mut dictionary = String::from("# AI 与人工智能：词\t编码\t权重\n");
    let mut audit =
        String::from("# 词\t编码\t人工权重\t正文出现次数\t最终权重\t状态\t分类\t来源\n");
    let mut included = 0;
    for ((word, code), fields) in &rows {
        let manual: u32 = fields[2].parse().expect("validated weight");
        let count = text.matches(word).count();
        // 专业语料仅辅助词级排序，不冒充通用语料一元计数，不修改全局语言模型。
        let weight = manual.max(count.min(200) as u32);
        let status = if existing.contains(&(word.clone(), code.clone())) {
            "existing"
        } else {
            "included"
        };
        writeln!(
            audit,
            "{word}\t{code}\t{manual}\t{count}\t{weight}\t{status}\t{}\t{}",
            fields[3], fields[6]
        )
        .expect("String write");
        if status == "included" {
            // 专用前缀使 AI / GAN 等恰好也是拼音的缩写不进入中文词图，避免重复候选。
            let key = if fields[4] == "english" {
                format!("@{code}")
            } else {
                code.clone()
            };
            writeln!(dictionary, "{word}\t{key}\t{weight}").expect("String write");
            included += 1;
        }
    }
    let dict = Dictionary::parse(&dictionary)?;
    if dict.is_empty() {
        return Err(ConvertError::Format {
            path: source.to_owned(),
            line: 0,
            reason: "empty AI dictionary after exclusions".into(),
        });
    }
    let version = rows
        .values()
        .map(|row| row[7].as_str())
        .max()
        .unwrap_or_default()
        .to_owned();
    let dir = out.join("dicts");
    std::fs::create_dir_all(&dir)?;
    glimmer_core::storage::write_atomic_str(&dir.join("ai.tsv"), &dictionary)?;
    dict.write_qj(&dir.join("ai.qj"), &Metadata {
        name: "AI 与人工智能".into(),
        license: "GPL-3.0-or-later AND Apache-2.0 AND CC-BY-SA-4.0".into(),
        attribution: "Glimmer contributors；动手学深度学习作者及中文贡献者；Eugene Siow 与 AI Glossary in Mandarin 贡献者".into(),
        source: "https://github.com/rambocode/glimmer/tree/main/assets/lexicon/ai".into(),
        version,
        ..Metadata::default()
    })?;
    glimmer_core::storage::write_atomic_str(&out.join("ai-audit.tsv"), &audit)?;
    tracing::info!(
        source_rows = rows.len(),
        included,
        excluded = rows.len() - included,
        "AI 领域词库已生成"
    );
    Ok(())
}

fn validate(f: &[&str]) -> Result<(), String> {
    if f.len() != 8 || f.iter().any(|v| v.is_empty() || *v != v.trim()) {
        return Err("expected eight non-empty, trimmed TSV fields".into());
    }
    let weight = f[2].parse::<u32>().map_err(|_| "invalid manual weight")?;
    if !(1..=200).contains(&weight) {
        return Err("manual weight must be in 1..=200".into());
    }
    let code: Vec<&str> = f[1].split(' ').collect();
    let han = |c: char| ('\u{4e00}'..='\u{9fff}').contains(&c);
    match f[4] {
        "term" | "chinese" => {
            if !f[0].chars().all(han)
                || f[0].chars().count() != code.len()
                || code.iter().any(|s| !is_syllable(s))
            {
                return Err(
                    "Chinese word requires one canonical pinyin syllable per character".into(),
                );
            }
        }
        "english" => {
            let compact: String = f[0]
                .bytes()
                .filter(u8::is_ascii_alphanumeric)
                .map(|b| (b as char).to_ascii_lowercase())
                .collect();
            if !f[0].is_ascii()
                || !f[0].bytes().any(|b| b.is_ascii_alphabetic())
                || f[0].bytes().any(|b| b.is_ascii_control())
                || compact != f[1]
            {
                return Err("English code must be the compact lowercase name".into());
            }
        }
        "mixed" => {
            if !f[0].starts_with("AI")
                || f[0].len() == 2
                || !f[0][2..].chars().all(han)
                || code.first() != Some(&"ai")
                || code.len() != f[0][2..].chars().count() + 1
                || code.iter().any(|s| !is_syllable(s))
            {
                return Err(
                    "mixed name requires AI followed by Chinese and ai + canonical pinyin".into(),
                );
            }
        }
        _ => return Err("unknown word kind".into()),
    }
    let date = f[7].as_bytes();
    if date.len() != 10
        || date[4] != b'-'
        || date[7] != b'-'
        || date
            .iter()
            .enumerate()
            .any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
    {
        return Err("checked date must be YYYY-MM-DD".into());
    }
    let year: u32 = f[7][..4].parse().expect("validated digits");
    let month: u32 = f[7][5..7].parse().expect("validated digits");
    let day: u32 = f[7][8..].parse().expect("validated digits");
    let max_day = match month {
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day {
        return Err("invalid calendar date".into());
    }
    Ok(())
}
