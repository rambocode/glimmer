//! 应用光标前文：壳读来的「光标前面已经有的文字」，是整句与词级排序的上文来源之一。
//!
//! 上文有两个来源，规则只有一条：**上屏链上有词就用链，链是空的才用应用前文。**
//!
//! - 链上有词，说明光标就在我们刚上屏的词后面，那个词是用户亲手选的，比从文本切出来的准（还带音节）；
//!   而且壳只在一段组句起头读一次前文，这段里再上屏的词不在前文里，用前文反而会把刚选的词丢掉。
//! - 链是空的（刚切应用、挪过光标、标点断开、会话刚开始、前文是别人发来的话或粘贴进来的），
//!   以前只能当句首算，现在按应用前文末尾那一小句切出最后两个词接上去。
//! - 壳给不出前文（读不到、私密输入框、平台不支持）时还是句首。
//!
//! 前文同时也是神经重打分给模型看的上文（[`Engine::rescoring_context`]），那一侧按字看、不切词。

mod before;

use super::Engine;
use crate::sentence::Context;

pub(super) use before::SurroundingBefore;

impl Engine {
    /// 壳告知应用里光标前的文本（一段组句起头读一次；读不到、私密输入框给 `None`）。
    /// 长度由壳按 [`super::RESCORE_CONTEXT_CHARS`] 截；这里只在文本真变了时才重新切词。
    pub fn set_surrounding_before(&mut self, before: Option<String>) {
        match before {
            // 壳每段组句都送，多半还是同一段文字：同样的文本不用再切一遍词
            Some(text) if self.surrounding.as_ref().is_some_and(|s| s.text() == text) => {}
            Some(text) => {
                let parsed = SurroundingBefore::new(text, &*self.language_model);
                self.surrounding = Some(parsed);
            }
            None => self.surrounding = None,
        }
    }

    /// 壳给的应用光标前文原文；没给过为 `None`。
    pub fn surrounding_before(&self) -> Option<&str> {
        self.surrounding.as_ref().map(SurroundingBefore::text)
    }

    /// 下一个词的上文，整句转换的第一个词与词级排序都用它。取舍规则见本模块文件头。
    pub(super) fn context(&self) -> Context<'_> {
        if self.chain.previous().is_some() {
            return self.chain.context();
        }
        self.surrounding
            .as_ref()
            .map_or(Context::START, SurroundingBefore::context)
    }
}
