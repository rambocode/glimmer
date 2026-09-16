//! Linux 前端的错误类型。

use std::path::PathBuf;

/// 起 IBus 引擎进程时可能出的错。
#[derive(Debug, thiserror::Error)]
pub enum LinuxError {
    /// 读 IBus 地址文件失败（daemon 没起，或用户配置目录 / DISPLAY 不对）。
    #[error("failed to read ibus address file {path}: {source}")]
    AddressFile {
        /// 地址文件路径。
        path: PathBuf,

        /// 底层错误。
        source: std::io::Error,
    },

    /// 地址文件里没有 `IBUS_ADDRESS=` 行。
    #[error("no IBUS_ADDRESS in {0}")]
    NoAddress(PathBuf),

    /// D-Bus 连接、导出对象或占名字失败。
    #[error("d-bus: {0}")]
    Bus(#[from] zbus::Error),

    /// 起异步运行时失败。
    #[error("failed to start runtime: {0}")]
    Runtime(std::io::Error),
}
