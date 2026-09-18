//! 五笔上屏与学习：按编码消耗、学习键、自动造词。

use super::*;

#[test]
fn commit_consumes_the_whole_code_and_records_by_code() {
    let learner = CountingLearner(HashMap::new());
    let mut engine = wubi_engine().with_learner(Box::new(learner));
    engine.set_input("gggg");
    let wang = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(wang.text, "王");
    assert_eq!(engine.commit(&wang), "王");
    assert!(engine.composition().is_empty());
    assert_eq!(engine.learner().weight("王"), 1);
    assert_eq!(engine.learner().choice_weight("gggg", "王"), 1);
    // 提示候选（编码比敲的长）吃完作用域
    engine.set_input("gg");
    let wang = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "王")
        .unwrap();
    engine.commit(&wang);
    assert!(engine.composition().is_empty());
    assert_eq!(engine.learner().choice_weight("gg", "王"), 1);
}

#[test]
fn consecutive_commits_in_one_buffer_form_a_word_with_the_encoded_code() {
    let shared = Arc::new(Mutex::new((Vec::new(), sentence::UserNgram::default())));
    let learner = WordLearner {
        shared: Arc::clone(&shared),
        ..WordLearner::default()
    };
    let table = "中\tkhk\t9000\n国\tlgyi\t8000\n王\tgggg\t9000\n";
    let mut engine = engine().with_learner(Box::new(learner));
    engine.set_wubi(Some(WubiScheme::new(
        Variant::Wubi86,
        Dictionary::parse(table).unwrap(),
        Options::default(),
    )));
    engine.set_phonetic(false);
    let select = |engine: &mut Engine, text: &str| {
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text)
            .unwrap();
        engine.commit(&candidate);
    };
    // 中 三码要自己选，国 四码自动上屏：连着上屏两次就造词
    for round in 1..=2 {
        engine.set_input("khk");
        select(&mut engine, "中");
        assert_eq!(type_keys(&mut engine, "lgyi"), "国");
        assert!(engine.composition().is_empty());
        assert_eq!(shared.lock().unwrap().0.len(), usize::from(round == 2));
        engine.note_passthrough('\n');
    }
    assert_eq!(shared.lock().unwrap().0, ["中国"]);
    // 造出的词按 AaAbBaBb 编码进用户词库，与码表一起被查到
    assert_eq!(wubi_texts(&mut engine, "khlg"), ["中国"]);
    engine.set_input("kh");
    let query = engine.query().unwrap();
    let zhongguo = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "中国")
        .unwrap();
    assert_eq!(zhongguo.reading.as_deref(), Some("khlg"));
    // 已经有了就不再造
    engine.set_input("khk");
    select(&mut engine, "中");
    assert_eq!(type_keys(&mut engine, "lgyi"), "国");
    assert_eq!(shared.lock().unwrap().0, ["中国"]);
}

#[test]
fn words_whose_chars_lack_full_codes_are_not_formed() {
    let shared = Arc::new(Mutex::new((Vec::new(), sentence::UserNgram::default())));
    let learner = WordLearner {
        shared: Arc::clone(&shared),
        ..WordLearner::default()
    };
    // 一 的全码只有一位，二字规则要取它的第二位：造不出
    let table = "一\tg\t9000\n国\tlgyi\t8000\n";
    let mut engine = engine().with_learner(Box::new(learner));
    engine.set_wubi(Some(WubiScheme::new(
        Variant::Wubi86,
        Dictionary::parse(table).unwrap(),
        Options::default(),
    )));
    engine.set_phonetic(false);
    for _ in 0..3 {
        engine.set_input("g");
        let yi = engine.query().unwrap().candidates.items[0].clone();
        assert_eq!(engine.commit(&yi), "一");
        assert_eq!(type_keys(&mut engine, "lgyi"), "国");
        engine.note_passthrough('\n');
    }
    assert!(shared.lock().unwrap().0.is_empty());
}

#[test]
fn raw_commit_of_an_unknown_code_is_not_learned_as_english() {
    let learner = CountingLearner(HashMap::new());
    let mut engine = wubi_engine().with_learner(Box::new(learner));
    engine.set_input("xxxx");
    assert_eq!(engine.take_raw(), "xxxx");
    assert_eq!(engine.learner().raw_count("xxxx"), 1);
    assert!(engine.learner().user_english().is_none());
}

#[test]
fn emoji_follow_the_word_and_consume_its_code() {
    let mut engine = wubi_engine().with_emoji(EmojiTable::parse("王\t👑\n").unwrap());
    engine.set_input("gggg");
    let query = engine.query().unwrap();
    let crown = query
        .candidates
        .items
        .iter()
        .find(|c| c.kind == CandidateKind::Emoji)
        .unwrap();
    assert_eq!(crown.syllables, ["gggg"]);
    engine.commit(crown);
    assert!(engine.composition().is_empty());
}
