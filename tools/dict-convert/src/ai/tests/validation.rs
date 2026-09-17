//! 领域输入码、显式别名和中英混合词的格式约束。

use super::super::validation::{is_alias, validate};

#[test]
fn accepts_explicit_aliases_and_legacy_mixed_codes() {
    for (word, code, kind, alias) in [
        ("C++", "cpp", "alias", true),
        ("C#", "csharp", "alias", true),
        (".env", "envfile", "alias", true),
        ("Claude Code", "claudecode", "english", true),
        ("Git分支", "gitfenzhi", "mixed", true),
        ("向量TopK", "xiangliangtopk", "mixed", true),
        ("AI模型", "ai mo xing", "mixed", false),
    ] {
        let fields = [
            word,
            code,
            "40",
            "开发",
            kind,
            word,
            "curated",
            "2026-09-17",
        ];
        assert!(validate(&fields).is_ok(), "{word}");
        assert_eq!(is_alias(&fields), alias);
    }
}

#[test]
fn rejects_unusable_or_ambiguous_encoding_formats() {
    for (word, code, kind) in [
        ("C++", "cpp", "english"), // 非默认编码须显式声明。
        ("C++", "../cpp", "alias"),
        ("C#", "CSharp", "alias"),
        ("name", "a b", "alias"),
        ("name", "1name", "alias"),
        ("name", "", "alias"),
        ("name", "@name", "alias"),
        ("Git分支", "git fen zhi", "mixed"),
        ("Git", "git", "mixed"),
        ("分支", "fenzhi", "mixed"),
        ("AI", "ai", "mixed"),
        ("bad\rname", "badname", "alias"),
        ("分支", "fen zhi", "unknown"),
    ] {
        assert!(
            validate(&[
                word,
                code,
                "40",
                "开发",
                kind,
                word,
                "curated",
                "2026-09-17"
            ])
            .is_err(),
            "{word}/{code}"
        );
    }
}
