//! 本机是哪个平台、哪种架构，用来在清单里挑安装包。

use crate::feed::Asset;

/// 清单里 `platform` 字段的三个取值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}

impl Platform {
    /// 清单里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::MacOs => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }
}

/// 平台 + 架构。架构在清单里是给人看的标签（`Apple Silicon` / `Intel` / `x64` / `ARM64`），
/// 发版脚本 `tools/release/releases_json.py` 的 `ASSET_KINDS` 定的；这里按别名表认，不区分大小写。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub platform: Platform,

    /// 清单里可能出现的架构写法，任一匹配即可。
    arch_labels: &'static [&'static str],
}

impl Target {
    /// 编译目标对应的本机平台与架构；没发过包的组合（32 位等）返回 `None`。
    pub fn current() -> Option<Self> {
        let platform = if cfg!(target_os = "macos") {
            Platform::MacOs
        } else if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "linux") {
            Platform::Linux
        } else {
            return None;
        };
        let arch_labels: &'static [&'static str] = match (platform, std::env::consts::ARCH) {
            (Platform::MacOs, "aarch64") => &["Apple Silicon", "arm64", "aarch64"],
            (Platform::MacOs, "x86_64") => &["Intel", "x86_64", "x64"],
            (Platform::Windows, "x86_64") => &["x64", "x86_64", "amd64"],
            (Platform::Windows, "aarch64") => &["ARM64", "arm64", "aarch64"],
            (Platform::Linux, "x86_64") => &["x86_64", "amd64", "x64"],
            (Platform::Linux, "aarch64") => &["ARM64", "arm64", "aarch64"],
            _ => return None,
        };
        Some(Self {
            platform,
            arch_labels,
        })
    }

    /// 测试与工具用：指定平台与架构别名。
    pub fn new(platform: Platform, arch_labels: &'static [&'static str]) -> Self {
        Self {
            platform,
            arch_labels,
        }
    }

    /// 这个安装包是不是给本机的。
    pub fn matches(&self, asset: &Asset) -> bool {
        asset.platform.eq_ignore_ascii_case(self.platform.key())
            && self
                .arch_labels
                .iter()
                .any(|label| asset.arch.eq_ignore_ascii_case(label))
    }
}
