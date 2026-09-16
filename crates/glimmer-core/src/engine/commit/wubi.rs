//! 上屏里的五笔分支：候选按编码消耗缓冲区、造词的编码、查重用的词库。

use glimmer_dictionary::Dictionary;

use crate::candidate::Candidate;
use crate::engine::{Engine, Learner, choice_key};

impl Engine {
    /// 五笔查词与查重用的词库：码表加用户词。用户词学习器按方案分目录，里面记的都是编码。
    pub(in crate::engine) fn wubi_dictionaries(&self) -> Vec<&Dictionary> {
        let mut all = Vec::with_capacity(2);
        if let Some(scheme) = &self.wubi {
            all.push(scheme.dictionary());
        }
        if let Some(user) = self.learner.user_words() {
            all.push(user);
        }
        all
    }

    /// 五笔候选消耗多少作用域字节：候选的编码那么多（提示候选的编码比敲的长，吃完作用域；反查 / 整句候选的音节就是整段），
    /// 学习键就是吃掉的编码。
    pub(in crate::engine) fn wubi_consumed_by(&self, candidate: &Candidate) -> (usize, String) {
        let keys = self.composition.scope();
        let consumed = candidate
            .syllables
            .first()
            .map_or(keys.len(), String::len)
            .min(keys.len());
        (consumed, choice_key(keys, consumed))
    }

    /// 自动造出的词按什么音节记：拼音下就是各词的音节连起来（字数要与音节数相等），
    /// 五笔下按造词规则算编码（某字全码不够长就造不出，返回 `None`）。
    pub(in crate::engine) fn auto_word_syllables(
        &self,
        text: &str,
        pinyin: Vec<String>,
    ) -> Option<Vec<String>> {
        match &self.wubi {
            Some(scheme) => scheme.code_of(text).map(|code| vec![code]),
            None => (text.chars().count() == pinyin.len()).then_some(pinyin),
        }
    }
}
