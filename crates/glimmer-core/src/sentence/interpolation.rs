use super::{
    CONFIDENCE_K, INITIAL_CONTEXT_WEIGHT, MAX_CONFIDENCE, SPAN_CANDIDATES, TRIGRAM_DISCOUNT,
    USER_LAMBDA, WORD_PENALTY,
};

/// 个人 n-gram 与静态模型插值的参数（见 [`UserNgram::blend`]），外加整句路径打分的两个常数（开头接上文的权重、每词代价）。缺省值是 `sentence` 模块里的常数，
/// 回放调参（`glimmer-cli --tune`）时可以整组换掉，引擎与壳只用缺省值。
///
/// [`UserNgram::blend`]: super::UserNgram::blend
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interpolation {
    /// 个人概率里 bigram 部分的权重，其余给个人一元（[`USER_LAMBDA`]）。
    pub lambda: f64,

    /// 插值权重 μ = c(v)/(c(v)+K) 里的 K（[`CONFIDENCE_K`]）。
    pub confidence_k: f64,

    /// 插值权重的封顶（[`MAX_CONFIDENCE`]）。
    pub max_confidence: f64,

    /// 个人三元的绝对折扣 D（[`TRIGRAM_DISCOUNT`]）。
    pub trigram_discount: f64,

    /// 整句第一个词按上文算的成分（[`INITIAL_CONTEXT_WEIGHT`]），0 为一律当句首。
    pub initial_weight: f64,

    /// 整句路径上每个词扣的分（[`WORD_PENALTY`]）。
    pub word_penalty: f64,

    /// 词图每个格子最多留几个词（[`SPAN_CANDIDATES`]）。放宽它是打分模型可信度的验收尺子，所以挂在这里能调。
    pub span_candidates: usize,
}

impl Interpolation {
    /// 现在的常数。
    pub const DEFAULT: Self = Self {
        lambda: USER_LAMBDA,
        confidence_k: CONFIDENCE_K,
        max_confidence: MAX_CONFIDENCE,
        trigram_discount: TRIGRAM_DISCOUNT,
        initial_weight: INITIAL_CONTEXT_WEIGHT,
        word_penalty: WORD_PENALTY,
        span_candidates: SPAN_CANDIDATES,
    };
}

impl Default for Interpolation {
    fn default() -> Self {
        Self::DEFAULT
    }
}
