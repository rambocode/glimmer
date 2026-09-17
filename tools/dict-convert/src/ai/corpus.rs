//! 去重的真实正文与透明的出现次数统计；不把子串计数当作分词词频。

use std::collections::HashSet;
use std::path::PathBuf;

use crate::error::ConvertError;

/// 忽略空行和注释；跨文件的完全相同段落只统计一次，段落间保留换行边界。
pub(super) fn load(paths: &[PathBuf]) -> Result<String, ConvertError> {
    let mut seen = HashSet::new();
    let mut text = String::new();
    for path in paths {
        for line in std::fs::read_to_string(path)?.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || !seen.insert(line.to_owned()) {
                continue;
            }
            text.push_str(line);
            text.push('\n');
        }
    }
    Ok(text)
}

/// 中文按非重叠子串计数；英文或混合名称的 ASCII 首尾须落在标识符边界，
/// 避免把 C、Go、R 等从 CSS、Google、Rust 或 C++ 中拆出来虚增权重。
pub(super) fn count(text: &str, word: &str) -> usize {
    let identifier =
        |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+' | '#');
    text.match_indices(word)
        .filter(|(start, _)| {
            let end = start + word.len();
            let first = word.chars().next().expect("validated nonempty word");
            let last = word.chars().next_back().expect("validated nonempty word");
            (!identifier(first)
                || text[..*start]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !identifier(c)))
                && (!identifier(last) || text[end..].chars().next().is_none_or(|c| !identifier(c)))
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::count;

    #[test]
    fn short_names_are_not_counted_inside_other_identifiers() {
        assert_eq!(count("C CSS C++ C# GC C\n", "C"), 2);
        assert_eq!(count("Go Google GoRouter Golang Go\n", "Go"), 2);
        assert_eq!(count("C++ clang++ C++\n", "C++"), 2);
        assert_eq!(count("Git分支 Git分支管理 MyGit分支\n", "Git分支"), 2);
        assert_eq!(count("向量 向量化 向量数据库\n", "向量"), 3);
        assert_eq!(count("段落\n边界", "段落边界"), 0);
    }
}
