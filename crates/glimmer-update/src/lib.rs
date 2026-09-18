//! 应用内检查更新。平台无关：只管「有没有新版本」与「把安装包下到本机并校验」，
//! 打开安装器（macOS `open` pkg、Windows 运行 Setup.exe）由各平台的壳做。
//!
//! 数据源是发版脚本生成、挂在 GitHub latest Release 上的 `releases.json`（结构见 `docs/notes/release.md`）。
//! 网络请求都在独立线程里跑，主线程只起线程、轮询通道（[`UpdateCheck::poll`] / [`Download::poll`]），
//! 与云联想的连通性测试同一套写法；也提供阻塞版（[`check_blocking`] / [`download_blocking`]）给自带后台任务的 UI 框架用。

mod available;
mod check;
mod config;
mod download;
mod error;
mod feed;
mod http;
mod stamp;
mod target;
mod version;

pub use available::Available;
pub use check::{UpdateCheck, check_blocking};
pub use config::{CHECK_INTERVAL, DEFAULT_FEED_URL, UpdateConfig};
pub use download::{Download, download_blocking};
pub use error::UpdateError;
pub use feed::{Asset, Feed, Release};
pub use stamp::{STAMP_FILE, check_due, touch};
pub use target::{Platform, Target};
pub use version::Version;
