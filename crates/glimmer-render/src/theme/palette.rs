//! 一套配色。缺省两套取自 macOS 系统语义色在 sRGB 下的实测值（label 0.847、secondaryLabel 0.498…）。

use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// 候选词。
    pub text: Color,

    /// 译文。
    pub gloss: Color,

    /// 词性，比译文更浅。
    pub pos: Color,

    /// 生词译文：比普通译文醒目，看熟了就回到译文色。
    pub fresh: Color,

    /// 序号。
    pub index: Color,

    /// 云联想的云朵与文字：比译文醒目一点，但不抢候选词。
    pub cloud: Color,

    /// 窗口背景。
    pub background: Color,

    /// 当前候选的高亮底色。
    pub highlight: Color,
}

impl Palette {
    pub const fn light() -> Self {
        Self {
            text: Color::gray(0, 216),
            gloss: Color::gray(0, 127),
            pos: Color::gray(0, 66),
            fresh: Color::rgb(255, 141, 40),
            index: Color::gray(0, 66),
            cloud: Color::rgb(0, 195, 208),
            background: Color::rgb(255, 255, 255),
            highlight: Color::rgba(176, 206, 125, 127),
        }
    }

    pub const fn dark() -> Self {
        Self {
            text: Color::gray(255, 216),
            gloss: Color::gray(255, 140),
            pos: Color::gray(255, 63),
            fresh: Color::rgb(255, 146, 48),
            index: Color::gray(255, 63),
            cloud: Color::rgb(0, 210, 224),
            background: Color::rgb(30, 30, 30),
            highlight: Color::rgba(36, 76, 36, 255),
        }
    }
}
