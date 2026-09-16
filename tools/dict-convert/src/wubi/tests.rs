//! `wubi` 子命令的测试：头部、表体、过滤、合并、stem 核对，以及写出的 TSV 能进词库、打成 `.qj` 再查。

use std::path::PathBuf;

use glimmer_dictionary::{Dictionary, SyllablePattern};
use glimmer_format::Metadata;

use super::charset::{Charset, is_gb2312};
use super::full_codes::FullCodes;
use super::table::Table;
use super::{convert, select};

/// 十几行的小样本（编码与真实码表一致：王 gggg、一 ggll、工 a/aaaa）：头部四列、注释、一级简码带 stem、重复行、扩展区字、生僻字、符号 z 码、非法码。
const SAMPLE: &str = "\
# Rime dictionary: sample
# encoding: utf-8

---
name: wubi86
version: \"0.7\"
sort: by_weight
columns:
  - text
  - code
  - weight
  - stem
encoder:
  exclude_patterns:
    - '^z.*$'
  rules:
    - length_equal: 2
      formula: \"AaAbBaBb\"
    - length_in_range: [4, 10]
      formula: \"AaBaCaZa\"
...
工\ta\t99454797\taa
工\taaaa\t551000000
王\tg\t2015124792\tgg
王\tgggg\t827000000
王\tgggg\t1
一\tggll\t100
人\tw\t999\tww
人\twwww\t5
工人\tawww\t300
𬳶\tcmkg
龘\tkkkk\t1
→\tzzj\t1
了\tbnh\t10
了\tb\t1477\tbx
#子\tb\t1
bad\tA1\t1
";

/// 测试用临时目录，用完删掉。
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "glimmer-dict-convert-wubi-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn header_fields() {
    let table = Table::parse(SAMPLE);
    assert_eq!(table.header.name.as_deref(), Some("wubi86"));
    assert_eq!(table.header.version.as_deref(), Some("0.7"));
    assert_eq!(table.header.columns, ["text", "code", "weight", "stem"]);
    assert_eq!(table.header.rules.len(), 2);
    assert_eq!(table.header.rules[0].formula, "AaAbBaBb");
    assert_eq!(
        (
            table.header.rules[0].min_length,
            table.header.rules[0].max_length
        ),
        (2, 2)
    );
    assert_eq!(
        (
            table.header.rules[1].min_length,
            table.header.rules[1].max_length
        ),
        (4, 10)
    );
}

#[test]
fn header_defaults_without_columns() {
    let table = Table::parse("---\nname: x\n...\n王\tggll\t5\n");
    assert_eq!(table.header.columns, ["text", "code", "weight"]);
    assert_eq!(table.entries.len(), 1);
    assert_eq!(table.entries[0].weight, 5);
    assert_eq!(table.entries[0].stem, None);
}

#[test]
fn body_rows_and_counts() {
    let table = Table::parse(SAMPLE);
    // 16 行表体：14 条有效 + 1 条 z 码 + 1 条非法码；`#子` 算注释
    assert_eq!(table.entries.len(), 13);
    assert_eq!(table.z_codes, 1);
    assert_eq!(table.invalid, 1);
    assert_eq!(table.lines, SAMPLE.lines().count());
    let first = &table.entries[0];
    assert_eq!((first.text.as_str(), first.code.as_str()), ("工", "a"));
    assert_eq!(first.weight, 99_454_797);
    assert_eq!(first.stem.as_deref(), Some("aa"));
    let second = &table.entries[1];
    assert_eq!(second.stem, None);
    let two_columns = table.entries.iter().find(|e| e.text == "𬳶").unwrap();
    assert_eq!(two_columns.weight, 0);
    assert_eq!(two_columns.code, "cmkg");
}

#[test]
fn body_without_yaml_header() {
    let table = Table::parse("# 手写表\n王\tggll\t5\n工\ta\n");
    assert_eq!(table.entries.len(), 2);
    assert_eq!(table.comments, 1);
}

#[test]
fn weight_accepts_float_and_clamps() {
    let table = Table::parse("---\n...\n王\tggll\t1e5\n工\ta\t99999999999\n一\tg\t-3\n");
    let weights: Vec<u32> = table.entries.iter().map(|e| e.weight).collect();
    assert_eq!(weights, [100_000, u32::MAX, 0]);
}

