//! 配置热加载记的状态。

use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use glimmer_core::Language;
use glimmer_platform::DictionariesConfig;
use glimmer_predict::PredictConfig;

/// 热加载状态。
pub(crate) struct ConfigReload {
    /// `config.toml` 路径。
    pub(super) config_path: PathBuf,

    /// 上次看文件的时间（节流用）。
    pub(super) last_check: Instant,

    /// 随包数据根目录（释义表在 `data/generated` 下）。
    pub(super) root: PathBuf,

    /// 随包领域词库目录。
    pub(super) bundled_dicts_dir: Option<PathBuf>,

    /// 用户数据目录（导入词库在其 `dicts/` 下，五笔的学习数据在其 `<方案键>/` 下）。
    pub(super) user_dir: Option<PathBuf>,

    /// 五笔码表所在目录（与主词库同目录）；`None` 时热加载开不了五笔。
    pub(super) wubi_dir: Option<PathBuf>,

    /// 上次看到的 mtime。
    pub(super) last_mtime: Option<SystemTime>,

    /// 已应用的 `[predict]`。
    pub(super) applied_predict: PredictConfig,

    /// 已应用的 `[dictionaries]`。
    pub(super) applied_dictionaries: DictionariesConfig,

    /// 已应用的学习语言（`None` 为关）。
    pub(super) applied_language: Option<Language>,
}
