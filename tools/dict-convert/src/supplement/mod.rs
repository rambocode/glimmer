//! 补充常用词：不重扫语料，在已有的基础词库与语言模型上找缺口、补词。
//!
//! 常用词表是词典词头，`mine` 只挖连续单字段、`phrases` 只收 ≥ 2000 次的组合，于是 取餐 / 弹出 / 这时 / 播放器 这类
//! 「一个成分是高频虚字或已成词」的常用词两头都漏了（`qucan` 出 午餐、`bofangqi` 出整句 播放其）。
//!
//! - `gaps`：拿许可清楚的外部词表（每行 `词[\t拼音]`）当白名单，筛出词库（含 `dicts/` 领域词库）与语言模型都没有的词，
//!   按 `lm.qj` 里成分词的二元计数合成它在语料里的次数（与 `bigram --phrases` 同一公式），写 `gap-candidates.tsv` 供人工挑。
//! - `supplement`：把人工挑好的词表（`词\t次数\t拼音`，与 `domain_words.tsv` 同格式）并进词库 TSV，并在 `lm.qj` 上合成它们的一元 / 二元，
//!   写回 `dict.tsv` 与 `lm-unigram.tsv` / `lm-bigram.tsv`，再 `pack` 成 `.qj`。结果与全量重跑
//!   `lexicon --extra-words` + `bigram --phrases` 一致（语言模型差在原表砍掉的低频二元），重复跑是幂等的：词库与模型里已有的词跳过。

mod counts;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use glimmer_core::parser::is_syllable;
use glimmer_dictionary::canonical_syllable;

use crate::bigram::{Vocabulary, is_han, phrase_count, synthesize_phrases};
use crate::error::ConvertError;
use crate::lexicon::corpus::extra_words;
use crate::phrases::load_readings;
use counts::LmCounts;

/// `gaps` 子命令的参数。
pub struct GapOptions {
    /// 候选词表（`词[\t拼音]`）。
    pub candidates: Vec<PathBuf>,

    /// 基础词库 TSV（同目录 `dicts/` 一并当已有词）。
    pub dict: PathBuf,

    /// 语言模型 `.qj`。
    pub lm: PathBuf,

    /// 合成次数下限。
    pub min_count: u32,

    /// 最多几个字。
    pub max_chars: usize,
}

/// 一条缺口候选。
struct Gap {
    text: String,
    count: u32,
    pinyin: String,
    components: String,
    reading_source: &'static str,
}

/// 找缺口：候选词表里词库与语言模型都没有、合成次数够的词，写 `gap-candidates.tsv`（`词\t次数\t拼音\t成分\t读音来源`，按次数降序）。
pub fn gaps(options: &GapOptions, out_dir: &Path) -> Result<(), ConvertError> {
    let vocabulary = Vocabulary::load(&options.dict)?;
    let readings = load_readings(&options.dict)?;
    let lm = LmCounts::load(&options.lm)?;
    let candidates = read_candidates(&options.candidates)?;
    tracing::info!(candidates = candidates.len(), "候选词表已读取");
    let mut rows = Vec::new();
    for (text, given) in &candidates {
        let chars = text.chars().count();
        // 已有的词（词库、领域词库、语言模型 token）不算缺口
        if !(2..=options.max_chars).contains(&chars)
            || !text.chars().all(is_han)
            || vocabulary.ids.contains_key(text)
            || lm.ids.contains_key(text)
        {
            continue;
        }
        let Some(parts) = lm.components(text, &vocabulary) else {
            continue;
        };
        let count = phrase_count(&parts, &lm.unigram, &lm.bigram);
        if count < f64::from(options.min_count) {
            continue;
        }
        let names: Vec<&str> = parts
            .iter()
            .map(|&id| lm.words[id as usize].as_str())
            .collect();
        // 读音：词表给了就用（CC-CEDICT 标了多音字），否则由成分词的主读音拼出，人工挑时重点看后者
        let (pinyin, reading_source) = match given {
            Some(pinyin) if valid_pinyin(text, pinyin) => (pinyin.clone(), "given"),
            _ => {
                let composed: Option<Vec<&str>> = names
                    .iter()
                    .map(|n| readings.get(*n).map(String::as_str))
                    .collect();
                match composed.map(|c| c.join(" ")) {
                    Some(pinyin) if valid_pinyin(text, &pinyin) => (pinyin, "composed"),
                    _ => continue,
                }
            }
        };
        rows.push(Gap {
            text: text.clone(),
            count: count.round() as u32,
            pinyin,
            components: names.join("+"),
            reading_source,
        });
    }
    rows.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.text.cmp(&b.text)));
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join("gap-candidates.tsv");
    let mut writer = BufWriter::new(File::create(&path)?);
    writeln!(
        writer,
        "# 由 glimmer-dict-convert gaps 写出：候选词表里词库与语言模型都没有的词，次数按成分二元合成。词\\t次数\\t拼音\\t成分\\t读音来源（given 词表给的 / composed 成分拼的）"
    )?;
    for row in &rows {
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}",
            row.text, row.count, row.pinyin, row.components, row.reading_source
        )?;
    }
    writer.flush()?;
    tracing::info!(rows = rows.len(), path = %path.display(), "缺口候选已写出");
    Ok(())
}

/// `supplement` 子命令的参数。
pub struct SupplementOptions {
    /// 人工挑好的补充词表（`词\t次数\t拼音`）。
    pub words: Vec<PathBuf>,

    /// 基础词库 TSV。
    pub dict: PathBuf,

    /// 语言模型 `.qj`。
    pub lm: PathBuf,

    /// 合成计数的下限，与 `bigram --min-count` 一致。
    pub min_count: u32,
}

