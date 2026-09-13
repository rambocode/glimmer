//! 一种字体用法：字号与行高（点）。字族不在这里定，由字体库按平台给界面字体。

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontSpec {
    /// 字号。
    pub size: f32,

    /// 行高：一行文字占的高度，字形在其中垂直居中。
    pub line_height: f32,
}

impl FontSpec {
    pub const fn new(size: f32, line_height: f32) -> Self {
        Self { size, line_height }
    }

    /// 点 → 像素。
    pub(crate) fn scaled(self, scale: f32) -> Self {
        Self {
            size: self.size * scale,
            line_height: self.line_height * scale,
        }
    }
}
