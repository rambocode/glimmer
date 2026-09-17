//! 生成覆盖清单：区分主词库、可选 IT 词库、英文表及本次独立领域词库。

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;
use std::path::Path;

use glimmer_dictionary::{Dictionary, WordList};

use crate::error::ConvertError;

/// 参考表位于源目录的上一级（标准布局为 assets/lexicon）。缺失时标明 unknown，
/// 不把缺少参考文件解释成词条缺失；实际主词库来源以 --exclude 参数为准。
pub(super) fn report(
    source: &Path,
    rows: &BTreeMap<(String, String), Vec<String>>,
    main: &HashSet<(String, String)>,
) -> Result<String, ConvertError> {
    let root = source.parent().unwrap_or(source);
    let it_path = root.join("dicts/it_computing.tsv");
    let it = if it_path.exists() {
        Some(Dictionary::from_path(&it_path)?)
    } else {
        None
    };
    let it_words: HashSet<(&str, &str)> = it
        .as_ref()
        .into_iter()
        .flat_map(Dictionary::entries)
        .map(|e| (e.text, e.pinyin))
        .collect();
    let english_path = root.join("english.tsv");
    let english = if english_path.exists() {
        Some(WordList::from_path(&english_path)?)
    } else {
        None
    };
    let mut report = String::from(
        "# 词\t输入码\t类型\t分类\t主词库同词同码\t可选IT同词同码\t英文表同码同词\t英文同码其他词\t本领域\t来源\n",
    );
    let status = |present: bool, known: bool| {
        if !known {
            "unknown"
        } else if present {
            "yes"
        } else {
            "no"
        }
    };
    for ((word, code), fields) in rows {
        let refs: Vec<&str> = fields.iter().map(String::as_str).collect();
        let alias = super::validation::is_alias(&refs);
        let key = if alias {
            format!("@{code}")
        } else {
            code.clone()
        };
        let in_main = main.contains(&(word.clone(), key));
        let english_word = english.as_ref().and_then(|list| list.get(code));
        writeln!(
            report,
            "{word}\t{code}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            fields[4],
            fields[3],
            status(in_main, !main.is_empty()),
            status(
                it_words.contains(&(word.as_str(), code.as_str())),
                it.is_some()
            ),
            status(english_word == Some(word), english.is_some()),
            english_word.filter(|other| *other != word).unwrap_or("-"),
            if in_main { "existing" } else { "included" },
            fields[6]
        )
        .expect("String write");
    }
    Ok(report)
}
