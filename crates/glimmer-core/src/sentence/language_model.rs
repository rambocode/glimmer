use super::Context;

/// 整句转换的打分来源：词级语言模型。实现放兄弟 crate（`glimmer-lm`），Core 只认这个 trait。
pub trait LanguageModel: Send {
    /// `log P(word | context)`；`context` 给前一个词与再前一个词（[`Context::START`] 是句首）。
    /// 模型不认识 `word` 时返回 `None`，由 Core 用词库词频兜底。
    ///
    /// 实现可以只看 `context.previous`（二元），看前二词的（三元）自己按上下文回退。
    fn log_prob(&self, context: Context<'_>, word: &str) -> Option<f64>;
}

/// 没接语言模型：一律兜底，整句转换退化为一元词频。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoLanguageModel;

impl LanguageModel for NoLanguageModel {
    fn log_prob(&self, _context: Context<'_>, _word: &str) -> Option<f64> {
        None
    }
}
