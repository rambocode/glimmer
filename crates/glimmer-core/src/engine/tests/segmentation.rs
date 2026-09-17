//! 同音节数的几种切分之间怎么挑（`dangao` 是 dang ao 还是 dan gao）：整句按哪种切分读、拼音行怎么显示。

use super::*;

/// 查一次，返回（最优切分的 `'` 连写，候选文本与种类）。
fn query_of(engine: &mut Engine, input: &str) -> (String, Vec<(String, CandidateKind)>) {
    engine.set_input(input);
    let query = engine.query().unwrap();
    let best = query.segmentations[0].joined("'");
    let items = query
        .candidates
        .items
        .into_iter()
        .map(|c| (c.text, c.kind))
        .collect();
    (best, items)
}

/// 后一个音节是零声母时，前一个音节吞掉 n / g 的切分（`dang ao`）只是切分器的贪心顺序，不该决定整句：
/// 同音节数的切分里按整句得分挑，蛋糕 / 蛋糕店 / 很干扰 不能被 当奥 / 当早点 / 能看到 压下去。
#[test]
fn sentence_reads_the_best_scoring_segmentation_not_the_greedy_one() {
    let dictionary = Dictionary::parse(
        "蛋糕\tdan gao\t50000\n蛋糕店\tdan gao dian\t3000\n当\tdang\t900000\n奥\tao\t20000\n\
         早点\tzao dian\t30000\n点\tdian\t100000\n单\tdan\t80000\n高\tgao\t90000\n\
         很\then\t800000\n干扰\tgan rao\t20000\n恒安\theng an\t3000\n能看到\tneng kan dao\t60000\n\
         绕\trao\t10000\n安\tan\t50000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);

    let (best, items) = query_of(&mut engine, "dangao");
    assert_eq!(best, "dan'gao");
    assert_eq!(items[0].0, "蛋糕");
    assert!(items.iter().all(|(text, _)| text != "当奥"));

    let (best, items) = query_of(&mut engine, "dangaodian");
    assert_eq!(best, "dan'gao'dian");
    assert_eq!(items[0].0, "蛋糕店");
    assert!(items.iter().all(|(text, _)| text != "当早点"));

    let (best, items) = query_of(&mut engine, "henganrao");
    assert_eq!(best, "hen'gan'rao");
    assert_eq!(items[0], ("很干扰".to_owned(), CandidateKind::Sentence));
    assert!(items.iter().all(|(text, _)| text != "能看到"));
}

/// 真是零声母的词照旧：方案 / 延安 / 长安 的整句得分本来就高，切分与首选不变。
#[test]
fn zero_initial_words_keep_their_segmentation() {
    let dictionary = Dictionary::parse(
        "方案\tfang an\t80000\n反感\tfan gan\t20000\n方\tfang\t300000\n案\tan\t40000\n\
         反\tfan\t100000\n感\tgan\t60000\n延安\tyan an\t30000\n沿岸\tyan an\t10000\n牙\tya\t50000\n\
         难\tnan\t200000\n长安\tchang an\t20000\n长\tchang\t500000\n产\tchan\t100000\n干\tgan\t200000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);
    for (input, segmentation, first) in [
        ("fangan", "fang'an", "方案"),
        ("yanan", "yan'an", "延安"),
        ("changan", "chang'an", "长安"),
    ] {
        let (best, items) = query_of(&mut engine, input);
        assert_eq!(best, segmentation, "{input}");
        assert_eq!(items[0].0, first, "{input}");
    }
    let (_, items) = query_of(&mut engine, "fangan");
    assert!(items.iter().take(3).any(|(text, _)| text == "反感"));
}

/// 上屏整句时重算的路径与查询时一致：蛋糕店 的转移能记进个人 n-gram（重算读成 当早点 就对不上、不记）。
#[test]
fn committing_the_sentence_replays_the_same_segmentation() {
    let dictionary = Dictionary::parse(
        "蛋糕\tdan gao\t50000\n店\tdian\t40000\n当\tdang\t900000\n奥\tao\t20000\n\
         早点\tzao dian\t30000\n点\tdian\t1000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_input("dangaodian");
    let query = engine.query().unwrap();
    let sentence = query.candidates.items[0].clone();
    assert_eq!(sentence.text, "蛋糕店");
    assert_eq!(sentence.kind, CandidateKind::Sentence);
    let words = engine.sentence_words(&sentence).unwrap();
    let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
    assert_eq!(texts, ["蛋糕", "店"]);
}

/// 光标按音节移动、按音节删走的是拼音行显示的那种切分：`hen'gan'rao` 时 ⌥← / ⌥→ 一格跨 rao / hen，不按 heng an 跳。
#[test]
fn syllable_cursor_moves_follow_the_displayed_segmentation() {
    let dictionary = Dictionary::parse(
        "很\then\t800000\n干扰\tgan rao\t20000\n恒安\theng an\t3000\n绕\trao\t10000\n\
         安\tan\t50000\n哼\theng\t1000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_input("henganrao");
    assert_eq!(engine.query().unwrap().marked_text(), "hen'gan'rao");
    // 光标在末尾：⌥← 跨过 rao
    assert!(engine.move_cursor_syllable_left());
    assert_eq!(engine.composition().cursor(), "hengan".len());
    // 回到开头：⌥→ 跨过 hen，不是 heng
    engine.move_cursor_home();
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), "hen".len());
    // 光标停在中间时拼音行后半段也按首选切分显示
    assert_eq!(engine.query().unwrap().rest, "gan'rao");
    // ⌥⌫ 删的也是显示出来的最后一个音节
    engine.set_input("henganrao");
    engine.query().unwrap();
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "hengan");
}

/// 查询记下的首选切分给光标移动复用；记下的与现算的一致，学习数据一变就作废。
#[test]
fn cursor_moves_reuse_the_segmentation_the_query_picked() {
    let dictionary =
        Dictionary::parse("蛋糕\tdan gao\t50000\n当\tdang\t900000\n奥\tao\t20000\n").unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_input("dangao");
    engine.query().unwrap();
    let remembered = engine.preferred_segmentations.borrow().clone();
    assert_eq!(remembered.len(), 1);
    assert_eq!(remembered[0].0, "dangao");
    assert_eq!(remembered[0].1.joined("'"), "dan'gao");
    // 复用的结果与清掉后现算的相同
    let (reused, _) = engine.preferred_segmentation("dangao").unwrap();
    engine.forget_span_cache();
    assert!(engine.preferred_segmentations.borrow().is_empty());
    let (fresh, tail) = engine.preferred_segmentation("dangao").unwrap();
    assert_eq!(reused, fresh);
    assert_eq!(tail, "");
}
