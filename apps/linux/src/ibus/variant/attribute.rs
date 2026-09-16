//! `IBusAttribute`：文字的一段样式（`src/ibusattribute.c` 的序列化：`type`、`value`、`start_index`、`end_index` 四个 `u`）。

use zbus::zvariant::Value;

use super::serializable;

/// 一段样式，下标按字符（`char`）数算，左闭右开。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attribute {
    /// `IBusAttrType`：1 下划线、2 前景色、3 背景色。
    pub kind: u32,

    /// 取值：下划线时是 `IBusAttrUnderline`，颜色时是 `0xRRGGBB`。
    pub value: u32,

    /// 起始字符下标（含）。
    pub start: u32,

    /// 结束字符下标（不含）。
    pub end: u32,
}

impl Attribute {
    /// `IBUS_ATTR_TYPE_UNDERLINE`。
    const UNDERLINE: u32 = 1;

    /// `IBUS_ATTR_TYPE_FOREGROUND`。
    const FOREGROUND: u32 = 2;

    /// `IBUS_ATTR_UNDERLINE_SINGLE`。
    const UNDERLINE_SINGLE: u32 = 1;

    /// 单下划线。
    pub fn underline(start: usize, end: usize) -> Self {
        Self::new(Self::UNDERLINE, Self::UNDERLINE_SINGLE, start, end)
    }

    /// 前景色（`0xRRGGBB`）。
    pub fn foreground(rgb: u32, start: usize, end: usize) -> Self {
        Self::new(Self::FOREGROUND, rgb, start, end)
    }

    /// 按字符下标构造；超过 `u32` 的长度在输入法里不会出现，截断即可。
    fn new(kind: u32, value: u32, start: usize, end: usize) -> Self {
        Self {
            kind,
            value,
            start: start as u32,
            end: end as u32,
        }
    }

    /// 编码成 `("IBusAttribute", {}, type, value, start, end)`。
    pub fn to_value(self) -> Value<'static> {
        serializable(
            "IBusAttribute",
            vec![
                Value::U32(self.kind),
                Value::U32(self.value),
                Value::U32(self.start),
                Value::U32(self.end),
            ],
        )
    }
}
