//! annotation 片段的深浅。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 译文。
    Gloss,

    /// 生词的译文（用户还没在候选里见过几轮），用强调色。
    Fresh,

    /// 词性与分隔符，最浅。
    Faint,
}
