//! 五笔方案的版本（86 / 98）：配置写法、方案键、码表文件名。

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 五笔的版本。配置 `[general] wubi = "86" | "98"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Variant {
    /// 86 版五笔（极点码表）。
    Wubi86,

    /// 98 版五笔。
    Wubi98,
}

/// 不认识的五笔版本。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown wubi variant: {0:?}")]
pub struct UnknownVariant(pub String);

impl Variant {
    /// 全部版本，设置界面按这个顺序列出。
    pub const ALL: [Self; 2] = [Self::Wubi86, Self::Wubi98];

    /// 方案键：输入日志与回放、学习数据的方案子目录都用它（`wubi86` / `wubi98`）。
    pub fn key(self) -> &'static str {
        match self {
            Self::Wubi86 => "wubi86",
            Self::Wubi98 => "wubi98",
        }
    }

    /// 配置文件里的写法（`86` / `98`）。
    pub fn config_key(self) -> &'static str {
        match self {
            Self::Wubi86 => "86",
            Self::Wubi98 => "98",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Wubi86 => "86 五笔",
            Self::Wubi98 => "98 五笔",
        }
    }

    /// 随包码表的文件名（`data/generated/` 下）。
    pub fn data_file(self) -> &'static str {
        match self {
            Self::Wubi86 => "wubi86.qj",
            Self::Wubi98 => "wubi98.qj",
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

    /// 接受配置写法（`86`）与方案键（`wubi86`），不分大小写、忽略首尾空白。
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "86" | "wubi86" => Ok(Self::Wubi86),
            "98" | "wubi98" => Ok(Self::Wubi98),
            _ => Err(UnknownVariant(s.to_owned())),
        }
    }
}