/// 补词：写 `dict.tsv`（基础词库 + 补充词）与 `lm-unigram.tsv` / `lm-bigram.tsv`（原模型 + 补充词的合成计数）。
pub fn supplement(options: &SupplementOptions, out_dir: &Path) -> Result<(), ConvertError> {
    let mut words = Vec::new();
    for path in &options.words {
        words.extend(extra_words(path)?.into_iter().map(|word| (path, word)));
    }
    let mut vocabulary = Vocabulary::load(&options.dict)?;
    std::fs::create_dir_all(out_dir)?;

    // 词库：基础词库逐行保留原顺序（品牌词、中英混杂词是手工插的，不是严格有序），补充词里没有的按给定读音与次数
    // 二分插到 (词, 音节) 该在的位置，与 lexicon 的排序一致，diff 只多出新增的行
    let (header, mut entries) = read_dict(&options.dict)?;
    let mut added = 0usize;
    let mut existing = 0usize;
    for (path, word) in &words {
        if vocabulary.ids.contains_key(&word.text) {
            existing += 1;
            continue;
        }
        let Some(syllables) = word
            .syllables
            .as_ref()
            .filter(|s| valid_pinyin(&word.text, &s.join(" ")))
        else {
            return Err(ConvertError::Format {
                path: (*path).clone(),
                line: 0,
                reason: format!("{} 的读音缺失或与字数不符", word.text),
            });
        };
        let key = (word.text.clone(), syllables.clone());
        let position = entries.partition_point(|(k, _)| *k < key);
        if entries.get(position).is_some_and(|(k, _)| *k == key) {
            existing += 1;
            continue;
        }
        let frequency = word.count.clamp(1, u64::from(u32::MAX)) as u32;
        entries.insert(position, (key, frequency));
        added += 1;
    }
    let dict_path = out_dir.join("dict.tsv");
    let mut writer = BufWriter::new(File::create(&dict_path)?);
    for line in &header {
        writeln!(writer, "{line}")?;
    }
    for ((text, syllables), frequency) in &entries {
        writeln!(writer, "{text}\t{}\t{frequency}", syllables.join(" "))?;
    }
    writer.flush()?;
    tracing::info!(added, existing, entries = entries.len(), path = %dict_path.display(), "词库已写出");

    // 语言模型：补充词从分词词表摘掉，按成分合成一元与前后接二元（模型里已有的跳过，重复跑不会叠加）
    for (_, word) in &words {
        vocabulary.ids.remove(&word.text);
    }
    let mut lm = LmCounts::load(&options.lm)?;
    let mut parts = Vec::new();
    let mut skipped = 0usize;
    for (_, word) in &words {
        if lm.ids.contains_key(&word.text) {
            continue;
        }
        match lm.components(&word.text, &vocabulary) {
            Some(components) => {
                let id = lm.push_word(&word.text);
                parts.push((id, components));
            }
            None => skipped += 1,
        }
    }
    tracing::info!(words = parts.len(), skipped, "补充词的成分已切好");
    let rows = synthesize_phrases(&parts, &mut lm.unigram, &lm.bigram, options.min_count);
    lm.write_tsv(&rows, out_dir)
}

/// 读候选词表：每行第一列是词，第二列（可选）是空格分隔的拼音；同一个词多行时优先留给了读音的那行。
fn read_candidates(paths: &[PathBuf]) -> Result<BTreeMap<String, Option<String>>, ConvertError> {
    let mut candidates: BTreeMap<String, Option<String>> = BTreeMap::new();
    for path in paths {
        for line in std::fs::read_to_string(path)?.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let Some(text) = fields.next().map(str::trim).filter(|t| !t.is_empty()) else {
                continue;
            };
            let pinyin = fields
                .next()
                .map(|p| {
                    p.split_whitespace()
                        .map(canonical_syllable)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .filter(|p| !p.is_empty());
            let slot = candidates.entry(text.to_owned()).or_default();
            if slot.is_none() {
                *slot = pinyin;
            }
        }
    }
    Ok(candidates)
}

/// 读音每个音节都合法、音节数等于字数。
fn valid_pinyin(text: &str, pinyin: &str) -> bool {
    let syllables: Vec<&str> = pinyin.split(' ').collect();
    syllables.len() == text.chars().count() && syllables.iter().all(|s| is_syllable(s))
}

/// 基础词库的一行：(词, 音节) 与词频。
type DictEntry = ((String, Vec<String>), u32);

/// 读基础词库：`#` 注释行原样保留，词条保持文件顺序。
fn read_dict(path: &Path) -> Result<(Vec<String>, Vec<DictEntry>), ConvertError> {
    let mut header = Vec::new();
    let mut entries = Vec::new();
    for (index, line) in std::fs::read_to_string(path)?.lines().enumerate() {
        if line.starts_with('#') {
            header.push(line.to_owned());
            continue;
        }
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(text), Some(pinyin), Some(frequency)) =
            (fields.next(), fields.next(), fields.next())
        else {
            return Err(ConvertError::Format {
                path: path.to_path_buf(),
                line: index + 1,
                reason: "expected word<TAB>syllables<TAB>frequency".to_owned(),
            });
        };
        let frequency: u32 = frequency.trim().parse().map_err(|_| ConvertError::Format {
            path: path.to_path_buf(),
            line: index + 1,
            reason: "frequency is not a number".to_owned(),
        })?;
        let syllables = pinyin.split(' ').map(str::to_owned).collect();
        entries.push(((text.to_owned(), syllables), frequency));
    }
    Ok((header, entries))
}

#[cfg(test)]
mod tests;
