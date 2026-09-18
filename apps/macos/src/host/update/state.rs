//! 检查更新的运行时状态：进行中的检查 / 下载、查到的新版本、已下好的安装包。

use std::path::PathBuf;

use glimmer_update::{Available, Download, UpdateCheck};
use objc2::MainThreadMarker;

use super::UpdateMonitor;

pub struct UpdateState {
    /// 进行中的检查；没在查为 `None`。
    pub check: Option<UpdateCheck>,

    /// 进行中的下载；没在下为 `None`。
    pub download: Option<Download>,

    /// 上次检查发现的新版本。
    pub available: Option<Available>,

    /// 已下载并校验过的安装包；再点「下载并安装」直接打开它。
    pub downloaded: Option<PathBuf>,

    /// 这轮检查是用户点的（要在「关于」页报告「已是最新」与错误），还是启动时自动的（只在有新版本时出声）。
    pub manual: bool,

    /// 轮询定时器。
    pub monitor: UpdateMonitor,
}

impl UpdateState {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self {
            check: None,
            download: None,
            available: None,
            downloaded: None,
            manual: false,
            monitor: UpdateMonitor::new(mtm),
        }
    }
}
