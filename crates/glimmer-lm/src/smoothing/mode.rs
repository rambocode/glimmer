//! 回退方式。

/// 静态模型怎么把三元 / 二元 / 一元三层混起来。扫参用（`glimmer-cli --tune lm-mode=`），
/// 缺省见 [`Smoothing::DEFAULT`]。
///
/// [`Smoothing::DEFAULT`]: super::Smoothing::DEFAULT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffMode {
    /// 老做法：P(w|v) = λ·c(v,w)/c(v) + (1−λ)·c(w)/N，λ 固定，不看三元。
    /// 留着是为了在同一个二进制上复现改之前的数字。
    Legacy,

    /// 绝对折扣 + 按上文定的回退权重，一元用 c(w)/N。
    Absolute,

    /// 同 [`Self::Absolute`]，但一元换成接续概率 N₁₊(·,w)/N₁₊(·,·)（Kneser-Ney 的下层）：
    /// 只在固定搭配里出现的词（旧金山 的 金山）不该因为总次数高就到处都能接。
    Continuation,
}

impl BackoffMode {
    /// 扫参用的编号：0 老做法、1 绝对折扣、2 绝对折扣 + 接续概率。别的数当 2。
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Legacy,
            1 => Self::Absolute,
            _ => Self::Continuation,
        }
    }
}
