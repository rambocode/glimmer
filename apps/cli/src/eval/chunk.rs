//! 分段输入：把一句拆成几个词一段，逐段喂给引擎，前面各段的原文当上文。
//!
//! 人很少一口气打完一句再上屏，多半两三个词一段。整句评测一句一条时上文几乎总是标点（句首），
//! 量不出「这段拼音的第一个词接着前面的词算」这类改动；拆开之后每段的上文就是同一句里前面的字。

use glimmer_core::sentence::{LanguageModel, segment_text};

use super::pair::{MAX_CONTEXT_CHARS, Pair};
use super::transcribe::Transcriber;

/// 一段至少几个字：单字段只有词级候选，不经过整句转换。
const MIN_CHUNK_CHARS: usize = 2;

/// 把 `pair` 按语言模型切词，每 `words` 个词一段；每段的上文 = 原上文 + 同句里前面各段的原文。
/// 有字查不到读音的段跳过（后面的段照常，上文仍含它）。
pub fn split(
    pair: &Pair,
    words: usize,
    transcriber: &Transcriber,
    model: &dyn LanguageModel,
) -> Vec<Pair> {
    let segmented: Vec<String> = match segment_text(&pair.text, model) {
        Some(clauses) => clauses.into_iter().flatten().collect(),
        None => return Vec::new(),
    };
    let mut chunks = Vec::new();
    let mut before = pair.context.clone();
    for group in segmented.chunks(words.max(1)) {
        let text: String = group.concat();
        if text.chars().count() >= MIN_CHUNK_CHARS
            && let Some(pinyin) = transcriber.transcribe(&text, model)
        {
            chunks.push(Pair {
                text: text.clone(),
                pinyin,
                context: last_chars(&before, MAX_CONTEXT_CHARS),
            });
        }
        before.push_str(&text);
    }
    chunks
}

fn last_chars(text: &str, count: usize) -> String {
    let total = text.chars().count();
    text.chars().skip(total.saturating_sub(count)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use glimmer_dictionary::Dictionary;

    /// 认得三个词的假模型。
    struct Model;

    impl LanguageModel for Model {
        fn log_prob(&self, _: Option<&str>, word: &str) -> Option<f64> {
            matches!(word, "我们" | "需要" | "测试").then_some(-3.0)
        }
    }

    #[test]
    fn later_chunks_see_the_earlier_text_as_context() {
        let dictionary =
            Dictionary::parse("我们\two men\t9\n需要\txu yao\t9\n测试\tce shi\t9\n").unwrap();
        let transcriber = Transcriber::new([&dictionary]);
        let pair = Pair {
            text: "我们需要测试".to_owned(),
            pinyin: "womenxuyaoceshi".to_owned(),
            context: "好，".to_owned(),
        };
        let chunks = split(&pair, 2, &transcriber, &Model);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "我们需要");
        assert_eq!(chunks[0].pinyin, "womenxuyao");
        assert_eq!(chunks[0].context, "好，");
        assert_eq!(chunks[1].text, "测试");
        assert_eq!(chunks[1].context, "好，我们需要");
    }
}
