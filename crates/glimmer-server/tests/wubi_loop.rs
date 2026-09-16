//! 五笔下的闭环测试：用样例词库 + 内嵌小码表（与 Core `engine/tests/wubi` 同一份样本）直接喂协议消息给 Router，
//! 看四码自动上屏、顶字、状态条方案名、配置热加载在 `KeyResult` 里怎么表达。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use glimmer_core::wubi::{Options, Scheme};
use glimmer_core::{Language, WubiVariant};
use glimmer_dictionary::Dictionary;
use glimmer_platform::Config;
use glimmer_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyOutcome, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use glimmer_server::dispatch::{StatusSink, StatusView};
use glimmer_server::{AssemblySpec, Router, RouterConfig, assembly};

const SESSION: SessionId = SessionId(1);

/// 小码表：`词\t编码\t词频`，一条编码整个是一个「音节」。
const TABLE: &str = "工\ta\t9000\n工\taaaa\t100\n式\taa\t8000\n王\tgggg\t9000\n一\tggll\t9000\n五\tgg\t7000\n玉\tgy\t6000\n主\tygd\t5000\n天\tgd\t7000\n是\tjghu\t9000\n中\tkhk\t9000\n中国\tkhlg\t8000\n国\tlgyi\t8000\n";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// 样例词库装的 Router，再把内嵌码表设成 86 五笔，开好一个会话。
fn wubi_router(options: Options, config: RouterConfig) -> Router {
    let root = repo_root();
    let mut engine = assembly::assemble(&AssemblySpec {
        glossary: Some((
            Language::English,
            root.join("assets/sample/glossary-en.tsv"),
        )),
        ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
    })
    .expect("assemble engine from sample data");
    let table = Dictionary::parse(TABLE).expect("parse inline wubi table");
    engine.set_wubi(Some(Scheme::new(WubiVariant::Wubi86, table, options)));
    let mut router = Router::new(engine, config);
    open_session(&mut router);
    router
}

fn open_session(router: &mut Router) {
    assert_eq!(
        router.handle(ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: PROTOCOL_VERSION,
        }),
        None
    );
}

fn letter(c: char) -> KeyEvent {
    KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), Default::default())
}

fn press(router: &mut Router, c: char) -> (KeyOutcome, Option<String>, Frame) {
    match router.handle(ClientMessage::Key {
        session: SESSION,
        event: letter(c),
    }) {
        Some(ServerMessage::KeyResult {
            outcome,
            commit,
            frame,
            ..
        }) => (outcome, commit, frame),
        other => panic!("expected KeyResult, got {other:?}"),
    }
}

/// 逐键敲，返回每一键的上屏文本与最后一帧。
fn type_keys(router: &mut Router, keys: &str) -> (Vec<Option<String>>, Frame) {
    let mut commits = Vec::new();
    let mut last = Frame::default();
    for c in keys.chars() {
        let (outcome, commit, frame) = press(router, c);
        assert_eq!(outcome, KeyOutcome::Consumed, "编码键 {c} 应被吃掉");
        commits.push(commit);
        last = frame;
    }
    (commits, last)
}

fn preedit(frame: &Frame) -> String {
    frame.preedit.iter().map(|s| s.text.as_str()).collect()
}

fn candidate_texts(frame: &Frame) -> Vec<&str> {
    frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect()
}

#[test]
fn four_codes_with_a_full_hit_commit_in_the_same_key_result() {
    let mut router = wubi_router(Options::default(), RouterConfig::default());
    let (commits, frame) = type_keys(&mut router, "ggg");
    assert_eq!(commits, vec![None, None, None]);
    assert_eq!(preedit(&frame), "ggg");
    assert!(
        candidate_texts(&frame).contains(&"王"),
        "三码应有提示候选 王：{:?}",
        candidate_texts(&frame)
    );

    // 第四码：上屏文本随本次 KeyResult 回去，帧为空（缓冲区已清、候选窗收起）
    let (outcome, commit, frame) = press(&mut router, 'g');
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("王"));
    assert!(frame.is_empty(), "自动上屏后应是空帧：{frame:?}");
}

