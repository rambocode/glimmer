//! 领域词条格式与输入码校验，兼容首版 AI 拼音混合词。

use glimmer_core::parser::is_syllable;
use glimmer_dictionary::valid_alias_code;

pub(super) fn validate(f: &[&str]) -> Result<(), String> {
    if f.len() != 8 || f.iter().any(|v| v.is_empty() || *v != v.trim()) {
        return Err("expected eight non-empty, trimmed TSV fields".into());
    }
    let weight = f[2].parse::<u32>().map_err(|_| "invalid manual weight")?;
    if !(1..=200).contains(&weight) {
        return Err("manual weight must be in 1..=200".into());
    }
    let code: Vec<&str> = f[1].split(' ').collect();
    let han = |c: char| ('\u{4e00}'..='\u{9fff}').contains(&c);
    match f[4] {
        "term" | "chinese" => {
            if !f[0].chars().all(han)
                || f[0].chars().count() != code.len()
                || code.iter().any(|s| !is_syllable(s))
            {
                return Err(
                    "Chinese word requires one canonical pinyin syllable per character".into(),
                );
            }
        }
        "english" | "alias" => {
            if !f[0].is_ascii()
                || !f[0].bytes().any(|b| b.is_ascii_alphabetic())
                || f[0].chars().any(char::is_control)
                || !valid_alias_code(f[1])
            {
                return Err("English name requires a lowercase alphanumeric alias code".into());
            }
            if f[4] == "english" {
                let compact: String = f[0]
                    .bytes()
                    .filter(u8::is_ascii_alphanumeric)
                    .map(|b| (b as char).to_ascii_lowercase())
                    .collect();
                if compact != f[1] {
                    return Err("use kind alias for an explicit non-default input code".into());
                }
            }
        }
        "mixed" => {
            if !f[0].chars().any(han)
                || !f[0].bytes().any(|b| b.is_ascii_alphabetic())
                || f[0].chars().any(char::is_control)
            {
                return Err("mixed name requires Chinese and ASCII letters".into());
            }
            // 新的中英混合词走显式别名；保留旧 AI 词的音节编码，避免既有双拼输入变化。
            if !valid_alias_code(f[1])
                && (!f[0].starts_with("AI")
                    || !f[0][2..].chars().all(han)
                    || code.first() != Some(&"ai")
                    || code.len() != f[0][2..].chars().count() + 1
                    || code.iter().any(|s| !is_syllable(s)))
            {
                return Err("mixed name requires an alias code or legacy AI pinyin code".into());
            }
        }
        _ => return Err("unknown word kind".into()),
    }
    let date = f[7].as_bytes();
    if date.len() != 10
        || date[4] != b'-'
        || date[7] != b'-'
        || date
            .iter()
            .enumerate()
            .any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
    {
        return Err("checked date must be YYYY-MM-DD".into());
    }
    let year: u32 = f[7][..4].parse().expect("validated digits");
    let month: u32 = f[7][5..7].parse().expect("validated digits");
    let day: u32 = f[7][8..].parse().expect("validated digits");
    let max_day = match month {
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day {
        return Err("invalid calendar date".into());
    }
    Ok(())
}

/// 新式英文和混合词的别名不进拼音词图；旧式 AI 音节词保持原有行为。
pub(super) fn is_alias(fields: &[&str]) -> bool {
    matches!(fields[4], "english" | "alias")
        || (fields[4] == "mixed" && valid_alias_code(fields[1]))
}
