//! 到达一个节点的一种走法。

/// 到达某个节点的第几好的走法：从哪个前驱的第几好的走法接过来，接完累计多少分。
///
/// 只要最优一条时每个节点只有一种走法，就是普通的 Viterbi 回指。要前 k 条时每个节点留前 k 种，
/// 回溯出来的才是真的前 k 条路径——只留最优回指的话，「前 k 条」只是 k 个不同的句尾词各自的最优链，
/// 句子中间全都一样，重打分救不了中间的错。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Arrival {
    /// 到此为止的累计得分。
    pub score: f64,

    /// 累计得分里静态模型的部分（见 `Conversion::static_score`）。
    pub static_score: f64,

    /// 到此为止路径上模糊音 / 敲错变体的代价之和（已从 `score` 里扣掉，另记一份给调用方判断路径是不是原样）。
    pub penalty: f64,

    /// 前驱在 `nodes[start]` 里的下标；`start == 0` 时无意义。
    pub back: usize,

    /// 接的是前驱的第几种走法。
    pub back_rank: usize,
}

impl Arrival {
    /// 虚拟起点。
    pub const ORIGIN: Self = Self {
        score: 0.0,
        static_score: 0.0,
        penalty: 0.0,
        back: 0,
        back_rank: 0,
    };
}
