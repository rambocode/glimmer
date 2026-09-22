//! 词库里「一个字对一个音节」的音节判定，`lexicon` 与 `supplement` 共用。
//!
//! 中英混杂词（C盘 / B站 / P0）里的拉丁字母与数字各占一个字，输入码是这个字符本身（`cpan` 出 C盘，
//! 搜狗 / 微软同款）。它们不是拼音音节、在 Unihan 里也没有读音，所以两条并入路径都要单独放行：
//! 不放行的话 `lexicon` 会把整条丢掉（`dropped`），`supplement` 会报「读音缺失或与字数不符」。

use glimmer_core::parser::is_syllable;

/// 这个「音节」能不能当词库键的一节：合法拼音音节，或中英混杂词里代表一个字符的单个 ASCII 字母 / 数字。
pub fn is_usable_syllable(syllable: &str) -> bool {
    is_syllable(syllable) || is_letter_syllable(syllable)
}

/// `syllable` 能不能当 `ch` 这个字的读音。
///
/// 汉字只认合法拼音音节：Unihan 给 儿 列了儿化的 `r`，放进去会顶掉 `er` 成主读音。
/// 拉丁字母 / 数字则由词表说了算（C盘 的 C 读 `c`，U盘 的 U 读 `you`），Unihan 里本来就没有它们。
pub fn is_reading_of(ch: char, syllable: &str) -> bool {
    if ch.is_ascii_alphanumeric() {
        is_usable_syllable(syllable)
    } else {
        is_syllable(syllable)
    }
}

/// 整词逐字校验：音节数与字数相等，且每一节都是对应那个字能有的读音。
pub fn is_reading_of_word(text: &str, syllables: &[impl AsRef<str>]) -> bool {
    text.chars().count() == syllables.len()
        && text
            .chars()
            .zip(syllables)
            .all(|(ch, syllable)| is_reading_of(ch, syllable.as_ref()))
}

/// 单个 ASCII 小写字母或数字：中英混杂词里一个拉丁字符自成一节。
pub fn is_letter_syllable(syllable: &str) -> bool {
    syllable.len() == 1
        && syllable
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// 这个字符是不是自带读音的拉丁字符（拉丁字母或数字），读音就是它的小写形式。
pub fn letter_reading(ch: char) -> Option<String> {
    ch.is_ascii_alphanumeric()
        .then(|| ch.to_ascii_lowercase().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_pinyin_and_single_latin_characters() {
        assert!(is_usable_syllable("pan"));
        assert!(is_usable_syllable("c"));
        assert!(is_usable_syllable("0"));
        assert!(!is_usable_syllable("cp"));
        assert!(!is_usable_syllable("C"));
        assert!(!is_usable_syllable(""));
    }

    #[test]
    fn han_characters_reject_the_erhua_r_but_latin_ones_follow_the_word_list() {
        assert!(is_reading_of('儿', "er"));
        assert!(!is_reading_of('儿', "r"));
        assert!(is_reading_of('C', "c"));
        assert!(is_reading_of('U', "you"));
        assert!(is_reading_of_word("U盘", &["you", "pan"]));
        assert!(!is_reading_of_word("U盘", &["you"]));
    }

    #[test]
    fn reads_latin_characters_as_their_lowercase_form() {
        assert_eq!(letter_reading('C').as_deref(), Some("c"));
        assert_eq!(letter_reading('0').as_deref(), Some("0"));
        assert_eq!(letter_reading('盘'), None);
    }
}
