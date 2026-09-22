//! 打分、回退与 `.qj` 往返的小样例。

use glimmer_core::sentence::{Context, LanguageModel};
use glimmer_format::Metadata;

use crate::smoothing::{BackoffMode, Smoothing};

use super::NgramModel;

const UNIGRAM: &str = "<s>\t100\n我\t50\n想\t30\n去\t20\n翔\t1\n";
const BIGRAM: &str = "<s>\t我\t40\n我\t想\t25\n想\t去\t15\n我\t翔\t1\n";
const TRIGRAM: &str = "<s>\t我\t想\t20\n我\t想\t去\t12\n";

/// 临时目录里的一个文件名，测试之间不撞。
fn temp_path(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("glimmer-lm-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{name}-{}.qj", std::process::id()))
}

fn model(smoothing: Smoothing) -> NgramModel {
    let mut model = NgramModel::parse(UNIGRAM, BIGRAM, TRIGRAM).unwrap();
    model.set_smoothing(smoothing);
    model
}

#[test]
fn legacy_mode_keeps_the_fixed_lambda_formula() {
    let model = model(Smoothing::LEGACY);
    let want: f64 = (0.8 * 25.0 / 50.0 + 0.2 * 30.0 / 101.0_f64).ln();
    let got = model.log_prob(Context::after("我"), "想").unwrap();
    assert!((got - want).abs() < 1e-12, "{got} != {want}");
    // 前词未知只用一元；模型不认识的词交回 Core 兜底
    let alone = model.log_prob(Context::after("火星"), "想").unwrap();
    assert!((alone - (30.0_f64 / 101.0).ln()).abs() < 1e-12);
    assert_eq!(model.log_prob(Context::after("我"), "火星"), None);
}

#[test]
fn trigram_beats_bigram_and_backs_off_when_unseen() {
    let model = model(Smoothing::DEFAULT);
    let bigram = model.log_prob(Context::after("想"), "去").unwrap();
    let trigram = model
        .log_prob(Context::after_two("我", "想"), "去")
        .unwrap();
    assert!(trigram > bigram, "{trigram} <= {bigram}");
    // 这对上文没接过 翔：退回二元，与只知道前一个词时一样（三元那层只把没分配的质量交下去）
    let unseen = model
        .log_prob(Context::after_two("我", "想"), "翔")
        .unwrap();
    let plain = model.log_prob(Context::after("想"), "翔").unwrap();
    assert!(unseen < plain);
    // 前二词的上文自己不是一条二元：整段退回二元
    assert_eq!(
        model.log_prob(Context::after_two("去", "想"), "去"),
        Some(bigram)
    );
}

#[test]
fn continuation_unigram_differs_from_the_plain_one() {
    let absolute = model(Smoothing {
        mode: BackoffMode::Absolute,
        ..Smoothing::DEFAULT
    });
    let continuation = model(Smoothing::DEFAULT);
    // 翔 只被 我 接过一次，一元计数也只有 1：两种一元都给它很低的分，但数值不同
    let a = absolute.log_prob(Context::START, "翔").unwrap();
    let c = continuation.log_prob(Context::START, "翔").unwrap();
    assert!(a.is_finite() && c.is_finite() && (a - c).abs() > 1e-9);
    // 句首见过 我：两种模式下都比 去 强
    for model in [&absolute, &continuation] {
        assert!(
            model.log_prob(Context::START, "我").unwrap()
                > model.log_prob(Context::START, "去").unwrap()
        );
    }
}

#[test]
fn qj_round_trip_gives_identical_probabilities() {
    let model = NgramModel::parse(UNIGRAM, BIGRAM, TRIGRAM).unwrap();
    let path = temp_path("trigram");
    let metadata = Metadata {
        name: "测试模型".to_owned(),
        ..Metadata::default()
    };
    model.write_qj(&path, &metadata).unwrap();
    let mapped = NgramModel::from_path(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(mapped.word_count(), model.word_count());
    assert_eq!(mapped.bigram_count(), model.bigram_count());
    assert_eq!(mapped.trigram_count(), 2);
    assert_eq!(mapped.metadata().unwrap().entries, 6);
    assert_eq!(
        mapped.unigrams().collect::<Vec<_>>(),
        [("<s>", 100), ("我", 50), ("想", 30), ("去", 20), ("翔", 1)]
    );
    assert_eq!(
        mapped.bigrams().collect::<Vec<_>>(),
        [(0, 1, 40), (1, 2, 25), (1, 4, 1), (2, 3, 15)]
    );
    // (前二词, 前词, 后词, 计数)：<s> 我 想 与 我 想 去
    assert_eq!(
        mapped.trigrams().collect::<Vec<_>>(),
        [(0, 1, 2, 20), (1, 2, 3, 12)]
    );
    for context in [
        Context::START,
        Context::after("我"),
        Context::after("想"),
        Context::after_two("我", "想"),
        Context::after_two("火星", "想"),
    ] {
        for word in ["我", "想", "去", "翔", "火星"] {
            assert_eq!(
                mapped.log_prob(context, word),
                model.log_prob(context, word)
            );
        }
    }
}

#[test]
fn a_file_without_the_trigram_sections_still_loads() {
    let model = NgramModel::parse(UNIGRAM, BIGRAM, "").unwrap();
    assert_eq!(model.trigram_count(), 0);
    let path = temp_path("bigram-only");
    model.write_qj(&path, &Metadata::default()).unwrap();
    let mapped = NgramModel::from_path(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(mapped.trigram_count(), 0);
    // 没有三元时，知道前二词与只知道前一个词的分一样
    assert_eq!(
        mapped.log_prob(Context::after_two("我", "想"), "去"),
        mapped.log_prob(Context::after("想"), "去")
    );
}
