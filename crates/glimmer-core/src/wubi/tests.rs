//! 五笔方案本身的测试：反查表、造词规则、版本解析。Engine 里的流程测试在 `engine/tests/wubi/`。

use super::*;
use glimmer_dictionary::Dictionary;

/// 与 `engine/tests/wubi/` 共用的小码表。
pub(crate) const TABLE: &str = "工\ta\t9000\n工\taaaa\t100\n式\taa\t8000\n王\tgggg\t9000\n一\tggll\t9000\n五\tgg\t7000\n玉\tgy\t6000\n主\tygd\t5000\n天\tgd\t6800\n是\tjghu\t9000\n中\tkhk\t9000\n中国\tkhlg\t8000\n国\tlgyi\t8000\n";

fn scheme() -> Scheme {
    Scheme::new(
        Variant::Wubi86,
        Dictionary::parse(TABLE).unwrap(),
        Options::default(),
    )
}

#[test]
fn reverse_table_keeps_the_longest_code_per_char() {
    let scheme = scheme();
    let reverse = scheme.reverse();
    assert_eq!(reverse.code('工'), Some("aaaa"));
    assert_eq!(reverse.code('王'), Some("gggg"));
    assert_eq!(reverse.code('一'), Some("ggll"));
    assert_eq!(reverse.code('中'), Some("khk"));
    assert_eq!(reverse.code('人'), None);
    // 多字词不进反查表
    assert_eq!(reverse.len(), 11);
}

#[test]
fn reverse_table_breaks_length_ties_by_frequency() {
    let dictionary = Dictionary::parse("土\tffff\t100\n土\tffgg\t900\n").unwrap();
    let reverse = Reverse::build(&dictionary);
    assert_eq!(reverse.code('土'), Some("ffgg"));
}

#[test]
fn encoder_follows_the_three_rules() {
    let scheme = scheme();
    let reverse = scheme.reverse();
    // 二字 AaAbBaBb
    assert_eq!(encode(&['中', '国'], reverse).as_deref(), Some("khlg"));
    // 三字 AaBaCaCb
    assert_eq!(
        encode(&['王', '中', '国'], reverse).as_deref(),
        Some("gklg")
    );
    // 四字及以上 AaBaCaZa
    assert_eq!(
        encode(&['王', '中', '国', '工'], reverse).as_deref(),
        Some("gkla")
    );
    assert_eq!(
        encode(&['王', '中', '国', '天', '工'], reverse).as_deref(),
        Some("gkla")
    );
    // 全码不够长（五 gg 没有第三位不要紧，二字只取前两位；主 ygd 与 五 gg 二字要各取两位可以）
    assert_eq!(encode(&['五', '主'], reverse).as_deref(), Some("ggyg"));
    // 查不到的字与单字造不出
    assert_eq!(encode(&['人', '工'], reverse), None);
    assert_eq!(encode(&['工'], reverse), None);
    assert_eq!(encode(&[], reverse), None);
}

#[test]
fn encoder_fails_when_a_code_is_too_short() {
    let dictionary = Dictionary::parse("一\tg\t100\n工\ta\t100\n").unwrap();
    let reverse = Reverse::build(&dictionary);
    assert_eq!(encode(&['一', '工'], &reverse), None);
}

#[test]
fn code_of_handles_single_and_multi_char_text() {
    let scheme = scheme();
    assert_eq!(scheme.code_of("王").as_deref(), Some("gggg"));
    assert_eq!(scheme.code_of("中国").as_deref(), Some("khlg"));
    assert_eq!(scheme.code_of("人"), None);
    assert_eq!(scheme.code_of(""), None);
    assert_eq!(scheme.key(), "wubi86");
}

#[test]
fn variant_parses_config_keys_and_scheme_keys() {
    assert_eq!("86".parse::<Variant>(), Ok(Variant::Wubi86));
    assert_eq!(" wubi98 ".parse::<Variant>(), Ok(Variant::Wubi98));
    assert_eq!("xsj".parse::<Variant>(), Ok(Variant::Xinshiji));
    assert_eq!("06".parse::<Variant>(), Ok(Variant::Xinshiji));
    assert_eq!(" XinShiJi ".parse::<Variant>(), Ok(Variant::Xinshiji));
    assert_eq!("wubixsj".parse::<Variant>(), Ok(Variant::Xinshiji));
    assert!("2000".parse::<Variant>().is_err());
    assert_eq!(Variant::Wubi86.data_file(), "wubi86.qj");
    assert_eq!(Variant::Wubi98.config_key(), "98");
    assert_eq!(Variant::Xinshiji.config_key(), "xsj");
    assert_eq!(Variant::Xinshiji.key(), "wubixsj");
    assert_eq!(Variant::Xinshiji.data_file(), "wubixsj.qj");
    assert_eq!(Variant::Xinshiji.label(), "新世纪五笔");
    assert_eq!(Variant::ALL.len(), 3);
}

#[test]
fn reverse_lookup_needs_a_key_after_z() {
    assert!(!is_reverse_lookup("z"));
    assert!(is_reverse_lookup("zzhong"));
    assert!(!is_reverse_lookup("ggll"));
    assert!(is_code_key('z') && !is_code_key('-'));
}
