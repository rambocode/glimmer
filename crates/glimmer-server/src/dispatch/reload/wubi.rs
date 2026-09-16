//! 配置热加载里的五笔：开关 / 换版本时重开码表并换成按方案分目录的学习器，只改 `[wubi]` 选项时码表不动。

use glimmer_platform::Config;

use super::Router;
use crate::assembly::{WubiSpec, load_learner, load_scheme};

impl Router {
    /// 让 Engine 的五笔状态跟上新配置。版本没变只同步 `[wubi]` 选项；开关 / 版本变了先试着装码表，
    /// 装出来的结果与 Engine 上现有的一样（配置写着开但码表缺失、Engine 本来就是拼音）就什么都不碰，
    /// 免得之后每一次无关的配置改动都清组句、重读学习数据。
    pub(super) fn apply_wubi_config(&mut self, config: &Config) {
        let wanted = config.general.wubi();
        let options = config.wubi.options();
        let current = self.engine.wubi().map(|scheme| scheme.variant());
        if wanted == current {
            // 版本没变：只有选项变了才换，`take_wubi` 会清缓存与待上屏候选，没变就别碰
            if self
                .engine
                .wubi()
                .is_some_and(|scheme| scheme.options() != options)
                && let Some(mut scheme) = self.engine.take_wubi()
            {
                scheme.set_options(options);
                self.engine.set_wubi(Some(scheme));
                tracing::info!("五笔选项已热加载");
            }
            return;
        }
        let wubi_dir = self
            .reload
            .as_ref()
            .and_then(|reload| reload.wubi_dir.clone());
        let scheme = wanted.and_then(|variant| {
            let Some(dir) = &wubi_dir else {
                tracing::warn!("没有码表目录，热加载开不了五笔");
                return None;
            };
            load_scheme(dir, WubiSpec { variant, options })
        });
        // 码表装不上时 loaded 为 None：与 Engine 上的拼音一致就直接返回，配置与实际不符只在日志里说
        let loaded = scheme.as_ref().map(|scheme| scheme.variant());
        if loaded == current {
            return;
        }
        // 确定要切换了：缓冲区里的键换了含义（编码 ↔ 拼音），组句作废；
        // 按输入串记的学习表在五笔下落在方案子目录，学习器也换（旧的先 flush）
        self.reset_composition();
        let scheme_key = loaded.map(|variant| variant.key());
        if let Some(dir) = self
            .reload
            .as_ref()
            .and_then(|reload| reload.user_dir.clone())
        {
            self.engine
                .set_learner(Box::new(load_learner(&dir, scheme_key)));
        }
        self.engine.set_wubi(scheme);
        tracing::info!(wubi = scheme_key.unwrap_or("关"), "五笔已热加载");
    }

    /// 当前装着的五笔方案键（`wubi86`）；拼音为 `None`。状态条与启动日志用。
    pub fn wubi_key(&self) -> Option<&'static str> {
        self.engine.wubi().map(|scheme| scheme.key())
    }
}
