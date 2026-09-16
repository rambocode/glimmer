//! 五笔方案的版本（86 / 98 / 新世纪）：配置写法、方案键、码表文件名。

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 五笔的版本。配置 `[general] wubi = "86" | "98" | "xsj"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Variant {
    /// 86 版五笔（极点码表）。
    Wubi86,

    /// 98 版五笔。
    Wubi98,

    /// 新世纪版五笔（王码 2008 版，社区惯称 06 版）。
    Xinshiji,
}

/// 不认识的五笔版本。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown wubi variant: {0:?}")]
pub struct UnknownVariant(pub String);

impl Variant {
    /// 全部版本，设置界面按这个顺序列出。
    pub const ALL: [Self; 3] = [Self::Wubi86, Self::Wubi98, Self::Xinshiji];

    /// 方案键：输入日志与回放、学习数据的方案子目录都用它（`wubi86` / `wubi98` / `wubixsj`）。
    pub fn key(self) -> &'static str {
        match self {
            Self::Wubi86 => "wubi86",
            Self::Wubi98 => "wubi98",
            Self::Xinshiji => "wubixsj",
        }
    }

    /// 配置文件里的写法（`86` / `98` / `xsj`）。
    pub fn config_key(self) -> &'static str {
        match self {
            Self::Wubi86 => "86",
            Self::Wubi98 => "98",
            Self::Xinshiji => "xsj",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Wubi86 => "86 五笔",
            Self::Wubi98 => "98 五笔",
            Self::Xinshiji => "新世纪五笔",
        }
    }

    /// 随包码表的文件名（`data/generated/` 下）。
    pub fn data_file(self) -> &'static str {
        match self {
            Self::Wubi86 => "wubi86.qj",
            Self::Wubi98 => "wubi98.qj",
            Self::Xinshiji => "wubixsj.qj",
        }
    }
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for Variant {
    type Err = UnknownVariant;

    /// 接受配置写法（`86`）与方案键（`wubi86`），不分大小写、忽略首尾空白；
    /// 新世纪多认两个别名：社区惯称的 `06` 与全拼写 `xinshiji`。
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "86" | "wubi86" => Ok(Self::Wubi86),
            "98" | "wubi98" => Ok(Self::Wubi98),
            "xsj" | "06" | "xinshiji" | "wubixsj" => Ok(Self::Xinshiji),
            _ => Err(UnknownVariant(s.to_owned())),
        }
    }
}
