//! 配置文件 `[wubi]` 分节：五笔的行为选项，方案本身在 `[general] wubi` 里选。

use glimmer_core::wubi::Options;
use serde::{Deserialize, Serialize};

/// `[wubi]` 分节。缺省值与 [`Options::default`] 一致（fcitx5 / librime 的共同缺省）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WubiConfig {
    /// 敲满四码且有全码命中时首选自动上屏。
    pub auto_select: bool,

    /// 逐键提示候选右侧显示完整编码。
    pub hint: bool,

    /// 不超过这么多码时候选只按码表顺序，不按用户词频重排（简码位置固定）。
    pub fixed_order_length: usize,

    /// 整句输入：连着打编码不按空格，引擎切词出整句；开着时不再四码自动上屏与顶字。
    pub sentence: bool,
}

impl Default for WubiConfig {
    fn default() -> Self {
        Self::from(Options::default())
    }
}

impl From<Options> for WubiConfig {
    fn from(options: Options) -> Self {
        Self {
            auto_select: options.auto_select,
            hint: options.hint,
            fixed_order_length: options.fixed_order_length,
            sentence: options.sentence,
        }
    }
}

impl WubiConfig {
    /// 交给 Core 的选项。
    pub fn options(&self) -> Options {
        Options {
            auto_select: self.auto_select,
            hint: self.hint,
            fixed_order_length: self.fixed_order_length,
            sentence: self.sentence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_core_and_round_trip() {
        let config = WubiConfig::default();
        assert_eq!(config.options(), Options::default());
        let parsed: WubiConfig =
            toml::from_str("auto_select = false\nfixed_order_length = 3\n").unwrap();
        assert!(!parsed.auto_select && parsed.hint);
        assert_eq!(parsed.options().fixed_order_length, 3);
        assert!(!parsed.sentence, "整句输入缺省关");
        let on: WubiConfig = toml::from_str("sentence = true\n").unwrap();
        assert!(on.options().sentence);
    }
}
