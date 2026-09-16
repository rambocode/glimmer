//! X11 keysym（IBus 送来的 `keyval`）常数与「keysym → Unicode 字符」的换算。
//!
//! 换算规则照 X11 协议附录与 `keysymdef.h`：`0x20..=0x7e`、`0xa0..=0xff` 两段 Latin-1 的 keysym 就是码位本身；
//! `0x0100_0100..=0x0110_ffff` 是 `0x0100_0000 | UCS` 的直接 Unicode 形式；小键盘可打印键单独查表。
//! 其余历史编码（`0x01a1` 起的 Latin-2 / 西里尔等旧 keysym）目前不换算，拼音输入用不到。

/// 空格。
pub const SPACE: u32 = 0x0020;

/// 退格。
pub const BACKSPACE: u32 = 0xff08;

/// Tab。
pub const TAB: u32 = 0xff09;

/// 回车。
pub const RETURN: u32 = 0xff0d;

/// Esc。
pub const ESCAPE: u32 = 0xff1b;

/// Delete。
pub const DELETE: u32 = 0xffff;

/// Home。
pub const HOME: u32 = 0xff50;

/// 左方向键。
pub const LEFT: u32 = 0xff51;

/// 上方向键。
pub const UP: u32 = 0xff52;

/// 右方向键。
pub const RIGHT: u32 = 0xff53;

/// 下方向键。
pub const DOWN: u32 = 0xff54;

/// PageUp（`Prior`）。
pub const PAGE_UP: u32 = 0xff55;

/// PageDown（`Next`）。
pub const PAGE_DOWN: u32 = 0xff56;

/// End。
pub const END: u32 = 0xff57;

/// Insert。
pub const INSERT: u32 = 0xff63;

/// Shift+Tab 产生的 keysym。
pub const ISO_LEFT_TAB: u32 = 0xfe20;

/// 左 Shift。
pub const SHIFT_L: u32 = 0xffe1;

/// 右 Shift。
pub const SHIFT_R: u32 = 0xffe2;

/// 左 Ctrl。
pub const CONTROL_L: u32 = 0xffe3;

/// 右 Ctrl。
pub const CONTROL_R: u32 = 0xffe4;

/// Caps Lock。
pub const CAPS_LOCK: u32 = 0xffe5;

/// 左 Alt。
pub const ALT_L: u32 = 0xffe9;

/// 右 Alt。
pub const ALT_R: u32 = 0xffea;

/// 左 Super（Win 键）。
pub const SUPER_L: u32 = 0xffeb;

/// 右 Super。
pub const SUPER_R: u32 = 0xffec;

/// 小键盘回车。
pub const KP_ENTER: u32 = 0xff8d;

/// 小键盘 0，`KP_1..KP_9` 依次加一。
pub const KP_0: u32 = 0xffb0;

/// 小键盘 9。
pub const KP_9: u32 = 0xffb9;

/// F1，F2..F12 依次加一。
pub const F1: u32 = 0xffbe;

/// F12。
pub const F12: u32 = 0xffc9;

/// 是不是 Shift 键本身（左右都算）。
pub fn is_shift(keyval: u32) -> bool {
    keyval == SHIFT_L || keyval == SHIFT_R
}

/// keysym 可打印时对应的 Unicode 字符；功能键、修饰键与控制字符返回 `None`。
pub fn to_char(keyval: u32) -> Option<char> {
    let code = match keyval {
        0x20..=0x7e | 0xa0..=0xff => keyval,
        0x0100_0100..=0x0110_ffff => keyval - 0x0100_0000,
        _ => return keypad_char(keyval),
    };
    char::from_u32(code).filter(|c| !c.is_control())
}

/// 小键盘上的可打印键（NumLock 亮着时 X 送的是这些 keysym）。
fn keypad_char(keyval: u32) -> Option<char> {
    let c = match keyval {
        0xff80 => ' ',
        0xffaa => '*',
        0xffab => '+',
        0xffac => ',',
        0xffad => '-',
        0xffae => '.',
        0xffaf => '/',
        0xffbd => '=',
        KP_0..=KP_9 => char::from(b'0' + (keyval - KP_0) as u8),
        _ => return None,
    };
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin1_keysyms_are_their_code_points() {
        assert_eq!(to_char(0x61), Some('a'));
        assert_eq!(to_char(0x41), Some('A'));
        assert_eq!(to_char(0x3b), Some(';'));
        assert_eq!(to_char(0xe9), Some('é'));
    }

    #[test]
    fn unicode_keysyms_strip_the_marker_bit() {
        assert_eq!(to_char(0x0100_4f60), Some('你'));
        assert_eq!(to_char(0x0100_20ac), Some('€'));
    }

    #[test]
    fn keypad_keysyms_map_to_their_characters() {
        assert_eq!(to_char(KP_0 + 7), Some('7'));
        assert_eq!(to_char(0xffae), Some('.'));
    }

    #[test]
    fn function_keys_have_no_character() {
        for keyval in [BACKSPACE, RETURN, ESCAPE, SHIFT_L, PAGE_UP, F1] {
            assert_eq!(to_char(keyval), None, "{keyval:#x}");
        }
    }
}
