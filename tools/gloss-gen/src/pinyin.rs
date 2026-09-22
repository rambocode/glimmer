//! 拼音标注：给多音字词标读音。词表一行一个词（`dict-convert lexicon --emit-ambiguous` 写出的），
//! 结果 JSONL 每行 `{"word":"一行","pinyin":["yi","hang"],"readings":[{"pinyin":["yi","hang"],"share":0.7},{"pinyin":["yi","xing"],"share":0.3}]}`，
//! `pinyin` 是主读音，`readings` 是全部常用读音与占比（词义不同读音不同的词才有多条）；
//! `dict-convert lexicon --pinyin` 读回去、按 Unihan 校验，再按占比把词频分给各个读音。

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use crate::args::PinyinArgs;
use crate::batch::{self, Task};
use crate::client::LlmClient;
use crate::entry::{PinyinEntry, PinyinReading};
use crate::error::GlossError;
use crate::store::Store;

/// 占比低于这个值的次要读音不要：模型爱把罕见、文言或错读也列上，留着会在输入法里冒出错字。
const MIN_SHARE: f64 = 0.05;

const SYSTEM_PROMPT: &str = "你是普通话审音专家。给每个中文词标注拼音，供拼音输入法建词库用。\n\
规则：\n\
- 多音字按这个词里的实际读音（重庆 chong2 qing4，长沙 chang2 sha1，蚌埠 beng4 bu4，行长 hang2 zhang3，六安 lu4 an1）；地名、人名、专业术语按通行读法。\n\
- 同一个词写法有几种常用读法、去掉声调后拼写不同时，每种都列出来，并估计各自在日常说话写字里的占比（百分数，合计 100）。包括两类：\
一是词义不同读音不同（一行 yi4 hang2 一行字 / yi4 xing2 一行人，朝阳 zhao1 yang2 / chao2 yang2 朝阳区，差事 chai1 shi4 / cha4 shi4 不中用）；\
二是口语里很多人这么读、输入法用户会按它打的读法（主角 zhu3 jue2 / zhu3 jiao3，角色 jue2 se4 / jiao3 se4，流血 liu2 xue4 / liu2 xie3，落枕 lao4 zhen3 / luo4 zhen3）。\n\
- 只有声调不同的不算（好事 hao3 / hao4 去掉声调都是 hao shi），只列一个。绝大多数词只有一个读法，就只列一个（占比 100）。\
不要列罕见、文言、方言或错误的读音，不要因为某个字是多音字就把它的别的读音都列上（银行 只有 yin2 hang2，长大 只有 zhang3 da4）。\n\
- 每个汉字恰好一个音节，音节之间空格，带声调数字 1–4，轻声 5；ü 写 v（女 nv3，略 lve4）；儿化的「儿」单独一个音节 er。\n\
- 不要解释，没有把握也要给最可能的读音。\n\
输出严格的 JSON：{\"items\":[{\"w\":\"一行\",\"r\":[{\"py\":\"yi4 hang2\",\"p\":70},{\"py\":\"yi4 xing2\",\"p\":30}]},{\"w\":\"重庆\",\"r\":[{\"py\":\"chong2 qing4\",\"p\":100}]}]}。\
items 与输入的词一一对应、顺序一致、每个词恰好一项，w 必须原样照抄输入的词，r 按占比从高到低排。";

/// 拼音任务。
struct PinyinTask;

#[derive(Debug, Deserialize)]
struct RawReply {
    #[serde(default)]
    items: Vec<RawItem>,
}

#[derive(Debug, Deserialize)]
struct RawItem {
    w: String,

    /// 各个读音。
    #[serde(default)]
    r: Vec<RawReading>,

    /// 旧格式的单个读音：模型偶尔不按新格式回，也收。
    #[serde(default)]
    py: String,
}

#[derive(Debug, Deserialize)]
struct RawReading {
    #[serde(default)]
    py: String,

    /// 占比，百分数。
    #[serde(default)]
    p: f64,
}

impl Task for PinyinTask {
    type Entry = PinyinEntry;

    fn system_prompt(&self) -> &str {
        SYSTEM_PROMPT
    }

    fn user_prompt(&self, words: &[String]) -> String {
        let mut text = String::from("词：\n");
        for word in words {
            text.push_str(word);
            text.push('\n');
        }
        text
    }

    /// 只收请求过的词；音节数必须等于字数，音节只能是字母（声调数字去掉）。
    fn parse_reply(
        &self,
        content: &str,
        words: &[String],
    ) -> Result<Vec<PinyinEntry>, serde_json::Error> {
        let reply: RawReply = serde_json::from_str(content.trim())?;
        let mut entries: Vec<PinyinEntry> = Vec::with_capacity(words.len());
        for item in reply.items {
            let word = item.w.trim();
            if !words.iter().any(|w| w == word) || entries.iter().any(|e| e.word == word) {
                continue;
            }
            let raw: Vec<(String, f64)> = if item.r.is_empty() {
                vec![(item.py, 100.0)]
            } else {
                item.r.into_iter().map(|r| (r.py, r.p)).collect()
            };
            let readings = clean_readings(&raw, word.chars().count());
            let Some(primary) = readings.first() else {
                tracing::debug!(word, ?raw, "拼音格式不对，丢弃");
                continue;
            };
            entries.push(PinyinEntry {
                word: word.to_owned(),
                pinyin: primary.pinyin.clone(),
                readings,
            });
        }
        Ok(entries)
    }
}

