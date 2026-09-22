//! 统计的小样例：临时目录里放词库与语料，跑完读回三张表。

use std::path::PathBuf;

use super::{ConvertOptions, convert};

const DICT: &str =
    "我\two\t100\n想\txiang\t100\n去\tqu\t100\n吃饭\tchi fan\t100\n吃\tchi\t10\n饭\tfan\t10\n";

/// 读回一张表的正文行（去掉 `#` 开头的表头）。
fn rows(dir: &std::path::Path, name: &str) -> Vec<String> {
    std::fs::read_to_string(dir.join(name))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

#[test]
fn counts_unigrams_bigrams_and_trigrams_of_a_tiny_corpus() {
    let dir: PathBuf = std::env::temp_dir().join(format!("glimmer-bigram-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("dict.tsv"), DICT).unwrap();
    std::fs::write(dir.join("corpus.txt"), "我想去吃饭。\n".repeat(10)).unwrap();
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    convert(
        &ConvertOptions {
            corpus: vec![dir.join("corpus.txt")],
            dict: dir.join("dict.tsv"),
            phrases: Vec::new(),
            brand: Vec::new(),
            min_count: 1,
            max_bigrams: 100,
            min_trigram_count: 1,
            max_trigrams: 100,
            max_trigram_entries: 1_000,
        },
        &out,
    )
    .unwrap();
    let unigram = rows(&out, "lm-unigram.tsv");
    let bigram = rows(&out, "lm-bigram.tsv");
    let mut trigram = rows(&out, "lm-trigram.tsv");
    trigram.sort();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(unigram.contains(&"<s>\t10".to_owned()));
    assert!(unigram.contains(&"吃饭\t10".to_owned()));
    assert!(bigram.contains(&"<s>\t我\t10".to_owned()));
    assert!(bigram.contains(&"去\t吃饭\t10".to_owned()));
    // 句首词没有前二词，所以三元从第二个词起：(<s>, 我, 想) 是第一条
    assert_eq!(
        trigram,
        ["<s>\t我\t想\t10", "想\t去\t吃饭\t10", "我\t想\t去\t10"]
    );
}

#[test]
fn synthesizes_dict_words_the_segmenter_never_picks() {
    // 一行 词频是底值，语料里总被切成 一 / 行，一元表里本来没有它；合成后 = 成分二元 c(一, 行)
    let dict = "一\tyi\t1000\n行\txing\t1000\n字\tzi\t1000\n一行\tyi hang\t1\n";
    let dir: PathBuf =
        std::env::temp_dir().join(format!("glimmer-bigram-unseen-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("dict.tsv"), dict).unwrap();
    std::fs::write(dir.join("corpus.txt"), "一行字\n".repeat(5)).unwrap();
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    convert(
        &ConvertOptions {
            corpus: vec![dir.join("corpus.txt")],
            dict: dir.join("dict.tsv"),
            phrases: Vec::new(),
            brand: Vec::new(),
            min_count: 1,
            max_bigrams: 100,
            min_trigram_count: 1,
            max_trigrams: 100,
            max_trigram_entries: 1_000,
        },
        &out,
    )
    .unwrap();
    let unigram = rows(&out, "lm-unigram.tsv");
    let bigram = rows(&out, "lm-bigram.tsv");
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(unigram.contains(&"一\t5".to_owned()));
    assert!(unigram.contains(&"一行\t5".to_owned()));
    // 后接：c(一行, 字) = c(一, 行)·c(行, 字)/c(行)
    assert!(bigram.contains(&"一行\t字\t5".to_owned()));
}
