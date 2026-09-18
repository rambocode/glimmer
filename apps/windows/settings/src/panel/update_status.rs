//! 「关于」页检查更新的状态，挂在 [`Settings`](super::Settings) 上，由根组件的 `update` 推进、关于页显示。

use std::path::PathBuf;

use glimmer_update::Available;

/// 检查更新 / 下载安装包走到了哪一步。
#[derive(Clone)]
pub(crate) enum UpdateStatus {
    /// 没查过。
    Idle,

    /// 正在查。`manual` 是用户点的（要报告「已是最新」与错误），启动时自动查的只在有新版本时出声。
    Checking { manual: bool },

    /// 查过了，已是最新。
    UpToDate,

    /// 有新版本，等用户点「下载并安装」。
    Available(Available),

    /// 正在下载这一版。
    Downloading(Available),

    /// 下载并校验完成，安装器已经拉起；再点按钮就再拉一次。
    Downloaded(PathBuf),

    /// 检查或下载失败（带说明）；有 `Available` 时按钮还在，可以重试下载。
    Failed(String, Option<Available>),
}
