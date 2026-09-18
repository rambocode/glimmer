//! 阻塞版 HTTP 客户端：清单一个（整体超时短），下载一个（不限总时长，只限连接与单次读）。

use std::time::Duration;

use reqwest::blocking::Client;

/// 请求头里的 User-Agent：让 GitHub 那边能认出是输入法在查。
fn user_agent() -> String {
    format!(
        "glimmer-update/{} ({})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    )
}

/// 拉清单用：几 KB 的文件，20 秒拉不完就当网络不通。
pub(crate) fn feed_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent(user_agent())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
}

/// 下载安装包用：几十 MB，慢网也要下得完，总时长给 30 分钟；阻塞客户端没有单次读超时，只能靠这个兜底。
pub(crate) fn download_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent(user_agent())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30 * 60))
        .build()
}
