//! 五笔整句（`[wubi] sentence`）：连着打编码出整句、不再自动上屏、四码以内不变、上屏按路径上的词学习。

use super::*;

/// 整句开着的只用形码引擎。
fn sentence_engine() -> Engine {
    wubi_engine_with(Options {
        sentence: true,
        ..Options::default()
    })
}

#[test]
fn codes_typed_in_a_row_become_a_sentence() {
    let mut engine = sentence_engine();
    // 王 gggg + 中国 khlg：一路敲下去不自动上屏、不顶字
    assert_eq!(type_keys(&mut engine, "ggggkhlg"), "");
    assert_eq!(engine.composition().text(), "ggggkhlg");
    let query = engine.query().unwrap();
    let first = &query.candidates.items[0];
    assert_eq!(first.text, "王中国");
    assert_eq!(first.kind, CandidateKind::Sentence);
    // 拼音行按整句的切法分开
    assert_eq!(query.marked_text(), "gggg'khlg");
    // 后面跟开头那段编码的词：先四码的 王，再两码的 五
    let rest: Vec<&str> = query.candidates.items[1..]
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert_eq!(rest, ["王", "五"]);
}

#[test]
fn four_codes_or_fewer_with_hits_stay_as_plain_wubi() {
    let mut plain = wubi_engine_with(Options {
        auto_select: false,
        ..Options::default()
    });
    let mut engine = sentence_engine();
    for input in ["g", "gg", "khk", "gggg"] {
        assert_eq!(
            wubi_texts(&mut engine, input),
            wubi_texts(&mut plain, input),
            "{input}"
        );
        let query = engine.query().unwrap();
        assert!(
            query
                .candidates
                .items
                .iter()
                .all(|c| c.kind != CandidateKind::Sentence)
        );
    }
}

#[test]
fn an_empty_code_within_four_keys_falls_back_to_a_sentence() {
    let mut engine = sentence_engine();
    // `ggaa` 码表里没有：读成 五 gg + 式 aa
    let query = {
        engine.set_input("ggaa");
        engine.query().unwrap()
    };
    assert_eq!(query.candidates.items[0].text, "五式");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Sentence);
}

#[test]
fn an_unfinished_last_code_is_read_by_prefix() {
    let mut engine = sentence_engine();
    engine.set_input("ggggkh");
    let query = engine.query().unwrap();
    let first = &query.candidates.items[0];
    assert_eq!(first.kind, CandidateKind::Sentence);
    // `kh` 本身不是编码，按前缀读成 中（khk）；整句吃掉整段
    assert_eq!(first.text, "王中");
    assert_eq!(query.marked_text(), "gggg'kh");
    engine.commit(&first.clone());
    assert!(engine.composition().is_empty());
}

#[test]
fn a_key_outside_the_table_gives_no_sentence() {
    let mut engine = sentence_engine();
    // `z` 不在首位时只是个普通字母，码表里没有它：路径走不通，不出整句
    assert!(wubi_texts(&mut engine, "ggggzz").is_empty());
}

#[test]
fn committing_the_sentence_records_transitions_word_by_word() {
    let shared = Arc::new(Mutex::new((Vec::new(), sentence::UserNgram::default())));
    let learner = WordLearner {
        shared: Arc::clone(&shared),
        ..WordLearner::default()
    };
    let mut engine = engine().with_learner(Box::new(learner));
    let table = Dictionary::parse(crate::wubi::tests::TABLE).unwrap();
    engine.set_wubi(Some(WubiScheme::new(
        Variant::Wubi86,
        table,
        Options {
            sentence: true,
            ..Options::default()
        },
    )));
    engine.set_phonetic(false);
    engine.set_input("ggggkhlg");
    let first = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(engine.commit(&first), "王中国");
    assert!(engine.composition().is_empty());
    let shared = shared.lock().unwrap();
    assert_eq!(shared.1.pair(None, "王"), 1);
    assert_eq!(shared.1.pair(Some("王"), "中国"), 1);
    // 整句不是一个词：不造词
    assert!(shared.0.is_empty());
}

#[test]
fn picking_the_head_word_leaves_the_rest_for_the_next_sentence() {
    let mut engine = sentence_engine();
    engine.set_input("ggggkhlgjghu");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "王中国是");
    let wang = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "王")
        .unwrap()
        .clone();
    assert_eq!(engine.commit(&wang), "王");
    assert_eq!(engine.composition().text(), "khlgjghu");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "中国是");
}

#[test]
fn sentence_is_off_by_default() {
    let mut engine = wubi_engine();
    // 缺省照旧：满四码自动上屏
    assert_eq!(type_keys(&mut engine, "gggg"), "王");
}
