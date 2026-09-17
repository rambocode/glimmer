use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write config {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid config {path}: {source}")]
    Parse {
        path: PathBuf,

        /// 装着不拆：`toml::de::Error` 一百多字节，直接放进枚举会把 `ConfigError`
        /// （以及各壳聚合它的错误类型）撑过 clippy 的 128 字节阈值。
        #[source]
        source: Box<toml::de::Error>,
    },

    #[error("cannot edit config {path} in place: {source}")]
    Edit {
        path: PathBuf,
        #[source]
        source: toml_edit::TomlError,
    },
}
