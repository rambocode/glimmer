//! 清单里的一次发布。

use serde::Deserialize;

use super::Asset;

/// 一次发布：版本号、日期、更新日志与各平台安装包。
#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub version: String,

    /// `YYYY-MM-DD`。
    #[serde(default)]
    pub date: String,

    /// `alpha` / `beta` / `rc` / `stable`。
    #[serde(default)]
    pub channel: String,

    /// 更新日志，一条一行（来自 CHANGELOG.md）。
    #[serde(default)]
    pub notes: Vec<String>,

    #[serde(default)]
    pub assets: Vec<Asset>,
}
