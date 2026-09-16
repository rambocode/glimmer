//! 启动装配要知道的平台路径与身份：各平台壳按自己的约定填（Windows `%APPDATA%\Glimmer`，Linux XDG 目录）。

use std::path::PathBuf;

/// 平台壳交给 [`super::build_router`] 的路径与身份。
#[derive(Debug, Clone)]
pub struct StartupPaths {
    /// 用户数据目录（学习数据、用户词库、个人模型）；拿不到为 `None`，那样都只在内存。
    pub user_dir: Option<PathBuf>,

    /// 配置文件 `config.toml`；拿不到为 `None`，那样不热加载。
    pub config_path: Option<PathBuf>,

    /// 随包资源根（其下 `data/` 与 `assets/`）。
    pub root: PathBuf,

    /// 壳的版本号，写进输入日志的会话信息。
    pub version: &'static str,

    /// 平台名（`windows` / `linux`），写进输入日志的会话信息。
    pub platform: &'static str,
}
