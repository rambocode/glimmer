//! 五笔的装配与热加载：按 `[general] wubi` 打开随包码表装成方案，`[wubi]` 选项变了只换选项，
//! 开关 / 变体变了同时换学习器（按输入串记的表落到方案分目录）。码表找不到或坏了当没开五笔，输入法照常起。

use glimmer_core::WubiVariant;
use glimmer_core::wubi::{Options, Scheme};
use glimmer_platform::Config;

use super::Host;
use super::init::load_learner;
use crate::app::paths;

/// 按变体打开随包码表（与 `dict.qj` 同一个 Resources 目录）并装成方案；文件缺失或坏了记 warn 返回 `None`。
pub(super) fn load_wubi_scheme(variant: WubiVariant, options: Options) -> Option<Scheme> {
    let path = match paths::resource(variant.data_file()) {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(%error, scheme = variant.key(), "五笔码表不在包里，按拼音");
            return None;
        }
    };
    let started = std::time::Instant::now();
    match glimmer_dictionary::Dictionary::from_path(&path) {
        Ok(table) => {
            let scheme = Scheme::new(variant, table, options);
            tracing::info!(
                scheme = variant.key(),
                entries = scheme.dictionary().len(),
                chars = scheme.reverse().len(),
                load_ms = started.elapsed().as_millis(),
                "五笔已启用"
            );
            Some(scheme)
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "五笔码表读不了，按拼音");
            None
        }
    }
}

impl Host {
    /// 把 `[general] wubi` 与 `[wubi]` 推给 Engine：变体没变只同步选项；开关或变体变了重新装码表，
    /// 并换一份按方案分目录的学习器（用户词 / 选择记录 / 敲错表在拼音与五笔下会撞键，不能共用）。
    pub(super) fn apply_wubi(&mut self, config: &Config) {
        let wanted = config.general.wubi();
        let options = config.wubi.options();
        let current = self.engine.wubi().map(Scheme::variant);
        if wanted == current {
            // Engine 只给 `&Scheme`，换选项要先卸下再装回；选项没变就什么都不碰（set_wubi 会清缓存）
            let stale =
                current.is_some() && self.engine.wubi().map(Scheme::options) != Some(options);
            if stale && let Some(mut scheme) = self.engine.take_wubi() {
                scheme.set_options(options);
                self.engine.set_wubi(Some(scheme));
            }
            return;
        }
        let scheme = wanted.and_then(|variant| load_wubi_scheme(variant, options));
        // 码表装不上时 loaded 为 None：Engine 回到拼音，学习器也按拼音装，与配置里写的不一致只在日志里说
        let loaded = scheme.as_ref().map(Scheme::variant);
        if loaded == current {
            return;
        }
        // 缓冲区里的键在拼音与五笔下含义不同，真换方案前把正在组句的内容作废（只改 [wubi] 选项不清）
        self.reset_composition();
        self.engine.set_wubi(scheme);
        if let Some(dir) = paths::user_data_dir() {
            let learner = load_learner(&dir, loaded.map(WubiVariant::key));
            self.engine.set_learner(Box::new(learner));
        }
        match loaded {
            Some(variant) => tracing::info!(scheme = variant.key(), "已切到五笔"),
            None => tracing::info!("五笔已关，回到拼音"),
        }
    }
}
