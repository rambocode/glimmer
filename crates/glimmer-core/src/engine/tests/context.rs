//! 整句转换接上文：上屏链或应用光标前文决定这段拼音第一个词怎么读。

use super::*;

/// 认得 做了 这个词，且 吧 / 把 的取舍只看前词的假模型。
struct ContextModel;

impl LanguageModel for ContextModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        match (previous, word) {
            (_, "做了") => Some(-2.0),
            (Some("做了"), "吧") => Some(-1.0),
            (Some("做了"), "把") => Some(-8.0),
            (_, "把") => Some(-3.0),
            (_, "吧") => Some(-7.0),
            (_, "吃") => Some(-4.0),
            _ => None,
        }
    }
}

/// 整句候选的文本。
fn sentence_of(engine: &mut Engine, input: &str) -> String {
    engine.set_input(input);
    let query = engine.query().unwrap();
    query
        .candidates
        .items
        .into_iter()
        .find(|c| c.kind == CandidateKind::Sentence)
        .map(|c| c.text)
        .unwrap()
}

/// 句首的 `bachi` 读 把吃；光标前是「我做了」时第一个词接着 做了 算，读 吧吃；前文以标点结尾又回到句首。
#[test]
fn sentence_conversion_follows_the_text_before_it() {
    let dictionary = Dictionary::parse(
        "做了\tzuo le\t9000\n吧\tba\t50000\n把\tba\t60000\n吃\tchi\t40000\n我\two\t90000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary).with_language_model(Box::new(ContextModel));
    assert_eq!(sentence_of(&mut engine, "bachi"), "把吃");
    engine.clear();
    engine.seed_chain("，我做了");
    assert_eq!(sentence_of(&mut engine, "bachi"), "吧吃");
    engine.clear();
    engine.seed_chain("我做了。");
    assert_eq!(sentence_of(&mut engine, "bachi"), "把吃");
}

/// 从用户写过的文本里学：文本里反复出现的「做了 → 吧」记进个人 n-gram，之后同样的接续整句跟着读 吧；
/// 私密输入中不学。
#[test]
fn learning_from_text_feeds_the_personal_ngram() {
    let dictionary = Dictionary::parse(
        "做了\tzuo le\t9000\n吧\tba\t50000\n把\tba\t60000\n吃\tchi\t40000\n我\two\t90000\n",
    )
    .unwrap();
    // 静态模型里 吧 / 把 不看前词，把 占优
    struct FlatModel;
    impl LanguageModel for FlatModel {
        fn log_prob(&self, _: Option<&str>, word: &str) -> Option<f64> {
            match word {
                "做了" => Some(-2.0),
                "把" => Some(-3.0),
                "吧" => Some(-4.0),
                "吃" => Some(-4.0),
                _ => None,
            }
        }
    }
    let mut engine = Engine::new(dictionary)
        .with_language_model(Box::new(FlatModel))
        .with_learner(Box::new(WordLearner::default()));
    assert_eq!(sentence_of(&mut engine, "zuoleba"), "做了把");
    engine.clear();
    engine.set_private(true);
    assert_eq!(engine.learn_text("做了吧，做了吧。"), 0);
    assert_eq!(sentence_of(&mut engine, "zuoleba"), "做了把");
    engine.clear();
    engine.set_private(false);
    let recorded = engine.learn_text("做了吧，做了吧。做了吧！做了吧？abc");
    assert_eq!(recorded, 8);
    assert_eq!(sentence_of(&mut engine, "zuoleba"), "做了吧");
}

/// 应用光标前文当上文：壳给了「我做了」，上屏链是空的，`bachi` 也读 吧吃；
/// 前文以标点结尾、壳没给前文都回到句首。
#[test]
fn sentence_conversion_follows_the_surrounding_text() {
    let dictionary = Dictionary::parse(
        "做了\tzuo le\t9000\n吧\tba\t50000\n把\tba\t60000\n吃\tchi\t40000\n我\two\t90000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary).with_language_model(Box::new(ContextModel));
    engine.set_surrounding_before(Some("聊了两句，我做了".to_owned()));
    assert_eq!(sentence_of(&mut engine, "bachi"), "吧吃");
    engine.clear();
    // clear 把这段组句的前文作废了，壳下一段第一键再送
    assert_eq!(sentence_of(&mut engine, "bachi"), "把吃");
    engine.clear();
    engine.set_surrounding_before(Some("我做了。".to_owned()));
    assert_eq!(sentence_of(&mut engine, "bachi"), "把吃");
    engine.clear();
    engine.set_surrounding_before(None);
    assert_eq!(sentence_of(&mut engine, "bachi"), "把吃");
}

/// 上屏链上有词时以链为准：链上是「做了」而应用前文早就不含它（壳只在组句起头读一次），
/// 仍按链读 吧吃；链断开之后才轮到应用前文。
#[test]
fn the_commit_chain_wins_over_the_surrounding_text() {
    let dictionary = Dictionary::parse(
        "做了\tzuo le\t9000\n吧\tba\t50000\n把\tba\t60000\n吃\tchi\t40000\n我\two\t90000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary).with_language_model(Box::new(ContextModel));
    engine.set_surrounding_before(Some("聊了两句。".to_owned()));
    engine.seed_chain("我做了");
    assert_eq!(sentence_of(&mut engine, "bachi"), "吧吃");
    engine.clear();
    engine.break_chain();
    engine.set_surrounding_before(Some("聊了两句。".to_owned()));
    assert_eq!(sentence_of(&mut engine, "bachi"), "把吃");
}
