//! 英文音标：从课本写法的音标表（`词\t/音标/`，`tools/corpus/ipa_en.py` 生成）查英文译词的读音，写进 `glossary-en.tsv` 的 `|读音` 槽。
//! 只取每个词的第一种读法，去掉斜线；多词短语按词逐个查，全查到才拼起来。

mod dict;

pub use dict::IpaDict;

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::error::GlossError;

/// 给一张已导出的 `glossary-en.tsv` 原地补音标（重跑覆盖旧音标），返回（释义总数, 补上音标的数）。
pub fn annotate(glossary: &Path, dict: &IpaDict) -> Result<(usize, usize), GlossError> {
    let source = fs::read_to_string(glossary)?;
    let mut out = String::with_capacity(source.len() * 2);
    let (mut total, mut hit) = (0usize, 0usize);
    for line in source.lines() {
        if line.is_empty() || line.starts_with('#') {
            out.push_str(header(line));
            out.push('\n');
            continue;
        }
        let mut fields = line.split('\t');
        // 第一列是中文词，原样保留
        out.push_str(fields.next().unwrap_or_default());
        for field in fields {
            total += 1;
            out.push('\t');
            out.push_str(&annotate_sense(field, dict, &mut hit));
        }
        out.push('\n');
    }
    // 先写临时文件再改名，中途出错不会留下半张表
    let temp = glossary.with_extension("tsv.tmp");
    fs::File::create(&temp)?.write_all(out.as_bytes())?;
    fs::rename(&temp, glossary)?;
    Ok((total, hit))
}

/// 表头注释换成带音标的写法，别的注释行原样。
fn header(line: &str) -> &str {
    if line.starts_with("# 由 glimmer-gloss-gen 生成") {
        HEADER
    } else {
        line
    }
}

/// `glossary-en.tsv` 的表头。
pub const HEADER: &str = "# 由 glimmer-gloss-gen 生成（LLM），音标来自 ipa-dict。词\t[词性. ]译词[|音标]\t[词性. ]译词[|音标]";

/// 一条 `[词性. ]译词[|旧音标]` → `[词性. ]译词[|音标]`。
fn annotate_sense(field: &str, dict: &IpaDict, hit: &mut usize) -> String {
    let sense = field
        .split_once('|')
        .map_or(field, |(sense, _)| sense)
        .trim();
    match dict.lookup(sense_text(sense)) {
        Some(ipa) => {
            *hit += 1;
            format!("{sense}|{ipa}")
        }
        None => sense.to_owned(),
    }
}

/// 去掉 `v. ` 这样的词性前缀，只留译词。
fn sense_text(sense: &str) -> &str {
    match sense.split_once(' ') {
        Some((head, rest)) if head.ends_with('.') => rest.trim(),
        _ => sense,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict() -> IpaDict {
        IpaDict::parse(
            "develop\t/dɪˈveləp/\nexploit\t/ˈeksˌplɔɪt/, /ˌeksˈplɔɪt/\nice\t/aɪs/\ncream\t/kriːm/\n",
        )
    }

    #[test]
    fn annotates_senses_and_replaces_old_reading() {
        let dict = dict();
        let mut hit = 0;
        assert_eq!(
            annotate_sense("v. develop", &dict, &mut hit),
            "v. develop|dɪˈveləp"
        );
        assert_eq!(
            annotate_sense("v. develop|old", &dict, &mut hit),
            "v. develop|dɪˈveləp"
        );
        assert_eq!(
            annotate_sense("n. ice cream", &dict, &mut hit),
            "n. ice cream|aɪs kriːm"
        );
        assert_eq!(annotate_sense("unknown", &dict, &mut hit), "unknown");
        assert_eq!(hit, 3);
    }

    #[test]
    fn rewrites_whole_table() {
        let dir = std::env::temp_dir().join(format!("glimmer-ipa-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("glossary-en.tsv");
        std::fs::write(
            &path,
            "# 由 glimmer-gloss-gen 生成（LLM）。词\t[词性. ]译词\n开发\tv. develop\tv. exploit\n冰淇淋\tn. ice cream\tn. gelato\n",
        )
        .unwrap();
        let (total, hit) = annotate(&path, &dict()).unwrap();
        assert_eq!((total, hit), (4, 3));
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            format!(
                "{HEADER}\n开发\tv. develop|dɪˈveləp\tv. exploit|ˈeksˌplɔɪt\n冰淇淋\tn. ice cream|aɪs kriːm\tn. gelato\n"
            )
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
