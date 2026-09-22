//! 从用户自己写的文本里学个人 n-gram。
//!
//! 静态语言模型的语料没有用户那个行当的文本，行话（词库 / 候选 / 释义表）全靠个人计数补；等用户一条条打出来太慢，
//! 他已经写好的笔记与文档就是现成的个人语料。2026-09-19 留出评测（学 42 个文件、测另外 10 个里没原样出现过的 1487 句）：
//! 整句首选 24.3% → 60.8%，字准确率 70.7% → 90.1%。

use crate::engine::{Engine, Learner, TRANSITION_WEIGHT};
use crate::sentence::{self, Context};

impl Engine {
    /// 学一段文本：按非汉字切成小句、按语言模型切词，逐词把转移记进学习器，每个小句从句首起、前两个词当上文
    /// （与上屏时记的一样，见 `record_word`）。只动个人 n-gram：不记词频、不造词、不写输入日志；私密输入中或学习关着时不学、返回 0。
    /// 文本从哪来（文件、剪贴板）是壳的事。返回记了多少条转移。
    pub fn learn_text(&mut self, text: &str) -> usize {
        if self.learner.muted() {
            return 0;
        }
        let Some(clauses) = sentence::segment_text(text, &*self.language_model) else {
            return 0;
        };
        self.forget_span_cache();
        let mut recorded = 0;
        for words in &clauses {
            let mut earlier: Option<&str> = None;
            let mut previous: Option<&str> = None;
            for word in words {
                // 与上屏一样按一次普通事件记：文本里出现一次抬不动首选，出现两次才起作用
                self.learner.record_transition(
                    Context { previous, earlier },
                    word,
                    TRANSITION_WEIGHT,
                );
                recorded += 1;
                earlier = previous;
                previous = Some(word.as_str());
            }
        }
        recorded
    }
}
