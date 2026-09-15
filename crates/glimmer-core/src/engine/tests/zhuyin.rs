use super::*;

#[test]
fn zhuyin_decodes_keys_before_lookup_and_shows_zhuyin_symbols() {
    let mut engine = engine();
    engine.set_zhuyin_mode(true);

    // 1: ㄅ, j: ㄨ, 4: ˋ -> 不 (bu)
    engine.set_input("1j4");

    let query = engine.query().unwrap();
    // 顯示上屏前的標記文字應該是注音符號
    assert_eq!(query.marked_text(), "ㄅㄨˋ");
    assert_eq!(query.marked_cursor(), 3);
    assert!(query.decoded_keys);

    // 驗證候選詞裡面包含「不」（拼音為 bu）
    let has_bu = query.candidates.items.iter().any(|c| c.text == "不");
    assert!(has_bu, "Candidate '不' should be present");

    // 驗證 take_raw() 回傳注音符號（對應修改的第 1 條規則）
    assert_eq!(engine.take_raw(), "ㄅㄨˋ");
}
