//! 从纯文本语料统计词级一元 / 二元 / 三元计数。
//!
//! 短语层（`assets/lexicon/phrases.tsv`，我的 / 不对 这类）不参与分词：它们进了词库，但语言模型里要是当 token 统计，
//! 「而 + 是」的二元证据就没了，二十 会压过 而是。所以分词时把短语从词表里摘掉，统计完再给每条短语**合成**计数：
//! 一元 = 成分二元计数 c(a,b)，前接 c(v,短语) = c(v,a)·c(a,b)/c(a)，后接 c(短语,w) = c(a,b)·c(b,w)/c(b)。
//! 这样 P(短语|v) = P(a|v)·P(b|a)、P(w|短语) = P(w|b)，短语在整句词图和词级排序里的得分与原来走 a / b 两个词的路径一模一样，
//! 只是多了一个能整块选的词。三词短语的一元按 c(a,b)·c(b,c)/c(b) 估。
//!
//! **三元不给短语合成**：三元的合成没有一个能保住上面那条等式的写法（P(w|u,短语) 要的是 c(u,a,b) 这类跨层计数，
//! 而 u 在语料里接的是 a，不是短语），硬估出来的数会让「短语当 token」的路径比走成分词的路径忽高忽低。
//! 所以短语在三元那层没有条目，打分时退回二元——代价是成分词路径可能多拿一份三元的证据，
//! 好处是短语的分仍然等于成分路径的二元分，不会凭空高出来。数字见 `docs/notes/language-model.md`。
//!
//! 分词用微明自己的词库做一元最大概率切分（与输入法词图同一套词表，统计出来的词才能在整句转换里用上）；
//! 只统计连续的汉字段，段与段之间（标点、数字、字母）算句子边界，句首用 `<s>` 标记；空格忽略（预分词语料）。
//! 词库里没有的字跳过，并切断前后的 n-gram 关系。

mod counts;
mod mine;
mod options;
mod phrase;
mod vocabulary;

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use crate::error::ConvertError;

use counts::{Counts, bigram_key, trigram_parts};
use phrase::phrase_components;

pub use mine::{MineOptions, mine};
pub use options::ConvertOptions;
pub(crate) use phrase::{phrase_count, synthesize_phrases};
pub(crate) use vocabulary::Vocabulary;

/// 品牌词次数里几分之一算在句首（请柬 的句首占比约 1/8）。
const BRAND_START_SHARE: u32 = 8;

pub(crate) fn is_han(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}')
}

/// 扫语料，写出 `lm-unigram.tsv` / `lm-bigram.tsv` / `lm-trigram.tsv`。
pub fn convert(options: &ConvertOptions, out_dir: &Path) -> Result<(), ConvertError> {
    let mut vocabulary = Vocabulary::load(&options.dict)?;
    tracing::info!(words = vocabulary.words.len(), "词表加载完成");
    // 短语不参与分词：先摘掉，记下每条的成分，统计完再合成它们的计数
    let mut phrase_parts = Vec::new();
    for path in &options.phrases {
        phrase_parts.extend(phrase_components(path, &mut vocabulary)?);
    }
    let mut counts = Counts::new(vocabulary.words.len(), options.max_trigram_entries);
    let mut tokens = Vec::new();
    let mut lines = 0u64;
    let mut runs = 0u64;
    for path in &options.corpus {
        tracing::info!(path = %path.display(), "统计语料");
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            if lines.is_multiple_of(200_000) {
                tracing::info!(
                    lines,
                    bigrams = counts.bigram.len(),
                    trigrams = counts.trigram.len(),
                    "进度"
                );
                counts.prune_trigrams();
            }
            // LCCC 这类语料是按词用空格分好的；空格不是句子边界，去掉再切
            let line: String = line.chars().filter(|c| *c != ' ').collect();
            for run in line.split(|c: char| !is_han(c)) {
                if run.chars().count() < 2 {
                    continue;
                }
                runs += 1;
                vocabulary.segment(run, &mut tokens);
                counts.record(&tokens);
            }
        }
    }
    tracing::info!(
        lines,
        sentences = runs,
        distinct_bigrams = counts.bigram.len(),
        distinct_trigrams = counts.trigram.len(),
        "统计完成"
    );
    // 品牌词（微明）与中英混杂词（C盘）语料里没有：按文件给的次数写进一元，句首二元给八分之一（请柬 209 次里 25 次在句首，同一比例），
    // 让词级排序不把它当模型不认识的词扣分、能与同音词（请柬）平起平坐，又不压过 请见 这种整句路径
    for path in &options.brand {
        let mut added = 0usize;
        for line in std::fs::read_to_string(path)?.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            if let (Some(text), Some(count)) = (fields.next(), fields.next())
                && let Some(&id) = vocabulary.ids.get(text.trim())
                && let Ok(count) = count.trim().parse::<u32>()
            {
                counts.unigram[id as usize] = u64::from(count);
                counts
                    .bigram
                    .insert(bigram_key(0, id), (count / BRAND_START_SHARE).max(1));
                added += 1;
            }
        }
        tracing::info!(path = %path.display(), words = added, "品牌词一元与句首二元已写入");
    }

    // 二元：按计数降序，砍掉低频与超出上限的；短语的合成行另加，不占真实行的名额
    let mut pairs: Vec<(u64, u32)> = counts
        .bigram
        .iter()
        .map(|(k, c)| (*k, *c))
        .filter(|(_, count)| *count >= options.min_count)
        .collect();
    pairs.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    pairs.truncate(options.max_bigrams);
    if !phrase_parts.is_empty() {
        let synthesized = synthesize_phrases(
            &phrase_parts,
            &mut counts.unigram,
            &counts.bigram,
            options.min_count,
        );
        pairs.extend(synthesized);
    }
    write_bigram(&pairs, &vocabulary, out_dir)?;
    let kept: HashSet<u64> = pairs.iter().map(|(key, _)| *key).collect();
    write_trigram(&counts, &kept, &vocabulary, options, out_dir)?;

    // 一元：只输出出现过的词
    let unigram_path = out_dir.join("lm-unigram.tsv");
    let mut writer = BufWriter::new(File::create(&unigram_path)?);
    writeln!(
        writer,
        "# 由 glimmer-dict-convert bigram 从语料统计。词\\t计数；<s> 是句首标记"
    )?;
    let mut written = 0usize;
    for (id, count) in counts.unigram.iter().enumerate() {
        if *count > 0 {
            writeln!(writer, "{}\t{count}", vocabulary.words[id])?;
            written += 1;
        }
    }
    writer.flush()?;
    tracing::info!(unigram = %unigram_path.display(), words = written, "一元表已写出");
    Ok(())
}

