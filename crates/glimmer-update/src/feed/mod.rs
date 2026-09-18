//! 发版清单 `releases.json`（`tools/release/releases_json.py` 生成）：只解析检查更新要用的字段。

mod asset;
mod release;

pub use asset::Asset;
pub use release::Release;

use serde::Deserialize;

use crate::available::Available;
use crate::target::Target;
use crate::version::Version;

/// 整份清单。`releases` 从新到旧，但 macOS / Windows / Linux 的版本号各自独立（`0.1.7` / `0.1.0-alpha.6` / `0.1.0-linux.2`），
/// 所以给某个平台找最新版不能取第一条，要在有该平台安装包的条目里按版本号取最大。
#[derive(Debug, Clone, Deserialize)]
pub struct Feed {
    /// 全部发布，从新到旧。
    #[serde(default)]
    pub releases: Vec<Release>,
}

impl Feed {
    /// 解析 JSON 文本。
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// 本机平台 / 架构能装的最新一版（版本号解析不了的条目跳过）。
    pub fn newest_for(&self, target: Target) -> Option<(Version, &Release, &Asset)> {
        self.releases
            .iter()
            .filter_map(|release| {
                let version = Version::parse(&release.version)?;
                let asset = release.assets.iter().find(|asset| target.matches(asset))?;
                Some((version, release, asset))
            })
            .max_by(|a, b| a.0.cmp(&b.0))
    }

    /// 比 `current` 新的可装版本；没有更新为 `None`。
    pub fn available(&self, current: &Version, target: Target) -> Option<Available> {
        let (version, release, asset) = self.newest_for(target)?;
        (version > *current).then(|| Available {
            version: release.version.clone(),
            date: release.date.clone(),
            notes: release.notes.clone(),
            asset: asset.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Feed;
    use crate::target::{Platform, Target};
    use crate::version::Version;

    /// 与真实清单同形：三个平台的版本号各排各的，顺序按发布时间。
    const FEED: &str = r#"{
      "latest": "0.1.7",
      "releases": [
        {"version": "0.1.7", "date": "2026-09-17", "channel": "beta", "notes": ["a"], "assets": [
          {"platform": "macos", "arch": "Apple Silicon", "file": "Glimmer-0.1.7-arm64.pkg", "url": "u1", "size": 1, "sha256": "s1"},
          {"platform": "macos", "arch": "Intel", "file": "Glimmer-0.1.7-x86_64.pkg", "url": "u2", "size": 1, "sha256": "s2"}
        ]},
        {"version": "0.1.0-linux.2", "date": "2026-09-17", "channel": "alpha", "notes": [], "assets": [
          {"platform": "linux", "arch": "x86_64", "file": "Glimmer-0.1.0-linux.2-amd64.deb", "url": "u3", "size": 1, "sha256": "s3"}
        ]},
        {"version": "0.1.0-alpha.6", "date": "2026-09-17", "channel": "alpha", "notes": ["w"], "assets": [
          {"platform": "windows", "arch": "x64", "file": "Glimmer-0.1.0-alpha.6-Setup.exe", "url": "u4", "size": 1, "sha256": "s4"}
        ]},
        {"version": "0.1.0-alpha.5", "date": "2026-09-16", "channel": "alpha", "notes": [], "assets": [
          {"platform": "windows", "arch": "x64", "file": "Glimmer-0.1.0-alpha.5-Setup.exe", "url": "u5", "size": 1, "sha256": "s5"}
        ]},
        {"version": "bogus", "assets": [{"platform": "windows", "arch": "x64", "file": "x", "url": "x"}]}
      ]
    }"#;

    #[test]
    fn picks_newest_per_platform_not_first_entry() {
        let feed = Feed::parse(FEED).unwrap();
        let mac = Target::new(Platform::MacOs, &["Intel"]);
        let (_, release, asset) = feed.newest_for(mac).unwrap();
        assert_eq!(release.version, "0.1.7");
        assert_eq!(asset.file, "Glimmer-0.1.7-x86_64.pkg");
        let win = Target::new(Platform::Windows, &["x64"]);
        assert_eq!(feed.newest_for(win).unwrap().1.version, "0.1.0-alpha.6");
        let arm_win = Target::new(Platform::Windows, &["ARM64"]);
        assert!(feed.newest_for(arm_win).is_none());
    }

    #[test]
    fn available_only_when_newer() {
        let feed = Feed::parse(FEED).unwrap();
        let win = Target::new(Platform::Windows, &["x64"]);
        let current = Version::parse("0.1.0-alpha.5").unwrap();
        let update = feed.available(&current, win).unwrap();
        assert_eq!(update.version, "0.1.0-alpha.6");
        assert_eq!(update.asset.url, "u4");
        assert_eq!(update.notes, vec!["w".to_owned()]);
        let latest = Version::parse("0.1.0-alpha.6").unwrap();
        assert!(feed.available(&latest, win).is_none());
        let dev = Version::parse("0.1.0-alpha.7-dev-abc").unwrap();
        assert!(feed.available(&dev, win).is_none());
    }
}
