//! 整句评测：拿用户自己写的中文文本，转成他会敲的全拼，冷启动喂给引擎，看整句转换能不能把原句还原出来。
//!
//! 与 `--replay` 不同，这把尺子不依赖输入日志里「当时选了什么」（那多半是当时的引擎自己的输出），
//! 只看原文；整句排序、语言模型的改动先在同一份句子集上比过再合。
//! 输入既可以是原始文本（一行一段，按标点切句、汉字转拼音），也可以是之前 `--eval-save` 冻结下来的
//! `句子\t拼音\t上文` 三列文件；后者保证不同时间、不同分支比的是同一份句子。
//! 每句独立：不上屏、不学习，只把这句在原文里的上文写进输入历史给整句转换用。

mod bucket;
mod chunk;
mod extract;
mod pair;
mod report;
mod transcribe;
mod typo;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use glimmer_core::Engine;

pub use report::Report;

use bucket::LengthBucket;
use pair::Pair;
use transcribe::Transcriber;

/// 句子集喂给引擎之前怎么变形；都不给就是一句一条、拼音原样。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shape {
    /// 分段输入：每段几个词（见 [`chunk::split`]）；`None` 为整句一条。
    pub chunk_words: Option<usize>,

    /// 每条拼音注一处敲错（见 [`typo::inject`]）。
    pub typos: bool,
}

/// 跑一遍评测集，返回报告；`save` 给了就把用到的句子集（变形之前的）写成三列文件。
pub fn run(
    engine: &mut Engine,
    paths: &[PathBuf],
    save: Option<&Path>,
    shape: Shape,
    show_misses: usize,
) -> Result<Report, EvalError> {
    let mut report = Report::default();
    let pairs = collect(engine, paths, &mut report)?;
    if let Some(path) = save {
        let mut text = String::new();
        for pair in &pairs {
            text.push_str(&pair.to_line());
            text.push('\n');
        }
        std::fs::write(path, text).map_err(|source| EvalError::Write {
            path: path.to_owned(),
            source,
        })?;
        tracing::info!(path = %path.display(), count = pairs.len(), "句子集已保存");
    }
    let pairs = reshape(engine, pairs, shape, &mut report);
    for pair in &pairs {
        evaluate(engine, pair, &mut report, show_misses);
    }
    Ok(report)
}

/// 按 `shape` 把句子集变形：先拆段、再注错（注的是每段自己的拼音）。
fn reshape(engine: &Engine, pairs: Vec<Pair>, shape: Shape, report: &mut Report) -> Vec<Pair> {
    let mut pairs = pairs;
    if let Some(words) = shape.chunk_words {
        let transcriber = transcriber_of(engine);
        pairs = pairs
            .iter()
            .flat_map(|pair| chunk::split(pair, words, &transcriber, engine.language_model()))
            .collect();
        report.mode.push_str(&format!("，分段输入每段 {words} 词"));
    }
    if shape.typos {
        pairs = pairs
            .into_iter()
            .filter_map(|pair| {
                let pinyin = typo::inject(&pair.text, &pair.pinyin)?;
                Some(Pair { pinyin, ..pair })
            })
            .collect();
        report.mode.push_str("，每条注一处敲错");
    }
    pairs
}

/// 从引擎的词库（主词库 + 附加词库）建读音反查表。
fn transcriber_of(engine: &Engine) -> Transcriber {
    let dictionaries = std::iter::once(engine.dictionary()).chain(engine.extra_dictionaries());
    let transcriber = Transcriber::new(dictionaries);
    tracing::info!(words = transcriber.len(), "读音反查表已建");
    transcriber
}

/// 读全部文件，得到去重后的句子集：有制表符的文件按冻结格式读，其余当原始文本抽句、转拼音。
fn collect(
    engine: &Engine,
    paths: &[PathBuf],
    report: &mut Report,
) -> Result<Vec<Pair>, EvalError> {
    let mut transcriber: Option<Transcriber> = None;
    let mut seen: HashSet<String> = HashSet::new();
    let mut pairs = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(path).map_err(|source| EvalError::Read {
            path: path.clone(),
            source,
        })?;
        if text.contains('\t') {
            for line in text.lines() {
                if let Some(pair) = Pair::parse(line)
                    && seen.insert(pair.text.clone())
                {
                    pairs.push(pair);
                }
            }
            continue;
        }
        let transcriber = transcriber.get_or_insert_with(|| transcriber_of(engine));
        for extracted in extract::extract(&text) {
            if !seen.insert(extracted.text.clone()) {
                continue;
            }
            report.extracted += 1;
            match transcriber.transcribe(&extracted.text, engine.language_model()) {
                Some(pinyin) => pairs.push(Pair {
                    text: extracted.text,
                    pinyin,
                    context: extracted.context,
                }),
                None => report.untranscribable += 1,
            }
        }
    }
    Ok(pairs)
}

/// 评一句：清空引擎状态、写入上文、喂拼音、看候选。
fn evaluate(engine: &mut Engine, pair: &Pair, report: &mut Report, show_misses: usize) {
    report.total += 1;
    engine.clear();
    engine.break_chain();
    engine.history_mut().clear();
    engine.history_mut().record(&pair.context);
    // 上文也摆进上屏链：壳里这段拼音之前上屏的词就在链上，整句的第一个词接着它算
    engine.seed_chain(&pair.context);
    engine.set_input(&pair.pinyin);
    let started = Instant::now();
    let query = match engine.query() {
        Ok(query) => query,
        Err(_) => {
            report.unparsable += 1;
            engine.clear();
            return;
        }
    };
    // 异步重打分：像壳一样停顿后请求、等结果、再查一次；等的时间也算进查询耗时
    let query = crate::rescoring::settled(engine, query);
    let elapsed = started.elapsed();
    report.query_time += elapsed;
    report.slowest_query = report.slowest_query.max(elapsed);
    let items = &query.candidates.items;
    let position = items.iter().position(|c| c.text == pair.text);
    if position == Some(0) {
        report.top1 += 1;
    }
    // 第一个盖住全部拼音的候选就是整句转换的答案（整句本身是个词时也可能是词库词）
    let length = pair.text.chars().count();
    let sentence = items.iter().find(|c| c.text.chars().count() == length);
    report.chars_total += length;
    let bucket = &mut report.buckets[LengthBucket::index_of(length)];
    bucket.total += 1;
    bucket.chars_total += length;
    if position == Some(0) {
        bucket.top1 += 1;
    }
    if let Some(sentence) = sentence {
        if sentence.text == pair.text {
            report.sentence_hit += 1;
        }
        let correct = sentence
            .text
            .chars()
            .zip(pair.text.chars())
            .filter(|(a, b)| a == b)
            .count();
        report.chars_correct += correct;
        report.buckets[LengthBucket::index_of(length)].chars_correct += correct;
    }
    if position != Some(0) && report.misses.len() < show_misses {
        let top: Vec<&str> = items.iter().take(3).map(|c| c.text.as_str()).collect();
        report.misses.push(format!(
            "{:<20} {:<28} 现在前三 {}{}",
            pair.text,
            pair.pinyin,
            top.join(" / "),
            position.map_or(String::from("（不在候选里）"), |i| format!(
                "（第 {} 位）",
                i + 1
            )),
        ));
    }
    engine.clear();
}

/// 整句评测的错误。
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("cannot read evaluation text {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot write sentence set {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
