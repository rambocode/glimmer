//! 单击 Shift 的判定（对应 tsf 的 `ShiftTap`）：按下 Shift 到释放之间没有插进别的键，就是一次单击，切中英模式。

use super::keysym::is_shift;

/// 单击 Shift 的状态机。IBus 把按下与释放都送给引擎（释放带 `RELEASE_MASK`），两头各喂一次。
#[derive(Debug, Default)]
pub struct ShiftTap {
    /// 按下 Shift 后还没有别的键插进来。
    alone: bool,

    /// Shift 正按着：自动重复的按下（IBus 不带重复标记）靠它认出来，不重新开始一次单击。
    held: bool,
}

impl ShiftTap {
    /// 任一键按下：Shift 开始一次候选单击，别的键打断它。Shift 自动重复再按下不影响。
    pub fn press(&mut self, keyval: u32) {
        if !is_shift(keyval) {
            self.alone = false;
        } else if !self.held {
            self.held = true;
            self.alone = true;
        }
    }

    /// 任一键释放；Shift 单独按下又释放时返回 `true`，一次释放只算一次。
    pub fn release(&mut self, keyval: u32) -> bool {
        if !is_shift(keyval) {
            return false;
        }
        self.held = false;
        std::mem::take(&mut self.alone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::keysym::{SHIFT_L, SHIFT_R};

    #[test]
    fn lone_shift_is_a_tap() {
        let mut tap = ShiftTap::default();
        tap.press(SHIFT_L);
        assert!(tap.release(SHIFT_L));
        assert!(!tap.release(SHIFT_L));
    }

    #[test]
    fn shift_with_another_key_is_not_a_tap() {
        let mut tap = ShiftTap::default();
        tap.press(SHIFT_R);
        tap.press(u32::from('A'));
        assert!(!tap.release(u32::from('A')));
        assert!(!tap.release(SHIFT_R));
    }

    #[test]
    fn auto_repeated_shift_press_does_not_restart_a_tap() {
        let mut tap = ShiftTap::default();
        tap.press(SHIFT_L);
        tap.press(u32::from('A'));
        tap.press(SHIFT_L);
        assert!(!tap.release(SHIFT_L));
    }

    #[test]
    fn release_of_other_key_does_not_count() {
        let mut tap = ShiftTap::default();
        tap.press(SHIFT_L);
        assert!(!tap.release(u32::from('a')));
        assert!(tap.release(SHIFT_L));
    }
}
