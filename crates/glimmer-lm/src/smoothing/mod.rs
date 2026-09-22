//! 回退平滑的参数。

mod mode;

pub use mode::BackoffMode;

/// 静态语言模型的平滑参数：回退方式与两层的绝对折扣。
///
/// 折扣 D 的意思是「每条见过的接续让出 D 份计数给回退」：上文见得越少、不同后继越多，让出去的比例越大，
/// 于是「这个上文只见过一两次」的接续不会把没见过的后继压死。D 取 0 就退回不打折的最大似然 + 剩余质量回退。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Smoothing {
    /// 三层怎么混。
    pub mode: BackoffMode,

    /// 三元的绝对折扣 D₃。
    pub trigram_discount: f64,

    /// 二元的绝对折扣 D₂。
    pub bigram_discount: f64,

    /// 用不用三元那一层。关掉就是「只换回退方式的二元模型」，同一份 `.qj` 上能量出三元自己的净效果。
    pub use_trigram: bool,
}

impl Smoothing {
    /// 缺省：绝对折扣 + 接续概率一元，D₃ = D₂ = 3。
    ///
    /// n-gram 平滑的惯用值是 0.75，这里取 3 是扫出来的：0.75 到 4 是一块平台，2 以上都一样
    /// （干净集首选 28.9% → 29.2%，见 `docs/notes/language-model.md`）。表按次数砍过（二元只留次数 ≥ 3 的），
    /// 留下来的那些本来就偏高，扣得多一点才不会把没进表的接续压死。
    /// **D 扣的是绝对计数，换一份大小差一个数量级的语料要重扫。**
    pub const DEFAULT: Self = Self {
        mode: BackoffMode::Continuation,
        trigram_discount: 3.0,
        bigram_discount: 3.0,
        use_trigram: true,
    };

    /// 改成三元之前的行为：固定 λ = 0.8 的插值，不看三元。
    pub const LEGACY: Self = Self {
        mode: BackoffMode::Legacy,
        ..Self::DEFAULT
    };
}

impl Default for Smoothing {
    fn default() -> Self {
        Self::DEFAULT
    }
}
