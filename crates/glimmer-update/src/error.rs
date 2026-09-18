//! 检查更新与下载的错误类型。

use thiserror::Error;

/// 检查更新 / 下载安装包的错误；给用户看的中文由壳按变体翻译。
#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("current version `{0}` is not a valid version number")]
    BadCurrentVersion(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("release feed is not valid JSON: {0}")]
    Feed(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("feed lists no sha256 for {0}")]
    MissingChecksum(String),

    #[error("checksum mismatch for {file}: expected {expected}, got {actual}")]
    Checksum {
        file: String,
        expected: String,
        actual: String,
    },
}
