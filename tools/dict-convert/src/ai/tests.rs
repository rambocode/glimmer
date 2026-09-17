//! AI 生成器的格式、去重、语料来源与容器回归测试。

use super::{convert, validate};
use glimmer_dictionary::{Dictionary, WordList};
use std::path::Path;

#[test]
fn validates_codes_weights_and_kinds() {
    assert!(
        validate(&[
            "重排序",
            "chong pai xu",
            "20",
            "检索",
            "term",
            "重排序",
            "curated",
            "2026-09-17"
        ])
        .is_ok()
    );
    assert!(
        validate(&[
            "Claude Opus 5",
            "claudeopus5",
            "40",
            "模型",
            "english",
            "Anthropic",
            "anthropic",
            "2026-09-17"
        ])
        .is_ok()
    );
    assert!(
        validate(&[
            "模型",
            "mo",
            "20",
            "基础",
            "term",
            "模型",
            "curated",
            "2026-09-17"
        ])
        .is_err()
    );
    assert!(
        validate(&[
            "模型",
            "mo xing",
            "0",
            "基础",
            "term",
            "模型",
            "curated",
            "2026-09-17"
        ])
        .is_err()
    );
    assert!(
        validate(&[
            "Claude",
            "cluade",
            "40",
            "模型",
            "english",
            "Anthropic",
            "anthropic",
            "2026-09-17"
        ])
        .is_err()
    );
}

#[test]
fn bundled_sources_generate_queryable_container_and_audit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("glimmer-ai-test-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let existing = out.join("existing.tsv");
    std::fs::write(&existing, "机器学习\tji qi xue xi\t100\nGGUF\t@gguf\t20\n").unwrap();
    convert(
        &root.join("assets/lexicon/ai"),
        &[existing],
        &[root.join("assets/lexicon/ai/corpus.txt")],
        &out,
    )
    .unwrap();
    let dict = Dictionary::from_path(out.join("dicts/ai.qj")).unwrap();
    assert!(dict.len() > 2500);
    assert_eq!(dict.metadata().unwrap().name, "AI 与人工智能");
    assert!(
        !dict
            .lookup(&["jian", "suo", "zeng", "qiang", "sheng", "cheng"], false)
            .is_empty()
    );
    assert!(
        dict.lookup(&["ji", "qi", "xue", "xi"], false)
            .iter()
            .all(|entry| entry.text != "机器学习")
    );
    let mut engine = glimmer_core::Engine::new(Dictionary::parse("爱\tai\t100\n").unwrap());
    engine.set_extra_dictionaries(vec![dict]);
    for input in ["jiansuozengqiangshengcheng", "j's'z'q's'ch"] {
        engine.set_input(input);
        assert!(
            engine
                .query()
                .unwrap()
                .candidates
                .items
                .iter()
                .any(|c| c.text == "检索增强生成"),
            "{input}"
        );
    }
    let scheme = glimmer_core::ShuangpinScheme::Xiaohe;
    let keys: String = ["jian", "suo", "zeng", "qiang", "sheng", "cheng"]
        .iter()
        .flat_map(|s| scheme.encode(s).unwrap())
        .collect();
    engine.set_shuangpin(Some(scheme));
    engine.set_input(&keys);
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|c| c.text == "检索增强生成")
    );
    let english = WordList::from_dictionaries(engine.extra_dictionaries());
    assert_eq!(english.get("claudeopus5"), Some("Claude Opus 5"));
    assert_eq!(english.get("gguf"), None);
    assert!(english.complete("claude", 20).contains(&"Claude Opus 5"));
    let audit = std::fs::read_to_string(out.join("ai-audit.tsv")).unwrap();
    assert!(
        audit
            .lines()
            .any(|line| line.starts_with("机器学习\tji qi xue xi\t20\t")
                && line.contains("\texisting\t"))
    );
    let row = audit
        .lines()
        .find(|line| line.starts_with("机器学习\t"))
        .unwrap()
        .split('\t')
        .collect::<Vec<_>>();
    assert_eq!(row[2], "20");
    assert!(row[3].parse::<u32>().unwrap() > 20);
    assert_eq!(
        row[4].parse::<u32>().unwrap(),
        row[3].parse::<u32>().unwrap().min(200)
    );
    std::fs::remove_dir_all(out).unwrap();
}

#[test]
fn rejects_invalid_date_and_unmarked_english_entries() {
    assert!(
        validate(&[
            "模型",
            "mo xing",
            "20",
            "基础",
            "term",
            "模型",
            "curated",
            "2026-02-30"
        ])
        .is_err()
    );
    let dict = Dictionary::parse("Claude\tclaude\t20\nAI\t@ai\t20\nC盘\tc pan\t20\n").unwrap();
    let words = WordList::from_dictionaries(&[dict]);
    assert_eq!(words.get("ai"), Some("AI"));
    assert_eq!(words.get("claude"), None);
}
