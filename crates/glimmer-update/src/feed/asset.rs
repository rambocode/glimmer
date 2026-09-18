//! 清单里的一个安装包。

use serde::Deserialize;

/// 一个安装包：平台 / 架构标签、文件名、下载地址、体积与 sha256。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Asset {
    /// `macos` / `windows` / `linux`。
    pub platform: String,

    /// 给人看的架构标签（`Apple Silicon` / `Intel` / `x64` / `x86_64` / `ARM64`）。
    pub arch: String,

    pub file: String,

    pub url: String,

    /// 字节数；旧清单没有这一项时为 0。
    #[serde(default)]
    pub size: u64,

    /// 十六进制 sha256；没有就不允许自动安装。
    #[serde(default)]
    pub sha256: String,
}
