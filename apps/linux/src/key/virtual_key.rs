//! X11 keysym → Windows 虚拟键码（`VK_*`）。协议按 Windows 语义定义按键，Router 靠 VK 认退格 / 翻页 / 方向键，
//! 靠「主键盘区数字键」的 VK 认修饰键 + 数字快捷键，所以 Linux 前端先翻成 VK 再送。
//!
//! 美式布局下 keysym 已经带上 Shift 的结果（`Shift+1` 送 `exclam`），这里按键位反推回同一个 VK，与 Windows 一致。

use super::keysym;

/// 主键盘区 OEM 标点键：`(不按 Shift 的字符, 按 Shift 的字符, VK)`，与 tsf 的 `resolve_char` 同表。
const OEM_KEYS: [(char, char, u32); 11] = [
    (';', ':', 0xBA),
    ('=', '+', 0xBB),
    (',', '<', 0xBC),
    ('-', '_', 0xBD),
    ('.', '>', 0xBE),
    ('/', '?', 0xBF),
    ('`', '~', 0xC0),
    ('[', '{', 0xDB),
    ('\\', '|', 0xDC),
    (']', '}', 0xDD),
    ('\'', '"', 0xDE),
];

/// 数字键 0–9 按住 Shift 出的符号，下标就是数字。
const SHIFTED_DIGITS: &[u8; 10] = b")!@#$%^&*(";

/// keysym 对应的 VK；不认识的键（多媒体键、死键、非美式布局的字母等）返回 `None`，前端直接放行。
pub fn virtual_key(keyval: u32) -> Option<u32> {
    if let Some(vk) = function_key(keyval) {
        return Some(vk);
    }
    let c = char::from_u32(keyval).filter(|_| keyval < 0x80)?;
    if c.is_ascii_alphabetic() {
        return Some(u32::from(c.to_ascii_uppercase() as u8));
    }
    if c.is_ascii_digit() {
        return Some(u32::from(c as u8));
    }
    if c == ' ' {
        return Some(0x20);
    }
    if let Some(index) = SHIFTED_DIGITS.iter().position(|&b| char::from(b) == c) {
        return Some(0x30 + index as u32);
    }
    OEM_KEYS
        .iter()
        .find(|(plain, shifted, _)| *plain == c || *shifted == c)
        .map(|&(_, _, vk)| vk)
}

/// 功能键、修饰键与小键盘。小键盘在 NumLock 灭时送 `KP_Home` 这类 keysym，按对应的导航键算。
fn function_key(keyval: u32) -> Option<u32> {
    let vk = match keyval {
        keysym::BACKSPACE => 0x08,
        keysym::TAB | keysym::ISO_LEFT_TAB => 0x09,
        keysym::RETURN | keysym::KP_ENTER => 0x0D,
        keysym::ESCAPE => 0x1B,
        keysym::PAGE_UP | 0xff9a => 0x21,
        keysym::PAGE_DOWN | 0xff9b => 0x22,
        keysym::END | 0xff9c => 0x23,
        keysym::HOME | 0xff95 => 0x24,
        keysym::LEFT | 0xff96 => 0x25,
        keysym::UP | 0xff97 => 0x26,
        keysym::RIGHT | 0xff98 => 0x27,
        keysym::DOWN | 0xff99 => 0x28,
        keysym::INSERT | 0xff9e => 0x2D,
        keysym::DELETE | 0xff9f => 0x2E,
        0xff80 => 0x20,
        keysym::KP_0..=keysym::KP_9 => 0x60 + (keyval - keysym::KP_0),
        0xffaa => 0x6A,
        0xffab => 0x6B,
        0xffac => 0x6C,
        0xffad => 0x6D,
        0xffae => 0x6E,
        0xffaf => 0x6F,
        keysym::F1..=keysym::F12 => 0x70 + (keyval - keysym::F1),
        keysym::SHIFT_L => 0xA0,
        keysym::SHIFT_R => 0xA1,
        keysym::CONTROL_L => 0xA2,
        keysym::CONTROL_R => 0xA3,
        keysym::ALT_L => 0xA4,
        keysym::ALT_R => 0xA5,
        keysym::SUPER_L => 0x5B,
        keysym::SUPER_R => 0x5C,
        keysym::CAPS_LOCK => 0x14,
        _ => return None,
    };
    Some(vk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_map_to_uppercase_codes_regardless_of_case() {
        assert_eq!(virtual_key(u32::from('a')), Some(0x41));
        assert_eq!(virtual_key(u32::from('Z')), Some(0x5A));
    }

    #[test]
    fn shifted_digits_map_back_to_the_digit_key() {
        assert_eq!(virtual_key(u32::from('1')), Some(0x31));
        assert_eq!(virtual_key(u32::from('!')), Some(0x31));
        assert_eq!(virtual_key(u32::from(')')), Some(0x30));
    }

    #[test]
    fn punctuation_maps_to_oem_keys() {
        assert_eq!(virtual_key(u32::from(';')), Some(0xBA));
        assert_eq!(virtual_key(u32::from(':')), Some(0xBA));
        assert_eq!(virtual_key(u32::from('"')), Some(0xDE));
        assert_eq!(virtual_key(u32::from('?')), Some(0xBF));
    }

    #[test]
    fn function_and_keypad_keys() {
        assert_eq!(virtual_key(keysym::BACKSPACE), Some(0x08));
        assert_eq!(virtual_key(keysym::ISO_LEFT_TAB), Some(0x09));
        assert_eq!(virtual_key(keysym::KP_ENTER), Some(0x0D));
        assert_eq!(virtual_key(keysym::PAGE_DOWN), Some(0x22));
        assert_eq!(virtual_key(keysym::KP_0 + 3), Some(0x63));
        assert_eq!(virtual_key(keysym::SPACE), Some(0x20));
        assert_eq!(virtual_key(keysym::SHIFT_R), Some(0xA1));
    }

    #[test]
    fn unknown_keys_are_none() {
        assert_eq!(virtual_key(0x1008ff13), None);
        assert_eq!(virtual_key(0x0100_4f60), None);
    }
}
