//! 候选窗口顶部的拼音行：若干段 + 渲染器自己画的光标。

mod segment;
mod style;

pub use segment::PreeditSegment;
pub use style::PreeditStyle;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preedit {
    /// 按顺序画的片段。
    pub segments: Vec<PreeditSegment>,

    /// 光标在拼接文本里的字符位置。
    pub cursor: usize,
}

impl Preedit {
    /// 单段普通文本。
    pub fn plain(text: &str, cursor: usize) -> Self {
        Self {
            segments: vec![PreeditSegment {
                text: text.to_owned(),
                style: PreeditStyle::Typed,
            }],
            cursor,
        }
    }

    /// 拼接后的全文。
    pub fn text(&self) -> String {
        self.segments.iter().map(|s| s.text.as_str()).collect()
    }

    /// 光标前的文字（用来量光标的 x）。
    pub fn before_cursor(&self) -> String {
        self.text().chars().take(self.cursor).collect()
    }
}