/// 写 `lm-bigram.tsv`。
fn write_bigram(
    pairs: &[(u64, u32)],
    vocabulary: &Vocabulary,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let path = out_dir.join("lm-bigram.tsv");
    let mut writer = BufWriter::new(File::create(&path)?);
    writeln!(
        writer,
        "# 由 glimmer-dict-convert bigram 从语料统计。前词\\t后词\\t计数"
    )?;
    for (key, count) in pairs {
        let first = &vocabulary.words[(key >> 32) as usize];
        let second = &vocabulary.words[(key & 0xFFFF_FFFF) as usize];
        writeln!(writer, "{first}\t{second}\t{count}")?;
    }
    writer.flush()?;
    tracing::info!(path = %path.display(), bigrams = pairs.len(), "二元表已写出");
    Ok(())
}

/// 写 `lm-trigram.tsv`：只留计数够、且上文 (前二词, 前词) 自己也在二元表里的那些。
///
/// 上文不在二元表里的三元在模型里挂不上段（三元 CSR 的段号就是二元的下标），写出来也会被加载时丢掉，
/// 白占几十兆文件；先在这里滤掉。
fn write_trigram(
    counts: &Counts,
    kept_bigrams: &HashSet<u64>,
    vocabulary: &Vocabulary,
    options: &ConvertOptions,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let min_count = options.min_trigram_count.max(counts.threshold() + 1);
    let mut rows: Vec<(u128, u32)> = counts
        .trigram
        .iter()
        .filter(|(_, count)| **count >= min_count)
        .filter(|(key, _)| {
            let (earlier, previous, _) = trigram_parts(**key);
            kept_bigrams.contains(&bigram_key(earlier, previous))
        })
        .map(|(k, c)| (*k, *c))
        .collect();
    rows.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    rows.truncate(options.max_trigrams);
    let path = out_dir.join("lm-trigram.tsv");
    let mut writer = BufWriter::new(File::create(&path)?);
    writeln!(
        writer,
        "# 由 glimmer-dict-convert bigram 从语料统计。前前词\\t前词\\t后词\\t计数"
    )?;
    for (key, count) in &rows {
        let (earlier, previous, word) = trigram_parts(*key);
        writeln!(
            writer,
            "{}\t{}\t{}\t{count}",
            vocabulary.words[earlier as usize],
            vocabulary.words[previous as usize],
            vocabulary.words[word as usize]
        )?;
    }
    writer.flush()?;
    tracing::info!(
        path = %path.display(),
        trigrams = rows.len(),
        min_count,
        "三元表已写出"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
