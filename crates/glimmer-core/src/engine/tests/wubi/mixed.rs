//! 混输（五笔 + 拼音）：两边都出候选时的顺序、去重、模式键、纠错与上屏消耗。

use super::*;

/// 混输用的拼音词库：与下面的码表有几处故意撞形（`ga` 既是 开 的全码又是 嘎 的拼音）。
const MIXED_SAMPLE: &str = "你好\tni hao\t20000\n你\tni\t30000\n好\thao\t25000\n开\tkai\t20000\n\
     干\tgan\t8000\n嘎\tga\t500\n我\two\t30000\n想\txiang\t9000\n学\txue\t8000\n\
     明天\tming tian\t50000\n名\tming\t9000\n天\ttian\t9000\n";

/// 混输用的五笔小码表（86 码）：开 ga / 开发 gant / 中共党员 kaik / 一 ggll / 嘎 gaj / 小 ih。
const MIXED_TABLE: &str = "开\tga\t9000\n开发\tgant\t8000\n中共党员\tkaik\t7000\n\
     一\tggll\t9000\n嘎\tgaj\t600\n小\tih\t5000\n";

/// 拼音侧开着 + 五笔开着 = 混输。
fn mixed_engine() -> Engine {
    let mut engine = Engine::new(Dictionary::parse(MIXED_SAMPLE).unwrap());
    engine.set_wubi(Some(WubiScheme::new(
        Variant::Wubi86,
        Dictionary::parse(MIXED_TABLE).unwrap(),
        Options::default(),
    )));
    engine
}

fn mixed_texts(engine: &mut Engine, input: &str) -> Vec<String> {
    engine.set_input(input);
    texts_of(engine)
}

/// 序号，找不到时 panic（断言里要的就是它一定在）。
fn position(texts: &[String], text: &str) -> usize {
    texts.iter().position(|t| t == text).expect(text)
}

#[test]
fn a_full_code_beats_the_pinyin_reading_of_the_same_keys() {
    let mut engine = mixed_engine();
    // `gant` 是 开发 的全码；同样的字母按拼音读是 gan + t，拼音候选跟在后面
    let gant = mixed_texts(&mut engine, "gant");
    assert_eq!(gant[0], "开发");
    assert!(gant.contains(&"干".to_owned()));
    // `ga` 是 开 的全码，拼音读作 嘎
    let ga = mixed_texts(&mut engine, "ga");
    assert_eq!(ga[0], "开");
    assert!(position(&ga, "嘎") > 0);
}

#[test]
fn prefix_only_code_hits_sit_behind_the_pinyin_candidates() {
    let mut engine = mixed_engine();
    // `kai` 没有全码，只有 中共党员（kaik）命中前缀：拼音的 开 才是首选，编码前缀词垫后
    let kai = mixed_texts(&mut engine, "kai");
    assert_eq!(kai[0], "开");
    assert!(position(&kai, "中共党员") > position(&kai, "开"));
}

#[test]
fn the_same_word_from_both_sides_is_listed_once() {
    let mut engine = mixed_engine();
    // 嘎 既是拼音 ga 的词，又是编码 gaj 的前缀命中：只留靠前的那条（拼音段）
    let ga = mixed_texts(&mut engine, "ga");
    assert_eq!(ga.iter().filter(|t| *t == "嘎").count(), 1);
    assert!(position(&ga, "开发") > position(&ga, "嘎"));
}

#[test]
fn keys_that_do_not_read_as_pinyin_fall_back_to_the_code_table() {
    let mut engine = mixed_engine();
    // `ggll` 是 一 的全码；按拼音只读得出简拼，这些词都排在全码后面
    let ggll = mixed_texts(&mut engine, "ggll");
    assert_eq!(ggll[0], "一");

    // `ih` 连简拼都读不出来（`i` 起不了头）：拼音那条路失败不算错，整段按五笔走
    assert_eq!(mixed_texts(&mut engine, "ih"), ["小"]);
    let query = engine.query().unwrap();
    assert!(query.segmentations.is_empty());
    assert_eq!(query.marked_text(), "ih");
}

