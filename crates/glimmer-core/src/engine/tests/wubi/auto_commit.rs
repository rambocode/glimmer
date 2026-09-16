//! 五笔自动上屏：四码上屏、顶字、空码。

use super::*;

#[test]
fn four_codes_with_a_full_hit_commit_the_first_candidate() {
    let mut engine = wubi_engine();
    assert_eq!(type_keys(&mut engine, "ggg"), "");
    assert!(engine.take_auto_commit().is_none());
    engine.push('g');
    let pending = engine.take_auto_commit().unwrap();
    assert_eq!(pending.text, "王");
    assert!(engine.take_auto_commit().is_none(), "取一次就没了");
    assert_eq!(engine.commit(&pending), "王");
    assert!(engine.composition().is_empty());
}

#[test]
fn auto_select_can_be_turned_off() {
    let mut engine = wubi_engine_with(Options {
        auto_select: false,
        ..Options::default()
    });
    assert_eq!(type_keys(&mut engine, "gggg"), "");
    assert_eq!(engine.composition().text(), "gggg");
    assert_eq!(texts_of(&engine), ["王"]);
    // 关着自动上屏时敲第五键：满四码的旧段顶字上屏，新键开新段
    assert_eq!(type_keys(&mut engine, "a"), "王");
    assert_eq!(engine.composition().text(), "a");
}

#[test]
fn a_key_with_no_hits_tops_the_previous_segment() {
    let mut engine = wubi_engine();
    // `ggl` + `x`：`gglx` 什么都命中不了，旧段 `ggl` 的首选（提示候选 一）上屏，缓冲区剩 x
    assert_eq!(type_keys(&mut engine, "gglx"), "一");
    assert_eq!(engine.composition().text(), "x");
    engine.clear();
    // `gg` + `x`：旧段首选是全码的 五
    assert_eq!(type_keys(&mut engine, "ggx"), "五");
    assert_eq!(engine.composition().text(), "x");
}

#[test]
fn topping_after_a_full_four_code_commit_leaves_only_the_new_key() {
    let mut engine = wubi_engine();
    assert_eq!(type_keys(&mut engine, "ggggx"), "王");
    assert_eq!(engine.composition().text(), "x");
}

#[test]
fn unknown_four_codes_stay_until_the_next_key_replaces_them() {
    let mut engine = wubi_engine();
    assert_eq!(type_keys(&mut engine, "xxxx"), "");
    assert_eq!(engine.composition().text(), "xxxx");
    assert!(texts_of(&engine).is_empty());
    assert_eq!(type_keys(&mut engine, "g"), "");
    assert_eq!(engine.composition().text(), "g");
    // 空码回车 / 空格：原样上屏并记一笔
    engine.set_input("xxxx");
    assert_eq!(engine.take_raw(), "xxxx");
    assert!(engine.composition().is_empty());
}

#[test]
fn reverse_lookup_and_raw_segments_never_auto_commit() {
    let mut engine = wubi_engine();
    assert_eq!(type_keys(&mut engine, "zwang"), "");
    assert_eq!(engine.composition().text(), "zwang");
    engine.clear();
    assert_eq!(type_keys(&mut engine, "gg-l"), "");
    assert_eq!(engine.composition().text(), "gg-l");
}

#[test]
fn pending_commit_is_dropped_by_backspace_clear_and_the_next_key() {
    let mut engine = wubi_engine();
    for c in "gggg".chars() {
        engine.push(c);
    }
    assert!(engine.backspace());
    assert!(engine.take_auto_commit().is_none());
    engine.push('g');
    engine.clear();
    assert!(engine.take_auto_commit().is_none());
    // 顶字后壳没上屏而是清空：暂存的新键跟着丢
    for c in "gglx".chars() {
        engine.push(c);
    }
    engine.clear();
    assert!(engine.composition().is_empty());
    engine.push('a');
    assert_eq!(engine.composition().text(), "a");
}

#[test]
fn english_mode_keeps_wubi_out() {
    let mut engine = wubi_engine();
    engine.set_english_mode(true);
    // 四码不上屏，缓冲区原样是英文字母
    for c in "gggg".chars() {
        engine.push(c);
        assert!(engine.take_auto_commit().is_none());
    }
    assert_eq!(engine.composition().text(), "gggg");
    // 无命中也不顶字、不丢键
    engine.clear();
    for c in "ggx".chars() {
        engine.push(c);
        assert!(engine.take_auto_commit().is_none());
    }
    assert_eq!(engine.composition().text(), "ggx");
}

#[test]
fn auto_commit_refreshes_the_query_snapshot_for_the_input_log() {
    let mut engine = wubi_engine();
    for c in "ggg".chars() {
        engine.push(c);
        engine.query().unwrap();
    }
    engine.push('g');
    let pending = engine.take_auto_commit().unwrap();
    // 壳拿到就上屏、不会先 query：快照必须已是四码那次
    let snapshot = engine.last_query.borrow().clone().unwrap();
    assert_eq!(snapshot.scope, "gggg");
    assert_eq!(snapshot.candidates.first().map(String::as_str), Some("王"));
    assert_eq!(engine.commit(&pending), "王");
}
