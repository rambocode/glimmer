//! `gloss-gen pinyin` 写出的 JSONL：每行 `{"word":"一行","pinyin":["yi","hang"],"readings":[{"pinyin":["yi","hang"],"share":0.7},…]}`；
//! 旧版文件只有 `pinyin`（主读音），按占比 1 的单个读音读。

use std::collections::HashMap;
use std::path::Path;

use glimmer_dictionary::canonical_syllable;
use serde::Deserialize;

use crate::error::ConvertError;

#[derive(Debug, Deserialize)]
struct Row {
    word: String,

    pinyin: Vec<String>,

    #[serde(default)]
    readings: Vec<RowReading>,
}

#[derive(Debug, Deserialize)]
struct RowReading {
    pinyin: Vec<String>,

    share: f64,
}

/// 一个词标注出的一个读音：音节与占比（同一个词的各读音合计 1）。
pub type Annotation = (Vec<String>, f64);

/// 词 → 各读音，主读音在前。同一个词以最后一条为准，坏行跳过。
pub fn load(path: &Path) -> Result<HashMap<String, Vec<Annotation>>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    let mut out = HashMap::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Row>(line) {
            Ok(row) => {
                let canonical = |syllables: Vec<String>| -> Vec<String> {
                    syllables
                        .iter()
                        .map(|s| canonical_syllable(&restore_u(s)).to_owned())
                        .collect()
                };
                let readings: Vec<Annotation> = if row.readings.is_empty() {
                    vec![(canonical(row.pinyin), 1.0)]
                } else {
                    row.readings
                        .into_iter()
                        .map(|r| (canonical(r.pinyin), r.share))
                        .collect()
                };
                out.insert(row.word, readings);
            }
            Err(error) => tracing::warn!(%error, "拼音标注坏行，跳过"),
        }
    }
    Ok(out)
}

/// j / q / x / y 后面的 ü 拼写里写 u：模型照「ü 写 v」把 学 写成 xve、去 写成 qv，Unihan 校验会把它当错读丢掉。
fn restore_u(syllable: &str) -> String {
    match syllable.as_bytes() {
        [b'j' | b'q' | b'x' | b'y', b'v', ..] => format!("{}u{}", &syllable[..1], &syllable[2..]),
        _ => syllable.to_owned(),
    }
}

/// 把多读音标注并进主标注：`base` 里有的词，只有 `extra` 的列表含它的主读音、且不止一个读音时才换成 `extra` 的列表
/// （主读音不动：多读音那轮批量标注时模型会把少数词的主读音改错，不对称 标成 bu dui chen）；`base` 没有的词直接收。
/// 返回换成多读音的词数。
pub fn merge_readings(
    base: &mut HashMap<String, Vec<Annotation>>,
    extra: HashMap<String, Vec<Annotation>>,
) -> usize {
    let mut merged = 0usize;
    for (word, list) in extra {
        match base.get_mut(&word) {
            Some(current) => {
                let primary = &current[0].0;
                if list.len() > 1 && list.iter().any(|(s, _)| s == primary) {
                    *current = list;
                    merged += 1;
                }
            }
            None => {
                if list.len() > 1 {
                    merged += 1;
                }
                base.insert(word, list);
            }
        }
    }
    tracing::info!(merged, "多读音标注已并入");
    merged
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
            "{\"word\":\"策略\",\"pinyin\":[\"ce\",\"lue\"]}\n{\"word\":\"虐待\",\"pinyin\":[\"nue\",\"dai\"]}\n\
{\"word\":\"一行\",\"pinyin\":[\"yi\",\"hang\"],\"readings\":[{\"pinyin\":[\"yi\",\"hang\"],\"share\":0.7},{\"pinyin\":[\"yi\",\"xing\"],\"share\":0.3}]}\n",
        )
        .unwrap();
        let annotations = load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            annotations["策略"],
            [(vec!["ce".to_owned(), "lve".to_owned()], 1.0)]
        );
        assert_eq!(annotations["虐待"][0].0, ["nve", "dai"]);
        assert_eq!(annotations["一行"].len(), 2);
        assert_eq!(
            annotations["一行"][1],
            (vec!["yi".to_owned(), "xing".to_owned()], 0.3)
        );
    }

    #[test]
    fn restores_u_after_j_q_x_y() {
        assert_eq!(restore_u("xve"), "xue");
        assert_eq!(restore_u("qv"), "qu");
        assert_eq!(restore_u("yvan"), "yuan");
        assert_eq!(restore_u("lve"), "lve");
    }

    #[test]
    fn merges_extra_readings_only_when_they_keep_the_primary() {
        let s = |text: &str| text.split(' ').map(str::to_owned).collect::<Vec<_>>();
        let mut base = HashMap::from([
            ("一行".to_owned(), vec![(s("yi xing"), 1.0)]),
            ("对称".to_owned(), vec![(s("dui cheng"), 1.0)]),
        ]);
        let extra = HashMap::from([
            (
                "一行".to_owned(),
                vec![(s("yi hang"), 0.7), (s("yi xing"), 0.3)],
            ),
            (
                "对称".to_owned(),
                vec![(s("dui chen"), 0.6), (s("dui qi"), 0.4)],
            ),
            (
                "角色".to_owned(),
                vec![(s("jue se"), 0.6), (s("jiao se"), 0.4)],
            ),
        ]);
        assert_eq!(merge_readings(&mut base, extra), 2);
        assert_eq!(base["一行"][0].0, s("yi hang"));
        assert_eq!(base["对称"], vec![(s("dui cheng"), 1.0)]);
        assert_eq!(base["角色"].len(), 2);
    }
}