#[test]
fn a_full_hit_then_a_new_key_starts_a_new_segment() {
    let mut router = wubi_router(Options::default(), RouterConfig::default());
    let (commits, frame) = type_keys(&mut router, "khlg");
    assert_eq!(commits.last().unwrap().as_deref(), Some("中国"));
    assert!(frame.is_empty());

    let (_, commit, frame) = press(&mut router, 'x');
    assert_eq!(commit, None);
    assert_eq!(preedit(&frame), "x", "新键开新段");
}

#[test]
fn topping_commits_the_old_segment_and_keeps_the_new_key() {
    // 关掉四码自动上屏：满四码留在缓冲区，第五键顶字——旧段首选上屏、新键留下，一次 KeyResult 表达
    let options = Options {
        auto_select: false,
        ..Options::default()
    };
    let mut router = wubi_router(options, RouterConfig::default());
    let (commits, frame) = type_keys(&mut router, "khlg");
    assert!(commits.iter().all(Option::is_none));
    assert_eq!(preedit(&frame), "khlg");
    assert_eq!(candidate_texts(&frame), ["中国"]);

    let (outcome, commit, frame) = press(&mut router, 'x');
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("中国"));
    assert_eq!(preedit(&frame), "x");
    assert!(frame.candidates.items.is_empty(), "x 没有命中：{frame:?}");
}

#[test]
fn a_key_with_no_hits_tops_the_prefix_candidate() {
    let mut router = wubi_router(Options::default(), RouterConfig::default());
    // `ggl` + `x`：`gglx` 什么都命中不了，旧段的提示候选 一 上屏，缓冲区剩 x
    let (commits, frame) = type_keys(&mut router, "gglx");
    assert_eq!(commits[3].as_deref(), Some("一"));
    assert_eq!(preedit(&frame), "x");
}

#[test]
fn space_and_digits_still_pick_candidates() {
    let mut router = wubi_router(Options::default(), RouterConfig::default());
    type_keys(&mut router, "gg");
    let (_, commit, frame) = press(&mut router, ' ');
    assert_eq!(commit.as_deref(), Some("五"), "二码首选是全码命中的 五");
    assert!(frame.is_empty());
}

/// 记录状态条调用：`Some(模式格文字)` 是显示、`None` 是收起。
#[derive(Clone, Default)]
struct RecordingStatus(Arc<Mutex<Vec<Option<String>>>>);

impl StatusSink for RecordingStatus {
    fn show_status(&self, view: StatusView) {
        let mode = if view.english {
            "英".to_owned()
        } else if view.zhuyin {
            "注".to_owned()
        } else {
            match view.scheme {
                Some(scheme) => format!("中 · {scheme}"),
                None => "中".to_owned(),
            }
        };
        self.0.lock().unwrap().push(Some(mode));
    }

    fn hide_status(&self) {
        self.0.lock().unwrap().push(None);
    }
}

#[test]
fn status_bar_shows_wubi_scheme_and_ignores_zhuyin() {
    let config = RouterConfig {
        status_enabled: true,
        zhuyin: true,
        ..RouterConfig::default()
    };
    let mut router = wubi_router(Options::default(), config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });

    assert_eq!(
        recorder.0.lock().unwrap().clone(),
        vec![Some("中 · 86 五笔".to_owned())]
    );
}

