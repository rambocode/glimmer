//! 五笔：共用的装配函数，用例按主题分文件（查询与提示、自动上屏、上屏与学习）。

mod auto_commit;
mod commit;
mod query;

use super::*;
use crate::wubi::{Options, Scheme as WubiScheme, Variant};

/// 拼音样例词库 + 五笔小码表的引擎。
fn wubi_engine() -> Engine {
    wubi_engine_with(Options::default())
}

fn wubi_engine_with(options: Options) -> Engine {
    let mut engine = engine();
    let table = Dictionary::parse(crate::wubi::tests::TABLE).unwrap();
    engine.set_wubi(Some(WubiScheme::new(Variant::Wubi86, table, options)));
    engine
}

/// 一键一键敲进去，每键之后照壳的协议先取自动上屏、有就上屏；返回上屏的文本。
fn type_keys(engine: &mut Engine, keys: &str) -> String {
    let mut committed = String::new();
    for c in keys.chars() {
        engine.push(c);
        if let Some(candidate) = engine.take_auto_commit() {
            committed.push_str(&engine.commit(&candidate));
        }
    }
    committed
}

fn wubi_texts(engine: &mut Engine, input: &str) -> Vec<String> {
    engine.set_input(input);
    texts_of(engine)
}
