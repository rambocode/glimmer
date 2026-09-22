/// 词图一个格子里的一个词：从词库命中里拷出来、不再借用词库的形式，能放进 [`super::SpanCache`]。
#[derive(Debug, Clone, PartialEq)]
pub struct SpanWord {
    /// 词。
    pub text: String,

    /// 词的音节。
    pub syllables: Vec<String>,

    /// 词库静态词频。
    pub frequency: u32,

    /// 命中的音节不是敲的原样（模糊音 / 敲错变体）时的代价之和，进路径得分时扣掉；原样命中是 0。
    pub penalty: f64,

    /// 多音字按这个读音命中要扣的分（见 [`crate::sentence::ReadingShare`]），已按系数与封顶算好；单读音词是 0。
    /// 与 `penalty` 分开存：那个是「猜用户敲错了」的代价，牵着原样成词保护，而这个是读音本身的概率份额，两者不能混。
    pub reading: f64,
}
