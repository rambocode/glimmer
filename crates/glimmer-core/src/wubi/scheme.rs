//! 五笔方案：码表、反查表、选项与方案键，Engine 里是 `Option<Scheme>`。

use glimmer_dictionary::Dictionary;

use super::{Options, Reverse, Variant, encode};

/// 一套装配好的五笔方案。
#[derive(Debug)]
pub struct Scheme {
    /// 版本（86 / 98），方案键与码表文件名由它定。
    variant: Variant,

    /// 码表：`词\t编码\t词频`，编码整个是一个音节。
    dictionary: Dictionary,

    /// 单字反查表，从码表建。
    reverse: Reverse,

    /// 行为选项。
    options: Options,
}

impl Scheme {
    /// 用码表装配方案，反查表当场建（十几万条几十毫秒，壳在后台线程装配）。
    pub fn new(variant: Variant, dictionary: Dictionary, options: Options) -> Self {
        let reverse = Reverse::build(&dictionary);
        tracing::debug!(
            scheme = variant.key(),
            entries = dictionary.len(),
            chars = reverse.len(),
            "五笔方案已装配"
        );
        Self {
            variant,
            dictionary,
            reverse,
            options,
        }
    }

    pub fn variant(&self) -> Variant {
        self.variant
    }

    /// 方案键（`wubi86` / `wubi98` / `wubixsj`）。
    pub fn key(&self) -> &'static str {
        self.variant.key()
    }

    pub fn dictionary(&self) -> &Dictionary {
        &self.dictionary
    }

    pub fn reverse(&self) -> &Reverse {
        &self.reverse
    }

    pub fn options(&self) -> Options {
        self.options
    }

    /// 换选项（配置热加载），码表不动。
    pub fn set_options(&mut self, options: Options) {
        self.options = options;
    }

    /// 一段文本的五笔码：单字查反查表，多字按造词规则算；算不出为 `None`。
    pub fn code_of(&self, text: &str) -> Option<String> {
        let chars: Vec<char> = text.chars().collect();
        match chars.as_slice() {
            [] => None,
            [ch] => self.reverse.code(*ch).map(str::to_owned),
            _ => encode(&chars, &self.reverse),
        }
    }
}