/// 模型给的 (读音, 百分数) → 去掉格式不对的、合并去声调后相同的（一 的 yi2 / yi4）、丢掉占比过低的次要读音，
/// 占比归一到合计 1，按占比从高到低排。一个能用的都没有就返回空。
fn clean_readings(raw: &[(String, f64)], chars: usize) -> Vec<PinyinReading> {
    let mut merged: Vec<PinyinReading> = Vec::new();
    for (py, percent) in raw {
        let Some(pinyin) = clean_pinyin(py, chars) else {
            continue;
        };
        // 占比缺了或不是正数的当作平分，别让它在归一时变成 0 被丢掉
        let share = if percent.is_finite() && *percent > 0.0 {
            *percent
        } else {
            1.0
        };
        match merged.iter_mut().find(|r| r.pinyin == pinyin) {
            Some(existing) => existing.share += share,
            None => merged.push(PinyinReading { pinyin, share }),
        }
    }
    let total: f64 = merged.iter().map(|r| r.share).sum();
    if total <= 0.0 {
        return Vec::new();
    }
    for reading in &mut merged {
        reading.share /= total;
    }
    merged.sort_by(|a, b| b.share.total_cmp(&a.share));
    // 主读音总留着；次要读音太少的去掉后再归一一次
    let primary = merged.first().map(|r| r.pinyin.clone());
    merged.retain(|r| Some(&r.pinyin) == primary.as_ref() || r.share >= MIN_SHARE);
    let total: f64 = merged.iter().map(|r| r.share).sum();
    for reading in &mut merged {
        reading.share /= total;
    }
    merged
}

/// `chong2 qing4` → `["chong", "qing"]`；音节数不等于字数、含非字母字符（去掉声调数字与 ü 转 v 之后）就不要。
fn clean_pinyin(raw: &str, chars: usize) -> Option<Vec<String>> {
    let syllables: Vec<String> = raw
        .split(|c: char| c.is_whitespace() || c == '\'')
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.chars()
                .filter(|c| !c.is_ascii_digit())
                .map(|c| match c {
                    'ü' | 'Ü' => 'v',
                    c => c.to_ascii_lowercase(),
                })
                .collect::<String>()
        })
        // j / q / x / y 后面的 ü 拼写里写 u：模型照「ü 写 v」会写出 xve、qv
        .map(|s| match s.as_bytes() {
            [b'j' | b'q' | b'x' | b'y', b'v', ..] => format!("{}u{}", &s[..1], &s[2..]),
            _ => s,
        })
        .collect();
    let ok = syllables.len() == chars
        && syllables
            .iter()
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase()));
    ok.then_some(syllables)
}

pub async fn run(args: PinyinArgs) -> Result<(), GlossError> {
    let source = std::fs::read_to_string(&args.words)?;
    let selected: Vec<String> = source
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect();
    if selected.is_empty() {
        return Err(GlossError::NoWords(args.words.clone()));
    }
    let store = Arc::new(Store::<PinyinEntry>::open(&args.out)?);
    let mut pending: Vec<String> = selected
        .iter()
        .filter(|w| !store.contains(w))
        .cloned()
        .collect();
    if let Some(limit) = args.limit {
        pending.truncate(limit);
    }
    tracing::info!(
        selected = selected.len(),
        already_done = store.len(),
        pending = pending.len(),
        model = %args.model,
        "开始标注拼音"
    );
    if pending.is_empty() {
        return Ok(());
    }
    let client = Arc::new(LlmClient::new(
        &args.base_url,
        &args.api_key,
        &args.model,
        Duration::from_secs(args.timeout_secs),
    ));
    batch::run(
        Arc::new(PinyinTask),
        client,
        store,
        pending,
        args.batch,
        args.concurrency,
    )
    .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_tones_and_rejects_wrong_syllable_counts() {
        assert_eq!(clean_pinyin("chong2 qing4", 2).unwrap(), ["chong", "qing"]);
        assert_eq!(clean_pinyin("nv3'ren2", 2).unwrap(), ["nv", "ren"]);
        assert_eq!(clean_pinyin("LÜ4", 1).unwrap(), ["lv"]);
        assert_eq!(clean_pinyin("xve2 qv4", 2).unwrap(), ["xue", "qu"]);
        assert!(clean_pinyin("chong2", 2).is_none());
        assert!(clean_pinyin("chong2 qing4 shi4", 2).is_none());
        assert!(clean_pinyin("chong-2 qing4", 2).is_none());
    }

    #[test]
    fn parses_only_requested_words() {
        let words = vec!["重庆".to_owned(), "长沙".to_owned()];
        let reply = r#"{"items":[{"w":"重庆","py":"chong2 qing4"},{"w":"北京","py":"bei3 jing1"},{"w":"长沙","py":"chang2"}]}"#;
        let entries = PinyinTask.parse_reply(reply, &words).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].word, "重庆");
        assert_eq!(entries[0].readings.len(), 1);
    }

    #[test]
    fn keeps_several_readings_with_normalized_shares() {
        let words = vec!["一行".to_owned()];
        let reply = r#"{"items":[{"w":"一行","r":[{"py":"yi4 xing2","p":30},{"py":"yi2 hang2","p":50},{"py":"yi4 hang2","p":17},{"py":"yi1 heng2","p":3}]}]}"#;
        let entries = PinyinTask.parse_reply(reply, &words).unwrap();
        let entry = &entries[0];
        // yi2 hang2 与 yi4 hang2 去声调后相同，合并；heng 占比 3% 丢掉
        assert_eq!(entry.pinyin, ["yi", "hang"]);
        assert_eq!(entry.readings.len(), 2);
        assert_eq!(entry.readings[1].pinyin, ["yi", "xing"]);
        assert!((entry.readings[0].share - 67.0 / 97.0).abs() < 1e-9);
        assert!((entry.readings.iter().map(|r| r.share).sum::<f64>() - 1.0).abs() < 1e-9);
    }
}
