//! 评测时上文怎么给引擎：走上屏链（自己刚打的）还是走应用光标前文（别人的话 / 粘贴来的）。

/// 一条评测里 `Pair::context` 送进引擎的途径。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum ContextSource {
    /// 上屏链：上文当成用户自己刚上屏的词（`Engine::seed_chain`），同时写进本会话历史。
    /// 这是壳里「接着自己打的往下打」的样子，也是历来的评测口径。
    #[default]
    Chain,

    /// 应用光标前文：上文只经 `Engine::set_surrounding_before` 给，上屏链保持空。
    /// 模拟「前文是对方发来的话 / 刚粘贴进来的 / 挪了光标之后接着打」——这些场合上屏链没有内容。
    App,
}

impl ContextSource {
    /// 报告里写的模式后缀；缺省口径不写。
    pub fn label(self) -> &'static str {
        match self {
            Self::Chain => "",
            Self::App => "，上文只经应用前文",
        }
    }
}
