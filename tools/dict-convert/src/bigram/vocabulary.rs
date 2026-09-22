//! 分词用的词表：从微明词库读词与词频，按一元最大概率把一段汉字切成词。

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::error::ConvertError;

/// 句首标记。
const SENTENCE_START: &str = "<s>";

/// 分词时一个词最多几个汉字。
const MAX_WORD_CHARS: usize = 8;

/// 词库里没有的单字的 log 概率（相对于词频 1 的词再扣这么多）。
const UNKNOWN_PENALTY: f64 = -12.0;

/// 分词用的词表：词 → 编号与 log 词频。
pub(crate) struct Vocabulary {
    /// 词 → 编号。
    pub(crate) ids: HashMap<String, u32>,

    /// 编号 → 词。
    pub(crate) words: Vec<String>,

    /// 编号 → log 概率：log(词频 + 1) − log(总词频)。不减总频的话多字词会输给它的单字。
    log_frequency: Vec<f64>,

    /// 未知单字的 log 概率。
    unknown: f64,
}

impl Vocabulary {
    /// 词表文件：`path` 加上同目录 `dicts/` 下的领域词库（拆分后基础词库不含领域词，分词仍要用全部词）。
    pub(crate) fn files(path: &Path) -> Vec<PathBuf> {
        let mut files = vec![path.to_path_buf()];
        if let Some(dir) = path.parent().map(|p| p.join("dicts"))
            && let Ok(entries) = std::fs::read_dir(&dir)
        {
            let mut extra: Vec<_> = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "tsv"))
                .collect();
            extra.sort();
            tracing::info!(dir = %dir.display(), files = extra.len(), "分词也用领域词库");
            files.extend(extra);
        }
        files
    }

    /// 读分词词表（见 [`Self::files`]）。
    pub(crate) fn load(path: &Path) -> Result<Self, ConvertError> {
        let files = Self::files(path);
        let mut total = 0.0_f64;
        let mut ids: HashMap<String, u32> = HashMap::new();
        let mut words = vec![SENTENCE_START.to_owned()];
        let mut log_frequency = vec![0.0];
        ids.insert(SENTENCE_START.to_owned(), 0);
        for line in files
            .iter()
            .map(|file| File::open(file).map(BufReader::new))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(BufRead::lines)
        {
            let line = line?;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(text), Some(_), Some(frequency)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let frequency: f64 = frequency.trim().parse().unwrap_or(0.0);
            total += frequency;
            let log = (frequency + 1.0).ln();
            match ids.get(text) {
                // 同一个词多个读音：取最高词频
                Some(&id) => {
                    if log > log_frequency[id as usize] {
                        log_frequency[id as usize] = log;
                    }
                }
                None => {
                    ids.insert(text.to_owned(), words.len() as u32);
                    words.push(text.to_owned());
                    log_frequency.push(log);
                }
            }
        }
        let log_total = total.max(1.0).ln();
        for log in &mut log_frequency {
            *log -= log_total;
        }
        Ok(Self {
            ids,
            words,
            log_frequency,
            unknown: UNKNOWN_PENALTY - log_total,
        })
    }

    /// 一段连续汉字按最大概率切成词编号；词库里没有的字用 `None` 占位。
    pub(crate) fn segment(&self, run: &str, output: &mut Vec<Option<u32>>) {
        output.clear();
        let offsets: Vec<usize> = run
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(run.len()))
            .collect();
        let n = offsets.len() - 1;
        let mut best = vec![f64::NEG_INFINITY; n + 1];
        let mut back: Vec<(usize, Option<u32>)> = vec![(0, None); n + 1];
        best[0] = 0.0;
        for start in 0..n {
            if best[start] == f64::NEG_INFINITY {
                continue;
            }
            let mut any = false;
            for end in start + 1..=n.min(start + MAX_WORD_CHARS) {
                let slice = &run[offsets[start]..offsets[end]];
                if let Some(&id) = self.ids.get(slice) {
                    any = true;
                    let score = best[start] + self.log_frequency[id as usize];
                    if score > best[end] {
                        best[end] = score;
                        back[end] = (start, Some(id));
                    }
                }
            }
            if !any {
                let score = best[start] + self.unknown;
                if score > best[start + 1] {
                    best[start + 1] = score;
                    back[start + 1] = (start, None);
                }
            }
        }
        let mut position = n;
        while position > 0 {
            let (start, id) = back[position];
            output.push(id);
            position = start;
        }
        output.reverse();
    }
}
