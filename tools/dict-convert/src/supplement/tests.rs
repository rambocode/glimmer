//! `gaps` / `supplement` 的端到端小样例：临时目录里放词库、语言模型，跑完读回输出。

use std::path::{Path, PathBuf};

use glimmer_format::Metadata;
use glimmer_lm::NgramModel;

use super::{GapOptions, SupplementOptions, gaps, read_dict, supplement};

/// 词库：取 / 餐 / 午餐 / 播放 / 器，取餐 与 播放器 不在里面。
const DICT: &str =
    "# 测试词库\n午餐\twu can\t50\n取\tqu\t40\n器\tqi\t30\n播放\tbo fang\t60\n餐\tcan\t20\n";

/// 语言模型：取 餐 共现 30 次、播放 器 12 次，其余是前后接。
const UNIGRAM: &str = "<s>\t100\n取\t40\n餐\t20\n午餐\t50\n播放\t60\n器\t30\n去\t10\n";
const BIGRAM: &str = "取\t餐\t30\n播放\t器\t12\n去\t取\t8\n餐\t<s>\t5\n器\t去\t6\n";

/// 三元：补词不碰它，只验它原样写回。
const TRIGRAM: &str = "去\t取\t餐\t7\n";

/// 在临时目录里写好词库与 `lm.qj`，返回目录。
fn fixture(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("glimmer-supplement-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("dict.tsv"), DICT).unwrap();
    let model = NgramModel::parse(UNIGRAM, BIGRAM, TRIGRAM).unwrap();
    model
        .write_qj(&dir.join("lm.qj"), &Metadata::default())
        .unwrap();
    dir
}

/// 读回 `lm-unigram.tsv` 里某个词的次数。
fn unigram_count(dir: &Path, word: &str) -> Option<u32> {
    std::fs::read_to_string(dir.join("lm-unigram.tsv"))
        .unwrap()
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .find(|(w, _)| *w == word)
        .and_then(|(_, c)| c.parse().ok())
}

#[test]
fn gaps_lists_missing_words_with_synthesized_counts() {
    let dir = fixture("gaps");
    let candidates = dir.join("candidates.tsv");
    // 午餐 已在词库；取餐 给了读音；播放器 没给读音，由成分拼出；取去 次数不够
    std::fs::write(&candidates, "午餐\twu can\n取餐\tqu can\n播放器\n取去\n").unwrap();
    let out = dir.join("out");
    gaps(
        &GapOptions {
            candidates: vec![candidates],
            dict: dir.join("dict.tsv"),
            lm: dir.join("lm.qj"),
            min_count: 10,
            max_chars: 4,
        },
        &out,
    )
    .unwrap();
    let written = std::fs::read_to_string(out.join("gap-candidates.tsv")).unwrap();
    let rows: Vec<&str> = written.lines().skip(1).collect();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        rows,
        [
            "取餐\t30\tqu can\t取+餐\tgiven",
            "播放器\t12\tbo fang qi\t播放+器\tcomposed"
        ]
    );
}

#[test]
fn supplement_adds_words_to_dict_and_model_and_is_idempotent() {
    let dir = fixture("supplement");
    let words = dir.join("words.tsv");
    std::fs::write(
        &words,
        "# 补充词\n取餐\t30\tqu can\n播放器\t12\tbo fang qi\n",
    )
    .unwrap();
    let out = dir.join("out");
    let options = SupplementOptions {
        words: vec![words.clone()],
        dict: dir.join("dict.tsv"),
        lm: dir.join("lm.qj"),
        min_count: 3,
    };
    supplement(&options, &out).unwrap();
    let (header, entries) = read_dict(&out.join("dict.tsv")).unwrap();
    assert_eq!(header, ["# 测试词库"]);
    let texts: Vec<&str> = entries.iter().map(|((text, _), _)| text.as_str()).collect();
    // 原有行顺序不动（DICT 本身不是有序的），新词二分插进去
    assert_eq!(texts, ["午餐", "取", "取餐", "器", "播放", "播放器", "餐"]);
    assert!(entries.contains(&(
        ("取餐".to_owned(), vec!["qu".to_owned(), "can".to_owned()]),
        30
    )));
    // 合成计数：一元 = c(取,餐)；前接 c(去,取餐) = c(去,取)·c(取,餐)/c(取) = 8·30/40 = 6
    assert_eq!(unigram_count(&out, "取餐"), Some(30));
    assert_eq!(unigram_count(&out, "播放器"), Some(12));
    let bigram = std::fs::read_to_string(out.join("lm-bigram.tsv")).unwrap();
    assert!(bigram.lines().any(|l| l == "去\t取餐\t6"));

    // 在补过的词库与模型上再跑一次，输出不变
    // 三元原样写回，补词不动它
    let trigram = std::fs::read_to_string(out.join("lm-trigram.tsv")).unwrap();
    assert!(trigram.lines().any(|l| l == "去\t取\t餐\t7"));
    let model = NgramModel::from_paths(
        &out.join("lm-unigram.tsv"),
        &out.join("lm-bigram.tsv"),
        Some(&out.join("lm-trigram.tsv")),
    )
    .unwrap();
    model
        .write_qj(&out.join("lm.qj"), &Metadata::default())
        .unwrap();
    let again = dir.join("again");
    supplement(
        &SupplementOptions {
            words: vec![words],
            dict: out.join("dict.tsv"),
            lm: out.join("lm.qj"),
            min_count: 3,
        },
        &again,
    )
    .unwrap();
    let first_dict = std::fs::read_to_string(out.join("dict.tsv")).unwrap();
    let second_dict = std::fs::read_to_string(again.join("dict.tsv")).unwrap();
    let second_unigram = unigram_count(&again, "取餐");
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(first_dict, second_dict);
    assert_eq!(second_unigram, Some(30));
}
