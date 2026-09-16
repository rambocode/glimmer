//! 配置热加载记的状态。

use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use glimmer_core::Language;
use glimmer_dictionary::Dictionary;
use glimmer_platform::{DictionariesConfig, extra_dictionaries};
use glimmer_predict::PredictConfig;

use crate::assembly::user_dicts_dir;

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

    /// 最近加载的用户词库文件快照（路径、修改时间、长度）。
    pub(super) dictionary_files: Vec<(PathBuf, Option<SystemTime>, u64)>,
}

impl ConfigReload {
    /// 按上次有效配置装配词库，不把用户目录中的学习数据当作词库。
    pub(super) fn load_dictionaries(&self) -> Vec<Dictionary> {
        let dictionaries = extra_dictionaries::load(
            self.bundled_dicts_dir.as_deref(),
            user_dicts_dir(self.user_dir.as_deref()).as_deref(),
            &self.applied_dictionaries,
        );
        tracing::info!(count = dictionaries.len(), "附加词库已热重装");
        dictionaries
    }
}