#[test]
fn merge_takes_max_weight() {
    let table = Table::parse(SAMPLE);
    let (entries, _, merged) = select(&table.entries, None);
    assert_eq!(merged, 1);
    let wang: Vec<_> = entries
        .iter()
        .filter(|e| e.text == "王" && e.code == "gggg")
        .collect();
    assert_eq!(wang.len(), 1);
    assert_eq!(wang[0].weight, 827_000_000);
}

#[test]
fn gb2312_membership() {
    assert!(is_gb2312('王'));
    assert!(is_gb2312('龟'));
    assert!(!is_gb2312('龘'));
    assert!(!is_gb2312('𬳶'));
    assert!(!is_gb2312('→'));
    assert!(!is_gb2312('a'));
}

#[test]
fn charset_filter_default_and_extended() {
    let table = Table::parse(SAMPLE);
    let (kept, filtered, _) = select(&table.entries, Some(&Charset::gb2312()));
    assert_eq!(filtered, 2);
    assert!(kept.iter().all(|e| e.text != "龘" && e.text != "𬳶"));
    let (all, filtered, _) = select(&table.entries, None);
    assert_eq!(filtered, 0);
    assert!(all.iter().any(|e| e.text == "龘"));
}

#[test]
fn charset_extends_with_dictionary() {
    let dir = temp_dir("charset");
    let dict = dir.join("dict.tsv");
    std::fs::write(&dict, "龘龘\tda da\t3\n").unwrap();
    let charset = Charset::with_dictionary(&dict).unwrap();
    assert!(charset.accepts("龘"));
    assert!(charset.accepts("龘王"));
    assert!(!charset.accepts("𬳶"));
    let table = Table::parse(SAMPLE);
    let (kept, filtered, _) = select(&table.entries, Some(&charset));
    assert_eq!(filtered, 1);
    assert!(kept.iter().any(|e| e.text == "龘"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn full_codes_take_longest_and_check_stems() {
    let table = Table::parse(SAMPLE);
    let (entries, _, _) = select(&table.entries, None);
    let codes = FullCodes::collect(&entries);
    assert_eq!(codes.get('王'), Some("gggg"));
    assert_eq!(codes.get('工'), Some("aaaa"));
    assert_eq!(codes.get('了'), Some("bnh"));
    assert_eq!(codes.get('人'), Some("wwww"));
    assert_eq!(codes.len(), 7);
    assert_eq!(codes.short(), [('了', "bnh")]);
    let mismatches = codes.check_stems(&entries);
    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0], ('了', "bx".to_owned(), "bnh".to_owned()));
}

#[test]
fn output_loads_as_dictionary_and_packs_to_qj() {
    let dir = temp_dir("convert");
    let input = dir.join("wubi86.dict.yaml");
    std::fs::write(&input, SAMPLE).unwrap();
    let tsv = dir.join("wubi86.tsv");
    convert(&input, &tsv, false, None).unwrap();

    let dictionary = Dictionary::from_path(&tsv).unwrap();
    let qj = dir.join("wubi86.qj");
    dictionary
        .write_qj(
            &qj,
            &Metadata {
                name: "五笔 86 码表（极点）".to_owned(),
                license: "LGPL-3.0".to_owned(),
                ..Metadata::default()
            },
        )
        .unwrap();
    let opened = Dictionary::open_qj(&qj).unwrap();
    assert_eq!(opened.len(), dictionary.len());
    assert_eq!(
        opened.metadata().map(|m| m.license.as_str()),
        Some("LGPL-3.0")
    );

    // 王 是 G 键的键名字，全码 gggg；一 才是 ggll
    let wang: Vec<&str> = opened
        .lookup_exact(&[SyllablePattern::complete("gggg")])
        .iter()
        .map(|m| m.text)
        .collect();
    assert_eq!(wang, ["王"]);
    let yi: Vec<&str> = opened
        .lookup_exact(&[SyllablePattern::complete("ggll")])
        .iter()
        .map(|m| m.text)
        .collect();
    assert_eq!(yi, ["一"]);
    let gong: Vec<&str> = opened
        .lookup_exact(&[SyllablePattern::complete("a")])
        .iter()
        .map(|m| m.text)
        .collect();
    assert_eq!(gong, ["工"]);
    // 前缀查询就是逐键提示
    let prefix: Vec<&str> = opened
        .lookup_pattern(&[SyllablePattern::prefix("gg")])
        .iter()
        .map(|m| m.text)
        .collect();
    assert!(prefix.contains(&"王") && prefix.contains(&"一"));
    assert!(
        opened
            .lookup_exact(&[SyllablePattern::complete("zzj")])
            .is_empty()
    );
    std::fs::remove_dir_all(dir).unwrap();
}