#[test]
fn beyond_four_letters_only_pinyin_is_left() {
    let mut engine = mixed_engine();
    // 五笔码最长四位，第 5 个字母起码表查不到东西
    assert_eq!(mixed_texts(&mut engine, "nihao")[0], "你好");
    let long = mixed_texts(&mut engine, "woxiangxuehao");
    assert!(long.contains(&"我".to_owned()));
    assert!(!engine.query().unwrap().segmentations.is_empty());
}

#[test]
fn mode_keys_move_to_the_shifted_letters_like_shuangpin() {
    let mut engine = mixed_engine();
    // 小写字母既是字根键又是拼音键，表达式 / 问字走 Shift+V / Shift+U
    assert!(engine.takes_mode_letter('V') && engine.takes_mode_letter('U'));
    engine.set_input("v");
    assert!(!engine.expression_mode());
    engine.set_input("V1+2");
    assert!(engine.expression_mode());
    engine.set_input("Unihao");
    assert!(engine.question_mode());
    assert_eq!(engine.scheme_key(), "pinyin+wubi86");
}

#[test]
fn spelling_correction_starts_where_the_codes_end() {
    let mut engine = mixed_engine();
    // 四个字母以内还可能是编码，不纠
    engine.set_input("gant");
    assert!(engine.query().unwrap().correction.is_none());
    // 超过四个字母只可能是拼音，照常纠（mingtain → mingtian）
    engine.set_input("mingtain");
    let query = engine.query().unwrap();
    assert!(query.correction.is_some());
    assert_eq!(query.candidates.items[0].text, "明天");
}

#[test]
fn four_codes_do_not_auto_commit_in_mixed_input() {
    let mut engine = mixed_engine();
    // `gant` 敲满四码也不自动上屏：混输下四个字母同样可能是拼音（`niha`）
    for c in "gant".chars() {
        engine.push(c);
        assert!(engine.take_auto_commit().is_none());
    }
    assert_eq!(engine.composition().text(), "gant");
}

#[test]
fn committing_takes_the_whole_code_for_code_candidates_and_the_syllables_for_pinyin() {
    let learner = CountingLearner(HashMap::new());
    let mut engine = mixed_engine().with_learner(Box::new(learner));
    // 五笔候选：吃掉整段编码，按编码记学习键
    engine.set_input("ga");
    let kai = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(kai.text, "开");
    assert_eq!(engine.commit(&kai), "开");
    assert!(engine.composition().is_empty());
    assert_eq!(
        engine
            .learner()
            .choice_weight("ga", "开", ChoicePosition::SentenceStart),
        1
    );

    // 只命中编码前缀的五笔候选同样吃掉整段作用域
    engine.set_input("kai");
    let party = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "中共党员")
        .unwrap();
    engine.commit(&party);
    assert!(engine.composition().is_empty());

    // 拼音候选照音节吃：选 你 只吃掉 ni，hao 留在缓冲区
    engine.set_input("nihao");
    let ni = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "你")
        .unwrap();
    engine.commit(&ni);
    assert_eq!(engine.composition().text(), "hao");
    assert_eq!(
        engine
            .learner()
            .choice_weight("ni", "你", ChoicePosition::Continuation),
        1
    );
}

#[test]
fn turning_the_pinyin_side_off_goes_back_to_code_only() {
    let mut engine = mixed_engine();
    assert_eq!(mixed_texts(&mut engine, "kai")[0], "开");
    engine.set_phonetic(false);
    // 只用形码：`kai` 只剩编码前缀命中，模式键让位给字根键
    assert_eq!(mixed_texts(&mut engine, "kai"), ["中共党员"]);
    assert!(!engine.takes_mode_letter('V'));
    assert_eq!(engine.scheme_key(), "wubi86");
}
