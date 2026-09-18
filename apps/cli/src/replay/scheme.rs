//! 回放期间按日志里记的方案装配引擎。
//!
//! 每条上屏的 `scheme` 字段写着当时用的是哪套方案（双拼的键、`zhuyin`、只用五笔的 `wubi86`、
//! 混输的 `xiaohe+wubi86`，全拼为空），回放要照着还原：拿拼音的读法去喂编码、或者反过来，
//! 算出来的命中率没有意义。整份日志通常只有一套方案，所以码表只在拼音条目与五笔条目之间来回挪，
//! 不重新读文件（`Engine::set_wubi` 收所有权）。

use glimmer_core::Engine;
use glimmer_core::wubi::Scheme as WubiScheme;
use glimmer_platform::Scheme;

/// 按日志里的方案串装配引擎，并替不用五笔的条目保管码表。
#[derive(Default)]
pub struct SchemeSwitcher {
    /// 拼音条目时从引擎上卸下来的五笔方案；五笔 / 混输条目再装回去。
    slot: Option<WubiScheme>,
}

impl SchemeSwitcher {
    /// 这条日志能不能回放：要用五笔的，装着的码表得正好是它那个版本（CLI 的 `--wubi`）。
    pub fn can_replay(&self, engine: &Engine, key: &str) -> bool {
        match split(key).1 {
            None => true,
            Some(wanted) => self.loaded(engine) == Some(wanted),
        }
    }

    /// 按 `key` 装配引擎。双拼 / 注音 / 拼音侧开关每次都设（便宜），码表只在两种条目之间挪。
    pub fn apply(&mut self, engine: &mut Engine, key: &str) {
        let (pinyin, wubi) = split(key);
        if wubi.is_some() {
            if engine.wubi().is_none() {
                engine.set_wubi(self.slot.take());
            }
        } else if let Some(taken) = engine.take_wubi() {
            self.slot = Some(taken);
        }
        engine.set_shuangpin(pinyin.shuangpin());
        engine.set_zhuyin_mode(pinyin == Scheme::Zhuyin);
        // 拼音侧关掉且五笔在场才是「只用形码」；没有五笔时拼音一定得开着
        engine.set_phonetic(pinyin.is_on() || wubi.is_none());
    }

    /// 手上这份码表是哪个版本的方案键（引擎上装着的，或暂存在这里的）。
    fn loaded(&self, engine: &Engine) -> Option<&'static str> {
        engine
            .wubi()
            .map(WubiScheme::key)
            .or_else(|| self.slot.as_ref().map(|scheme| scheme.key()))
    }
}

/// 日志里的方案串拆成「拼音侧方案 + 五笔的方案键」：
/// `"xiaohe+wubi86"` → 小鹤 + 86；`"wubi98"` → 拼音关 + 98；`""` → 全拼 + 不用五笔。
/// 空串与认不出来的写法都按全拼（老日志里没有这个字段）。
fn split(key: &str) -> (Scheme, Option<&str>) {
    if let Some((pinyin, wubi)) = key.split_once('+') {
        return (pinyin.parse().unwrap_or_default(), Some(wubi));
    }
    if key.starts_with("wubi") {
        return (Scheme::Off, Some(key));
    }
    (key.parse().unwrap_or_default(), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glimmer_core::ShuangpinScheme;

    #[test]
    fn scheme_strings_split_into_the_two_axes() {
        // 老日志没有 scheme 字段，解出来是全拼、不用五笔
        assert_eq!(split(""), (Scheme::Pinyin, None));
        assert_eq!(split("没见过的写法"), (Scheme::Pinyin, None));
        assert_eq!(
            split("xiaohe"),
            (Scheme::Shuangpin(ShuangpinScheme::Xiaohe), None)
        );
        // 只用五笔
        assert_eq!(split("wubi86"), (Scheme::Off, Some("wubi86")));
        assert_eq!(split("wubixsj"), (Scheme::Off, Some("wubixsj")));
        // 混输：拼音侧那部分照解，五笔跟着开
        assert_eq!(
            split("xiaohe+wubi98"),
            (Scheme::Shuangpin(ShuangpinScheme::Xiaohe), Some("wubi98"))
        );
        assert_eq!(split("pinyin+wubi86"), (Scheme::Pinyin, Some("wubi86")));
        assert_eq!(split("zhuyin+wubi86"), (Scheme::Zhuyin, Some("wubi86")));
    }

    #[test]
    fn entries_that_need_a_table_are_skipped_without_one() {
        use glimmer_core::wubi::{Options, Variant};
        use glimmer_dictionary::Dictionary;

        let mut engine = Engine::new(Dictionary::parse("开\tkai\t100\n").unwrap());
        let switcher = SchemeSwitcher::default();
        assert!(switcher.can_replay(&engine, ""));
        assert!(switcher.can_replay(&engine, "xiaohe"));
        assert!(!switcher.can_replay(&engine, "wubi86"));
        assert!(!switcher.can_replay(&engine, "pinyin+wubi86"));

        engine.set_wubi(Some(WubiScheme::new(
            Variant::Wubi86,
            Dictionary::parse("一\tggll\t100\n").unwrap(),
            Options::default(),
        )));
        assert!(switcher.can_replay(&engine, "wubi86"));
        // 版本对不上的照样跳过
        assert!(!switcher.can_replay(&engine, "wubi98"));
    }

    #[test]
    fn switching_moves_the_table_on_and_off_the_engine() {
        use glimmer_core::wubi::{Options, Variant};
        use glimmer_dictionary::Dictionary;

        let mut engine = Engine::new(Dictionary::parse("开\tkai\t100\n").unwrap());
        engine.set_wubi(Some(WubiScheme::new(
            Variant::Wubi86,
            Dictionary::parse("一\tggll\t100\n").unwrap(),
            Options::default(),
        )));
        let mut switcher = SchemeSwitcher::default();

        switcher.apply(&mut engine, "wubi86");
        assert!(!engine.is_phonetic());
        engine.set_input("ggll");
        assert_eq!(engine.query().unwrap().candidates.items[0].text, "一");

        // 混输：码表还在，拼音侧也开着
        switcher.apply(&mut engine, "pinyin+wubi86");
        assert!(engine.is_phonetic() && engine.wubi_mode());
        engine.set_input("kai");
        assert_eq!(engine.query().unwrap().candidates.items[0].text, "开");

        // 切回全拼：码表卸到保管处，同一串编码不再出中文候选
        switcher.apply(&mut engine, "");
        assert!(!engine.wubi_mode());
        engine.set_input("ggll");
        let texts: Vec<String> = engine
            .query()
            .map(|q| q.candidates.items.into_iter().map(|c| c.text).collect())
            .unwrap_or_default();
        assert!(!texts.contains(&"一".to_owned()));
        // 再切回五笔：还是原来那份码表
        assert!(switcher.can_replay(&engine, "wubi86"));
    }
}
