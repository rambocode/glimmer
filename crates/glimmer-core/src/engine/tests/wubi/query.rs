//! 五笔查询：全码 / 前缀 / 固定序 / 空码 / 反查 / 模式键。

use super::*;

#[test]
fn full_code_hits_come_before_prefix_hints_with_their_codes() {
    let mut engine = wubi_engine();
    assert_eq!(wubi_texts(&mut engine, "gggg"), ["王"]);
    assert_eq!(wubi_texts(&mut engine, "ggll"), ["一"]);
    engine.set_input("gg");
    let query = engine.query().unwrap();
    let items = &query.candidates.items;
    assert_eq!(items[0].text, "五");
    assert_eq!(items[0].reading, None);
    assert_eq!(items[0].syllables, ["gg"]);
    let wang = items.iter().find(|c| c.text == "王").unwrap();
    assert_eq!(wang.reading.as_deref(), Some("gggg"));
    assert_eq!(wang.syllables, ["gggg"]);
    assert!(items.iter().all(|c| c.kind == CandidateKind::Chinese));
    // 显示原样是敲的码，没有切分
    assert_eq!(query.marked_text(), "gg");
    assert!(query.segmentations.is_empty() && !query.decoded_keys);
    // 前缀命中短码在前：`g` 下 五 / 玉 / 天（两码）排在 王（四码）前
    let g = wubi_texts(&mut engine, "g");
    assert!(g.iter().position(|t| t == "王").unwrap() > g.iter().position(|t| t == "天").unwrap());
    assert_eq!(g[0], "五");
}

#[test]
fn same_text_under_two_codes_is_listed_once() {
    let mut engine = wubi_engine();
    let a = wubi_texts(&mut engine, "a");
    assert_eq!(a.iter().filter(|t| *t == "工").count(), 1);
    assert_eq!(a[0], "工");
    assert_eq!(wubi_texts(&mut engine, "aaaa"), ["工"]);
}

#[test]
fn hint_can_be_turned_off() {
    let mut engine = wubi_engine_with(Options {
        hint: false,
        ..Options::default()
    });
    engine.set_input("gg");
    let query = engine.query().unwrap();
    assert!(query.candidates.items.iter().all(|c| c.reading.is_none()));
}

#[test]
fn short_codes_keep_the_table_order_regardless_of_learning() {
    let mut learner = CountingLearner(HashMap::new());
    for _ in 0..30 {
        learner.record(&Candidate {
            text: "式".into(),
            kind: CandidateKind::Chinese,
            syllables: vec!["aa".into()],
            reading: None,
            translation: None,
        });
        learner.record_choice("a", "式");
    }
    let mut engine = wubi_engine().with_learner(Box::new(learner));
    // 一级简码 `a` 下 工 永远在第一，式 只是前缀提示
    assert_eq!(wubi_texts(&mut engine, "a")[0], "工");
    // 超过固定序长度的编码才叠用户权重：三码以上按 record 的次数排
    let mut engine = wubi_engine_with(Options {
        fixed_order_length: 0,
        ..Options::default()
    });
    engine.learner_mut().record_choice("gg", "王");
    assert_eq!(wubi_texts(&mut engine, "gg")[0], "五");
}

#[test]
fn choices_reorder_full_code_hits_beyond_the_fixed_length() {
    let table = "王\tgggg\t9000\n五\tgggg\t7000\n";
    let mut engine = engine();
    engine.set_wubi(Some(WubiScheme::new(
        Variant::Wubi86,
        Dictionary::parse(table).unwrap(),
        Options::default(),
    )));
    assert_eq!(wubi_texts(&mut engine, "gggg")[0], "王");
    let learner = CountingLearner(HashMap::new());
    let mut engine = engine.with_learner(Box::new(learner));
    engine.learner_mut().record_choice("gggg", "五");
    assert_eq!(wubi_texts(&mut engine, "gggg")[0], "五");
}

