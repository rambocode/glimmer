//! 拼音侧方案：全拼 / 双拼六套 / 大千注音 / 关。配置项 `[general] scheme` 的值。
//!
//! 「输入方案是配置项，不是模式」：中英切换始终是布尔，换方案不改变别的方案的既定按键行为。
//!
//! **拼音与形码是两条独立的轴**：这里只管拼音侧（读法），形码侧（五笔版本）看 `[general] wubi`。
//! 两边都开就是混输，单选一个方案表达不了，所以拆成两项。

use std::fmt;
use std::str::FromStr;

use glimmer_core::{ShuangpinScheme, WubiVariant};

/// 拼音侧方案。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scheme {
    /// 全拼。
    #[default]
    Pinyin,

    /// 双拼，六套键位见 [`ShuangpinScheme`]。
    Shuangpin(ShuangpinScheme),

    /// 大千注音。
    Zhuyin,

    /// 关：拼音侧不参与查询，只用五笔。五笔也关着的话按全拼（不然一个候选都没有）。
    Off,
}

impl Scheme {
    /// 全部方案，设置界面与状态条按这个顺序列。
    pub const ALL: [Self; 9] = [
        Self::Pinyin,
        Self::Shuangpin(ShuangpinScheme::Xiaohe),
        Self::Shuangpin(ShuangpinScheme::Ziranma),
        Self::Shuangpin(ShuangpinScheme::Microsoft),
        Self::Shuangpin(ShuangpinScheme::Sogou),
        Self::Shuangpin(ShuangpinScheme::Xiaolang),
        Self::Shuangpin(ShuangpinScheme::Abc),
        Self::Zhuyin,
        Self::Off,
    ];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Pinyin => "pinyin",
            Self::Shuangpin(scheme) => scheme.key(),
            Self::Zhuyin => "zhuyin",
            Self::Off => "none",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Pinyin => "全拼",
            Self::Shuangpin(scheme) => scheme.label(),
            Self::Zhuyin => "大千注音",
            Self::Off => "关（只用五笔）",
        }
    }

    /// 这套方案是双拼时是哪一套；装配引擎用（不是双拼时为 `None`）。
    pub fn shuangpin(self) -> Option<ShuangpinScheme> {
        match self {
            Self::Shuangpin(scheme) => Some(scheme),
            _ => None,
        }
    }

    /// 拼音侧参不参与查询。
    pub fn is_on(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// 日志里的写法：全拼为空串（老日志里没有这个字段就是全拼），其余同 [`Self::key`]。
    pub fn log_key(self) -> &'static str {
        match self {
            Self::Pinyin => "",
            other => other.key(),
        }
    }
}

/// 状态条上显示的方案名，五笔在前（与候选顺序一致）；全拼且不开五笔时为空串。
/// 全拼只在同时开着五笔时才写出来——单开全拼是缺省，标它没意义。
///
/// 做成跟 [`Scheme`] 与五笔版本一起算的自由函数，而不是存进配置结构：
/// 存下来的话它会与那两项冗余，手搓配置的地方就会漂移。
pub fn scheme_label(pinyin: Scheme, wubi: Option<WubiVariant>) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(variant) = wubi {
        parts.push(variant.label());
    }
    if pinyin.is_on() && (wubi.is_some() || pinyin != Scheme::Pinyin) {
        parts.push(pinyin.label());
    }
    parts.join(" + ")
}

impl FromStr for Scheme {
    type Err = String;

    /// 认不出来的写法报错，由调用方决定退回什么（配置层退回全拼并警告）。
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        match text {
            "" | "pinyin" => Ok(Self::Pinyin),
            "zhuyin" => Ok(Self::Zhuyin),
            "none" | "off" => Ok(Self::Off),
            // 旧写法把五笔写在这一栏，等价于「拼音关」；版本本身由 `[general] wubi` 那条轴认
            _ if text.parse::<WubiVariant>().is_ok() => Ok(Self::Off),
            other => other
                .parse()
                .map(Self::Shuangpin)
                .map_err(|_| other.to_owned()),
        }
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_key_it_prints() {
        for scheme in Scheme::ALL {
            assert_eq!(scheme.key().parse::<Scheme>(), Ok(scheme), "{scheme}");
        }
    }

    #[test]
    fn empty_and_unknown_spellings() {
        assert_eq!("".parse::<Scheme>(), Ok(Scheme::Pinyin));
        assert_eq!(" pinyin ".parse::<Scheme>(), Ok(Scheme::Pinyin));
        assert_eq!("none".parse::<Scheme>(), Ok(Scheme::Off));
        assert_eq!("off".parse::<Scheme>(), Ok(Scheme::Off));
        assert!("flypy".parse::<Scheme>().is_err());
    }

    #[test]
    fn the_old_wubi_spelling_means_turn_the_pinyin_side_off() {
        // 曾经把五笔写在 `scheme` 里（单选的方案），那时它等价于「拼音关 + 五笔开」
        for key in ["wubi86", "86", "98", "xsj"] {
            assert_eq!(key.parse::<Scheme>(), Ok(Scheme::Off), "{key}");
        }
        assert!(!Scheme::Off.is_on());
        assert!(Scheme::Pinyin.is_on());
    }

    #[test]
    fn the_label_puts_wubi_first_and_hides_the_bare_default() {
        assert_eq!(scheme_label(Scheme::Pinyin, None), "");
        assert_eq!(
            scheme_label(Scheme::Shuangpin(ShuangpinScheme::Xiaohe), None),
            "小鹤双拼"
        );
        assert_eq!(
            scheme_label(Scheme::Off, Some(WubiVariant::Wubi86)),
            "86 五笔"
        );
        assert_eq!(
            scheme_label(Scheme::Pinyin, Some(WubiVariant::Wubi98)),
            "98 五笔 + 全拼"
        );
        assert_eq!(
            scheme_label(
                Scheme::Shuangpin(ShuangpinScheme::Xiaohe),
                Some(WubiVariant::Xinshiji)
            ),
            "新世纪五笔 + 小鹤双拼"
        );
    }
}
