//! 五笔装配：按 `[general] wubi` 的版本在词库目录里找码表文件装成 [`Scheme`]；启动与配置热加载共用。

use std::path::Path;
use std::time::Instant;

use glimmer_core::WubiVariant;
use glimmer_core::wubi::{Options, Scheme};
use glimmer_dictionary::Dictionary;

/// 五笔要装的东西：版本与 `[wubi]` 选项。码表文件名由版本定（`wubi86.qj`），放在主词库同目录。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WubiSpec {
    /// 版本（`[general] wubi`）。
    pub variant: WubiVariant,

    /// 行为选项（`[wubi]` 分节）。
    pub options: Options,
}

/// 打开 `<dir>/<码表文件>` 装成方案。文件不存在或坏了记一条 warn 并返回 `None`：这时按五笔没开处理
/// （学习器也用拼音那份目录），不让一个缺文件把整个 Server 拖死。
pub fn load_scheme(dir: &Path, spec: WubiSpec) -> Option<Scheme> {
    let path = dir.join(spec.variant.data_file());
    if !path.is_file() {
        tracing::warn!(path = %path.display(), "找不到五笔码表，五笔按没开处理");
        return None;
    }
    let started = Instant::now();
    match Dictionary::from_path(&path) {
        Ok(table) => {
            let scheme = Scheme::new(spec.variant, table, spec.options);
            tracing::info!(
                scheme = scheme.key(),
                entries = scheme.dictionary().len(),
                chars = scheme.reverse().len(),
                load_ms = started.elapsed().as_millis(),
                "五笔已启用"
            );
            Some(scheme)
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "五笔码表打不开，五笔按没开处理");
            None
        }
    }
}
