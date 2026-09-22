//! 应用光标前文的一份加工结果：原文 + 从它末尾切出来的最后两个词。

use crate::sentence::{self, Context, LanguageModel};

/// 壳读来的「光标前的文字」，连同按语言模型切出来的末尾两个词。
///
/// 切词只在壳送来新前文时做一次（组句期间前文不变），不是每键都切：切一段 64 字的文本要跑一遍 Viterbi，
/// 摊到每一键上是白花的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SurroundingBefore {
    /// 壳给的原文，原样留着给神经重打分当前文（它按字看，不需要切词）。
    text: String,

    /// 前文末尾那一小句的最后一个词；末尾不是汉字（标点、字母、空）时为 `None`，即光标在句首。
    previous: Option<String>,

    /// 最后一个词之前的那个词；不足两个词时为 `None`。
    earlier: Option<String>,
}

impl SurroundingBefore {
    /// 切出末尾两个词。只看末尾**连续的汉字**：遇到标点、换行、字母数字就断——那些地方是句子的边界，
    /// 跨过去接上文只会把不相干的话当前文（「你好。」之后打的第一个词该按句首算，不该接「你好」）。
    pub(crate) fn new(text: String, model: &dyn LanguageModel) -> Self {
        let clause_start = text
            .char_indices()
            .rev()
            .take_while(|(_, c)| sentence::is_han(*c))
            .last()
            .map(|(index, _)| index);
        let words = clause_start
            .and_then(|start| sentence::segment_text(&text[start..], model))
            .map(|clauses| clauses.into_iter().flatten().collect::<Vec<String>>())
            .unwrap_or_default();
        let mut tail = words.into_iter().rev();
        let previous = tail.next();
        let earlier = tail.next();
        Self {
            text,
            previous,
            earlier,
        }
    }

    /// 壳给的原文。
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// 接着这段前文往下打时，下一个词的上文。
    pub(crate) fn context(&self) -> Context<'_> {
        Context {
            previous: self.previous.as_deref(),
            earlier: self.earlier.as_deref(),
        }
    }
}