#[test]
fn hot_reload_switches_wubi_on_and_off_with_its_own_learning_dir() {
    // 码表目录：TSV 内容写成 wubi86.qj（Dictionary 按魔数认格式，不看扩展名）
    let dir = std::env::temp_dir().join(format!("glimmer-windows-wubi-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let data_dir = dir.join("data");
    let user_dir = dir.join("user");
    std::fs::create_dir_all(&data_dir).unwrap();
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(data_dir.join("wubi86.qj"), TABLE).unwrap();
    let config_path = dir.join("config.toml");
    std::fs::write(&config_path, "").unwrap();

    let root = repo_root();
    let engine = assembly::assemble(&AssemblySpec {
        user_dir: Some(user_dir.clone()),
        ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
    })
    .unwrap();
    let mut router = Router::new(engine, RouterConfig::default());
    router.watch_config(
        &Config::default(),
        config_path,
        None,
        Some(user_dir.clone()),
        Some(data_dir),
    );
    open_session(&mut router);
    assert_eq!(router.wubi_key(), None);

    // 开五笔：码表从目录里重开，四码自动上屏
    let mut config = Config::default();
    config.general.wubi = "86".to_owned();
    router.apply_config(&config);
    assert_eq!(router.wubi_key(), Some("wubi86"));
    let (commits, _) = type_keys(&mut router, "gggg");
    assert_eq!(commits[3].as_deref(), Some("王"));
    router.flush_learning();
    assert!(
        user_dir.join("wubi86").join("user-choices.tsv").is_file(),
        "五笔的选择记录应落在方案子目录"
    );
    assert!(
        !user_dir.join("user-choices.tsv").exists(),
        "拼音那份不该被五笔写到"
    );

    // 只改 [wubi] 选项：码表不动，四码不再自动上屏
    config.wubi.auto_select = false;
    router.apply_config(&config);
    let (commits, frame) = type_keys(&mut router, "gggg");
    assert!(commits.iter().all(Option::is_none));
    assert_eq!(preedit(&frame), "gggg");
    press(&mut router, ' ');

    // 关五笔：回到拼音，学习器换回拼音那份
    router.apply_config(&Config::default());
    assert_eq!(router.wubi_key(), None);
    let (_, frame) = type_keys(&mut router, "nihao");
    assert!(
        candidate_texts(&frame).contains(&"你好"),
        "拼音应恢复：{:?}",
        candidate_texts(&frame)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hot_reload_with_a_missing_table_leaves_pinyin_and_the_buffer_alone() {
    // 码表目录是空的：配置写着 wubi = "86" 也开不了；之后每次配置改动都不该再清组句、重建学习器
    let dir = std::env::temp_dir().join(format!(
        "glimmer-windows-wubi-missing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config_path = dir.join("config.toml");
    std::fs::write(&config_path, "").unwrap();

    let root = repo_root();
    let engine =
        assembly::assemble(&AssemblySpec::new(root.join("assets/sample/dict.tsv"))).unwrap();
    let mut router = Router::new(engine, RouterConfig::default());
    router.watch_config(
        &Config::default(),
        config_path,
        None,
        None,
        Some(dir.clone()),
    );
    open_session(&mut router);

    let mut config = Config::default();
    config.general.wubi = "86".to_owned();
    router.apply_config(&config);
    assert_eq!(router.wubi_key(), None, "码表缺失应按拼音");

    // 组句中再来两次无关的配置改动（改主题之类）：缓冲区还在、候选还是拼音的
    let (_, frame) = type_keys(&mut router, "ni");
    assert_eq!(preedit(&frame), "ni");
    for _ in 0..2 {
        router.apply_config(&config);
        let frame = match router.handle(ClientMessage::Poll { session: SESSION }) {
            Some(ServerMessage::Update { frame, .. }) => frame,
            other => panic!("expected Update, got {other:?}"),
        };
        assert_eq!(preedit(&frame), "ni", "配置重载不该清掉组句");
        assert!(
            candidate_texts(&frame).contains(&"你"),
            "仍是拼音候选：{:?}",
            candidate_texts(&frame)
        );
    }
    assert_eq!(router.wubi_key(), None);
    let _ = std::fs::remove_dir_all(&dir);
}
