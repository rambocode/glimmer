//! 路径上的词按什么量宽度：第二轮重排要把几条路径对齐到同一把尺子上才能拼。

use crate::sentence::SentenceWord;

/// 一个词在输入里占多宽。拼音的路径按音节对齐；五笔一个词只有一条编码（一个「音节」），
/// 同一段字母切法不同时词数就不同，按音节数对不齐，要按编码的字母数量。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathUnit {
    /// 拼音：词的音节数。
    Syllables,

    /// 五笔：词敲了几个字母（[`SentenceWord::syllables`] 里是敲的那段编码）。
    CodeLetters,
}

impl PathUnit {
    /// `word` 的宽度。
    pub(crate) fn width(self, word: &SentenceWord) -> usize {
        match self {
            Self::Syllables => word.syllables.len(),
            Self::CodeLetters => word.syllables.iter().map(String::len).sum(),
        }
    }
}
