//! 五笔的行为选项，配置 `[wubi]` 分节；缺省值照 fcitx5 `wbx.conf` 与 librime `wubi86.schema.yaml` 的共同缺省。

use serde::{Deserialize, Serialize};

/// 五笔选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    /// 敲满四码且有全码命中时首选自动上屏（fcitx `AutoSelect`）。
    pub auto_select: bool,

    /// 逐键提示候选（前缀命中）右侧显示完整编码；全码命中不注，自己敲的码不用看。
    pub hint: bool,

    /// 作用域不超过这么多码时全码命中只按码表静态词频排，不叠用户权重：一级 / 二级简码的位置是肌肉记忆（fcitx `NoSortInputLength`）。
    pub fixed_order_length: usize,

    /// 整句输入：连着打编码不按空格，引擎自己切词出整句（Rime `enable_sentence`）。开着时四码自动上屏与顶字都停掉
    /// （缓冲区要能接着长），不超过四码且有命中时候选不变；缺省关，老五笔用户要的是四码即上屏。
    pub sentence: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            auto_select: true,
            hint: true,
            fixed_order_length: 2,
            sentence: false,
        }
    }
}