#[test]
fn unknown_code_has_no_candidates_and_keeps_the_buffer() {
    let mut engine = wubi_engine();
    assert!(wubi_texts(&mut engine, "xxxx").is_empty());
    assert_eq!(engine.query().unwrap().marked_text(), "xxxx");
    assert!(engine.backspace());
    assert_eq!(engine.composition().text(), "xxx");
}

#[test]
fn reverse_lookup_annotates_pinyin_candidates_with_codes() {
    let table = "中\tkhk\t9000\n中国\tkhlg\t8000\n国\tlgyi\t8000\n王\tgggg\t9000\n";
    let dictionary =
        Dictionary::parse("中\tzhong\t9000\n国\tguo\t8000\n中国\tzhong guo\t9000\n王\twang\t100\n")
            .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_wubi(Some(WubiScheme::new(
        Variant::Wubi86,
        Dictionary::parse(table).unwrap(),
        Options::default(),
    )));
    engine.set_input("zzhong");
    let query = engine.query().unwrap();
    let zhong = &query.candidates.items[0];
    assert_eq!(zhong.text, "中");
    assert_eq!(zhong.reading.as_deref(), Some("khk"));
    assert_eq!(zhong.syllables, ["zzhong"]);
    assert_eq!(query.marked_text(), "z'zhong");
    assert_eq!(query.marked_cursor(), 7);
    engine.set_input("zzhongguo");
    let query = engine.query().unwrap();
    let zhongguo = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "中国")
        .unwrap();
    assert_eq!(zhongguo.reading.as_deref(), Some("khlg"));
    assert_eq!(zhongguo.syllables, ["zzhongguo"]);
    assert_eq!(query.marked_text(), "z'zhong'guo");
    // 只有一个 z：没有候选
    assert!(wubi_texts(&mut engine, "z").is_empty());
    // 反查候选上屏吃掉整段
    engine.set_input("zzhongguo");
    let zhongguo = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "中国")
        .unwrap();
    assert_eq!(engine.commit(&zhongguo), "中国");
    assert!(engine.composition().is_empty());
}

#[test]
fn mode_keys_are_codes_and_other_characters_make_a_raw_segment() {
    let mut engine = wubi_engine();
    engine.set_input("v");
    assert!(!engine.expression_mode());
    engine.set_input("u");
    assert!(!engine.question_mode() && !engine.unicode_entry());
    // 表达式 / 问字键当编码查：小码表里没有就没有候选，但不是表达式候选
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.kind == CandidateKind::Chinese)
    );
    engine.set_input("gg-");
    assert!(engine.raw_mode());
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "gg-");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
    assert!(!engine.takes_semicolon());
    assert_eq!(engine.scheme_key(), "wubi86");
}

#[test]
fn shuangpin_and_zhuyin_are_ignored_while_wubi_is_on() {
    let mut engine = wubi_engine();
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_zhuyin_mode(true);
    assert_eq!(wubi_texts(&mut engine, "gggg"), ["王"]);
    assert!(!engine.query().unwrap().decoded_keys);
    assert!(!engine.zhuyin_needs_tone());
    assert_eq!(engine.scheme_key(), "wubi86");
    // 关掉五笔，双拼又生效
    let taken = engine.take_wubi();
    assert!(taken.is_some() && engine.wubi().is_none());
    engine.set_zhuyin_mode(false);
    engine.set_input("kdfa");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发");
}

#[test]
fn compose_prediction_is_off_but_question_still_asks_the_cloud() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = wubi_engine();
    engine.set_predictor(Box::new(EchoPredictor {
        submitted: submitted.clone(),
        replies: Vec::new(),
        sentence: true,
    }));
    // 缓冲区里是编码不是拼音，组句联想不发
    engine.set_input("gggg");
    assert_eq!(engine.request_prediction(None, &[]), None);
    // `?` 问字敲的仍是拼音，照发
    engine.set_input("?mumumu");
    assert!(engine.request_prediction(None, &[]).is_some());
    let request = submitted.borrow()[0].clone();
    assert_eq!(request.kind, PredictionKind::Question);
    assert_eq!(request.pinyin, "mu'mu'mu");
}
