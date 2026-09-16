//! IBus 的修饰键状态位（`state`，对照 ibus `src/ibustypes.h` 的 `IBusModifierType`）→ 协议的 [`KeyModifiers`]。

use glimmer_platform::protocol::KeyModifiers;

/// Shift 按着。
pub const SHIFT_MASK: u32 = 1 << 0;

/// Caps Lock 亮着。
pub const LOCK_MASK: u32 = 1 << 1;

/// Ctrl 按着。
pub const CONTROL_MASK: u32 = 1 << 2;

/// Alt（`Mod1`）按着。
pub const MOD1_MASK: u32 = 1 << 3;

/// `Mod4`：多数 X 键位表里 Super 落在这一位。
pub const MOD4_MASK: u32 = 1 << 6;

/// Super 按着（XKB 虚拟修饰键，ibus 会补上）。
pub const SUPER_MASK: u32 = 1 << 26;

/// 这是一次释放事件。
pub const RELEASE_MASK: u32 = 1 << 30;

/// 由 IBus 的 `state` 与前端记的中英模式拼出协议修饰键。
pub fn key_modifiers(state: u32, english_mode: bool) -> KeyModifiers {
    KeyModifiers {
        ctrl: state & CONTROL_MASK != 0,
        shift: state & SHIFT_MASK != 0,
        alt: state & MOD1_MASK != 0,
        win: state & (SUPER_MASK | MOD4_MASK) != 0,
        caps: state & LOCK_MASK != 0,
        english_mode,
    }
}

/// `state` 里带不带释放位。
pub fn is_release(state: u32) -> bool {
    state & RELEASE_MASK != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_mask_bit() {
        let modifiers = key_modifiers(
            SHIFT_MASK | LOCK_MASK | CONTROL_MASK | MOD1_MASK | SUPER_MASK,
            true,
        );
        assert!(modifiers.shift && modifiers.caps && modifiers.ctrl && modifiers.alt);
        assert!(modifiers.win && modifiers.english_mode);
        assert!(key_modifiers(MOD4_MASK, false).win);
        assert_eq!(key_modifiers(0, false), KeyModifiers::default());
    }

    #[test]
    fn release_bit() {
        assert!(is_release(RELEASE_MASK | SHIFT_MASK));
        assert!(!is_release(SHIFT_MASK));
    }
}
