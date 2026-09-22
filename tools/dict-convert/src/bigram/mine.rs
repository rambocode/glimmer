//! 挖词库没收的词：分词时连续落成单字的那一段多半是一个漏收的词。

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::ConvertError;
use crate::oov_filter::OovFilter;

use super::{Vocabulary, is_han};

/// `mine` 的参数。
pub struct MineOptions {
    /// 语料文件。
    pub corpus: Vec<PathBuf>,

    /// 分词用的词库。
    pub dict: PathBuf,

    /// 次数下限。
    pub min_count: u32,

    /// 最多几个字。
    pub max_chars: usize,

    /// 一元表（算 PMI 用）。
    pub frequency: PathBuf,

    /// PMI 下限，0 不过滤。
    pub min_pmi: f64,

    /// 只过滤这份现成的候选文件，不扫语料。
    pub candidates: Option<PathBuf>,
}

/// 挖词库里没有的词：分词时连续落成单字的那一段（2–`max_chars` 个字）多半是一个词库没收的词，
/// 按出现次数统计，出现 `min_count` 次以上的写到 `oov-candidates.tsv`（`词\t次数`），
/// 再经 [`OovFilter`]（虚词规则 + 相邻字对 PMI）筛成 `oov-filtered.tsv` 与一行一词的 `oov-words.txt`，
/// 后者交给 `gloss-gen pinyin` 标音、前者给 `lexicon --extra-words` 并进词库。
/// 单字词本身在词库里（规范字全收了），所以这里只看「本可以成词却被拆成单字」的连续段：
/// 段内每个字都是词库里的单字词、且整段不在词库里。
pub fn mine(options: &MineOptions, out_dir: &Path) -> Result<(), ConvertError> {
    std::fs::create_dir_all(out_dir)?;
    let counts = match &options.candidates {
        Some(path) => read_candidates(path)?,
        None => {
            let counts = scan_single_runs(options)?;
            write_counts(
                &out_dir.join("oov-candidates.tsv"),
                "# 由 glimmer-dict-convert mine 从语料挖出的词库未收词（未过滤）。词\t次数",
                &sorted_rows(&counts, options.min_count),
            )?;
            counts
        }
    };
    let filter = OovFilter::from_unigram_file(&options.frequency, options.min_pmi)?;
    let kept: Vec<(String, u32)> = sorted_rows(&counts, options.min_count)
        .into_iter()
        .filter(|(word, _)| {
            if options.min_pmi <= 0.0 {
                OovFilter::passes_function_rules(word)
            } else {
                filter.keeps(word, &counts)
            }
        })
        .collect();
    write_counts(
        &out_dir.join("oov-filtered.tsv"),
        &format!(
            "# mine 结果经虚词规则 + 相邻字对 PMI≥{} 过滤。词\t次数",
            options.min_pmi
        ),
        &kept,
    )?;
    let words_path = out_dir.join("oov-words.txt");
    let mut writer = BufWriter::new(File::create(&words_path)?);
    for (word, _) in &kept {
        writeln!(writer, "{word}")?;
    }
    writer.flush()?;
    tracing::info!(
        candidates = counts.values().filter(|c| **c >= options.min_count).count(),
        kept = kept.len(),
        path = %words_path.display(),
        "挖词过滤完成"
    );
    Ok(())
}

/// 读上一次写出的候选文件（`词\t次数`，`#` 注释）。
fn read_candidates(path: &Path) -> Result<HashMap<String, u32>, ConvertError> {
    let mut counts = HashMap::new();
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        if line.starts_with('#') {
            continue;
        }
        if let Some((word, count)) = line.split_once('\t')
            && let Ok(count) = count.parse::<u32>()
        {
            counts.insert(word.to_owned(), count);
        }
    }
    Ok(counts)
}

/// 次数不低于 `min_count` 的候选，按次数降序、同次数按词排。
fn sorted_rows(counts: &HashMap<String, u32>, min_count: u32) -> Vec<(String, u32)> {
    let mut rows: Vec<(String, u32)> = counts
        .iter()
        .filter(|(_, c)| **c >= min_count)
        .map(|(w, c)| (w.clone(), *c))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    rows
}

fn write_counts(path: &Path, header: &str, rows: &[(String, u32)]) -> Result<(), ConvertError> {
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(writer, "{header}")?;
    for (word, count) in rows {
        writeln!(writer, "{word}\t{count}")?;
    }
    writer.flush()?;
    tracing::info!(rows = rows.len(), path = %path.display(), "已写出");
    Ok(())
}

/// 扫语料，数连续单字段里 2–`max_chars` 字的子串。
fn scan_single_runs(options: &MineOptions) -> Result<HashMap<String, u32>, ConvertError> {
    let MineOptions {
        corpus,
        dict,
        max_chars,
        ..
    } = options;
    let max_chars = *max_chars;
    let vocabulary = Vocabulary::load(dict)?;
    tracing::info!(words = vocabulary.words.len(), "词表加载完成");
    let single_ids: std::collections::HashSet<u32> = vocabulary
        .words
        .iter()
        .enumerate()
        .filter(|(_, w)| w.chars().count() == 1)
        .map(|(i, _)| i as u32)
        .collect();
    let mut counts: HashMap<String, u32> = HashMap::new();
    let mut tokens = Vec::new();
    let mut lines = 0u64;
    for path in corpus {
        tracing::info!(path = %path.display(), "扫描语料");
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            if lines.is_multiple_of(500_000) {
                tracing::info!(lines, candidates = counts.len(), "进度");
                // 内存兜底：候选太多先把只出现一次的扔掉
                if counts.len() > 20_000_000 {
                    counts.retain(|_, c| *c > 1);
                }
            }
            let line: String = line.chars().filter(|c| *c != ' ').collect();
            for run in line.split(|c: char| !is_han(c)) {
                if run.chars().count() < 2 {
                    continue;
                }
                vocabulary.segment(run, &mut tokens);
                // 连续单字段：token 是单字词编号的一串
                let mut start = 0usize;
                let chars: Vec<char> = run.chars().collect();
                let mut position = 0usize;
                let mut singles: Vec<usize> = Vec::new();
                for token in &tokens {
                    let width = match token {
                        Some(id) => vocabulary.words[*id as usize].chars().count(),
                        None => 1,
                    };
                    let is_single = token.is_some_and(|id| single_ids.contains(&id));
                    if is_single {
                        if singles.is_empty() {
                            start = position;
                        }
                        singles.push(position);
                    } else if !singles.is_empty() {
                        collect_runs(&chars, start, position, max_chars, &mut counts);
                        singles.clear();
                    }
                    position += width;
                }
                if !singles.is_empty() {
                    collect_runs(&chars, start, position, max_chars, &mut counts);
                }
            }
        }
    }
    tracing::info!(lines, candidates = counts.len(), "扫描完成");
    Ok(counts)
}

/// 一段连续单字 `chars[start..end]` 里所有 2–`max_chars` 字的子串各计一次。
fn collect_runs(
    chars: &[char],
    start: usize,
    end: usize,
    max_chars: usize,
    counts: &mut HashMap<String, u32>,
) {
    let len = end - start;
    if len < 2 {
        return;
    }
    for width in 2..=max_chars.min(len) {
        for from in start..=end - width {
            let word: String = chars[from..from + width].iter().collect();
            *counts.entry(word).or_insert(0) += 1;
        }
    }
}
