//! 五笔：码表输入方案，与 `shuangpin` / `zhuyin` 平级。
//!
//! 码表复用 [`glimmer_dictionary::Dictionary`]，一条编码整个当一个「音节」（`王\tggll\t9000`），
//! 全码用 `lookup_exact`、逐键提示用前缀模式的 `lookup_pattern`。这里只放方案本身的数据与规则：
//! 码表、反查表（字 → 全码）、自动造词的编码规则、选项；查询、上屏、自动上屏的流程在 `engine` 里。

mod encoder;
mod options;
mod reverse;
mod scheme;
mod variant;

pub use encoder::encode;
pub use options::Options;
pub use reverse::Reverse;
pub use scheme::Scheme;
pub use variant::{UnknownVariant, Variant};

/// 一条编码最多几个键：五笔全码四码。
pub const MAX_CODE_LEN: usize = 4;

/// 反查键：作用域以它开头、后面跟全拼时按拼音查字，候选右侧注五笔码。
pub const REVERSE_LOOKUP_KEY: char = 'z';

/// `c` 是不是五笔编码键：`a`–`z` 小写字母（`z` 在首位是反查，其他位置本期当普通字母）。
pub fn is_code_key(c: char) -> bool {
    c.is_ascii_lowercase()
}

/// 这段作用域是不是拼音反查（`z` 开头且后面还有字符）。
pub fn is_reverse_lookup(scope: &str) -> bool {
    scope.len() > 1 && scope.starts_with(REVERSE_LOOKUP_KEY)
}

#[cfg(test)]
pub(crate) mod tests;
