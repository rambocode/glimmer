//! 英文及领域别名词表的行为测试。

use super::*;

#[test]
fn looks_up_by_lowercase_code() {
    let list = WordList::parse("GitHub\tgithub\nhello\thello\niPhone\n").unwrap();
    assert_eq!(list.get("github"), Some("GitHub"));
    assert_eq!(list.get("iphone"), Some("iPhone"));
    assert_eq!(list.get("hello"), Some("hello"));
    assert_eq!(list.get("nope"), None);
}

#[test]
fn completes_prefixes_by_frequency() {
    let list = WordList::parse(
        "compass\tcompass\t300\ncompany\tcompany\t900\ncompare\tcompare\t500\ncom\tcom\t100\ncomma\tcomma\n",
    )
    .unwrap();
    assert_eq!(list.complete("comp", 2), ["company", "compare"]);
    assert_eq!(
        list.complete("com", 10),
        ["company", "compare", "compass", "comma"]
    );
    assert!(list.complete("zzz", 3).is_empty());
    assert!(list.complete("", 3).is_empty());
}

#[test]
fn explicit_aliases_preserve_symbols_and_mixed_names() {
    let dict = crate::Dictionary::parse("C++\t@cpp\t40\nC#\t@csharp\t40\nGit分支\t@gitfenzhi\t20\nAGENTS.md\t@agentsmd\t40\nC++\t@cplusplus\t40\n旧条目\tlegacy\t20\n").unwrap();
    let words = WordList::from_dictionaries(&[dict]);
    assert_eq!(words.get("cpp"), Some("C++"));
    assert_eq!(words.get("csharp"), Some("C#"));
    assert_eq!(words.get("cplusplus"), Some("C++"));
    assert_eq!(words.get("gitfenzhi"), Some("Git分支"));
    assert_eq!(words.complete("agents", 3), ["AGENTS.md"]);
    assert_eq!(words.get("legacy"), None);
}

#[test]
fn imported_invalid_aliases_are_ignored_and_first_dictionary_wins() {
    let first = crate::Dictionary::parse(
        "C++\t@cpp\t40\nbad\t@../x\t20\nempty\t@\t20\nupper\t@UPPER\t20\n",
    )
    .unwrap();
    let second = crate::Dictionary::parse("different\t@cpp\t80\n").unwrap();
    let words = WordList::from_dictionaries(&[first, second]);
    assert_eq!(words.len(), 1);
    assert_eq!(words.get("cpp"), Some("C++"));
}
