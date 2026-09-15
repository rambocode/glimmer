//! `gloss-gen pinyin` 写出的 JSONL：每行 `{"word":"重庆","pinyin":["chong","qing"]}`。

use std::collections::HashMap;
use std::path::Path;

use glimmer_dictionary::canonical_syllable;
use serde::Deserialize;

use crate::error::ConvertError;

#[derive(Debug, Deserialize)]
struct Row {
    word: String,

    pinyin: Vec<String>,
}

/// 词 → 音节。同一个词以最后一条为准，坏行跳过。
pub fn load(path: &Path) -> Result<HashMap<String, Vec<String>>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    let mut out = HashMap::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Row>(line) {
            Ok(mut row) => {
                for syllable in &mut row.pinyin {
                    *syllable = canonical_syllable(syllable).to_owned();
                }
                out.insert(row.word, row.pinyin);
            }
            Err(error) => tracing::warn!(%error, "拼音标注坏行，跳过"),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_u_umlaut_spellings() {
        let path = std::env::temp_dir().join(format!(
            "glimmer-annotations-normalize-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "{\"word\":\"策略\",\"pinyin\":[\"ce\",\"lue\"]}\n{\"word\":\"虐待\",\"pinyin\":[\"nue\",\"dai\"]}\n",
        )
        .unwrap();
        let annotations = load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(annotations["策略"], ["ce", "lve"]);
        assert_eq!(annotations["虐待"], ["nve", "dai"]);
    }
}
