//! 检查更新：拉清单、比版本。线程 + 通道版给主线程轮询，阻塞版给自带后台任务的 UI 框架。

use std::sync::mpsc;

use crate::available::Available;
use crate::config::UpdateConfig;
use crate::error::UpdateError;
use crate::feed::Feed;
use crate::http::feed_client;
use crate::target::Target;
use crate::version::Version;

/// 一次进行中的检查。
pub struct UpdateCheck {
    /// 检查线程送回的结果；线程只发一次。
    result: mpsc::Receiver<Result<Option<Available>, UpdateError>>,
}

impl UpdateCheck {
    /// 起线程检查。`current` 是本机版本号（解析不了当场报错），`target` 是本机平台 / 架构。
    pub fn start(
        config: &UpdateConfig,
        current: &str,
        target: Target,
    ) -> Result<Self, UpdateError> {
        let current = Version::parse(current)
            .ok_or_else(|| UpdateError::BadCurrentVersion(current.to_owned()))?;
        let feed_url = config.feed_url.clone();
        let (sender, result) = mpsc::channel();
        std::thread::Builder::new()
            .name("glimmer-update-check".to_owned())
            .spawn(move || {
                // 收的一方不在了（窗口关了、又点了一次）就算了
                let _ = sender.send(run(&feed_url, &current, target));
            })?;
        Ok(Self { result })
    }

    /// 结果到了就取走；没到返回 `None`。取走之后再调永远是 `None`。
    pub fn poll(&self) -> Option<Result<Option<Available>, UpdateError>> {
        self.result.try_recv().ok()
    }
}

/// 阻塞地查一次：拉清单、解析、与 `current` 比。`Ok(None)` 是已经最新。
pub fn check_blocking(
    config: &UpdateConfig,
    current: &str,
    target: Target,
) -> Result<Option<Available>, UpdateError> {
    let current = Version::parse(current)
        .ok_or_else(|| UpdateError::BadCurrentVersion(current.to_owned()))?;
    run(&config.feed_url, &current, target)
}

/// 真正的检查；日志在这里记一次，两个入口共用。
fn run(
    feed_url: &str,
    current: &Version,
    target: Target,
) -> Result<Option<Available>, UpdateError> {
    let outcome = fetch(feed_url).map(|feed| feed.available(current, target));
    match &outcome {
        Ok(Some(update)) => {
            tracing::info!(version = %update.version, file = %update.asset.file, "有新版本")
        }
        Ok(None) => tracing::info!("已是最新版本"),
        Err(error) => tracing::warn!(%error, feed_url, "检查更新失败"),
    }
    outcome
}

/// 下载并解析清单。
fn fetch(feed_url: &str) -> Result<Feed, UpdateError> {
    let text = feed_client()?
        .get(feed_url)
        .send()?
        .error_for_status()?
        .text()?;
    Ok(Feed::parse(&text)?)
}

#[cfg(test)]
mod tests {
    use crate::config::UpdateConfig;
    use crate::target::{Platform, Target};

    /// 真去拉一次缺省清单：`cargo test -p glimmer-update -- --ignored`。要联网，平时不跑。
    #[test]
    #[ignore = "要联网"]
    fn live_feed_lists_a_macos_package() {
        let target = Target::new(Platform::MacOs, &["Apple Silicon"]);
        let update = super::check_blocking(&UpdateConfig::default(), "0.0.1", target)
            .expect("拉清单失败")
            .expect("清单里该有 macOS 包");
        assert!(
            update.asset.file.ends_with("-arm64.pkg"),
            "{}",
            update.asset.file
        );
        assert_eq!(update.asset.sha256.len(), 64);
    }
}
