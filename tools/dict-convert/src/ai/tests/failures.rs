//! 生成错误须可定位，并且不会覆盖上一次有效词库。

use std::path::Path;

use super::super::convert;

fn source(dir: &Path, names: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("terms.tsv"),
        "项目\t xiang mu\t20\t开发\tterm\t项目\tcurated\t2026-09-17\n"
            .replace("\t xiang", "\txiang"),
    )
    .unwrap();
    std::fs::write(dir.join("names.tsv"), names).unwrap();
    std::fs::write(
        dir.join("sources.json"),
        r#"{"curated":{"license":"GPL-3.0-or-later"}}"#,
    )
    .unwrap();
}

#[test]
fn collisions_across_source_files_fail_before_output_changes() {
    let dir = std::env::temp_dir().join(format!("glimmer-alias-collision-{}", std::process::id()));
    let input = dir.join("source");
    let out = dir.join("output");
    source(
        &input,
        "C++\tcpp\t40\t语言\talias\tcpp\tcurated\t2026-09-17\n",
    );
    convert(&input, &[], &[], &out).unwrap();
    let before = std::fs::read(out.join("dicts/ai.qj")).unwrap();
    std::fs::create_dir_all(input.join("domains")).unwrap();
    std::fs::write(
        input.join("domains/conflict.tsv"),
        "C#\tcpp\t40\t语言\talias\tcsharp\tcurated\t2026-09-17\n",
    )
    .unwrap();
    let error = convert(&input, &[], &[], &out).unwrap_err().to_string();
    assert!(error.contains("alias code 'cpp' conflicts"), "{error}");
    assert!(error.contains(":1"), "{error}");
    assert_eq!(std::fs::read(out.join("dicts/ai.qj")).unwrap(), before);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn different_aliases_for_one_word_are_allowed_and_unknown_sources_fail() {
    let dir = std::env::temp_dir().join(format!("glimmer-alias-source-{}", std::process::id()));
    let input = dir.join("source");
    let out = dir.join("output");
    source(
        &input,
        "C++\tcpp\t40\t语言\talias\tcpp\tcurated\t2026-09-17\nC++\tcplusplus\t40\t语言\talias\tcpp\tcurated\t2026-09-17\n",
    );
    convert(&input, &[], &[], &out).unwrap();
    let dict = glimmer_dictionary::Dictionary::from_path(out.join("dicts/ai.qj")).unwrap();
    let words = glimmer_dictionary::WordList::from_dictionaries(&[dict]);
    assert_eq!(words.get("cpp"), words.get("cplusplus"));
    assert_eq!(words.complete("c", 9), ["C++"]);
    std::fs::write(
        input.join("names.tsv"),
        "C++\tcpp\t40\t语言\talias\tcpp\tmissing\t2026-09-17\n",
    )
    .unwrap();
    assert!(
        convert(&input, &[], &[], &out)
            .unwrap_err()
            .to_string()
            .contains("missing source")
    );
    std::fs::remove_dir_all(dir).unwrap();
}
