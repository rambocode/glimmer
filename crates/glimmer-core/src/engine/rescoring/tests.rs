use std::time::{Duration, Instant};

use super::*;
use crate::sentence::SentenceWord;

/// 假打分器：偏爱某个文本，其余都给低分。
struct Prefers(&'static str);

impl SentenceScorer for Prefers {
    fn score(&self, _context: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }
}

fn path(text: &str, score: f64) -> Conversion {
    Conversion {
        text: text.to_owned(),
        syllables: Vec::new(),
        words: vec![SentenceWord {
            text: text.to_owned(),
            syllables: Vec::new(),
            placeholder: false,
        }],
        score,
        static_score: score,
        penalty: 0.0,
    }
}

fn engine() -> Engine {
    Engine::new(Dictionary::parse("开发\tkai fa\t9000\n").unwrap())
}

fn texts(paths: &[Conversion]) -> Vec<&str> {
    paths.iter().map(|p| p.text.as_str()).collect()
}

#[test]
fn sync_scorer_reorders_paths_in_place() {
    let engine = engine().with_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, PathUnit::Syllables);
    assert_eq!(texts(&paths), ["开放", "开饭"]);
    // λ 0.5：开饭 −10 + 0.5·(−20 + 10) = −15；开放 −11 + 0.5·(−1 + 11) = −6
    assert!((paths[0].score - -6.0).abs() < 1e-9);
    assert!((paths[1].score - -15.0).abs() < 1e-9);
    assert!(!engine.rescoring_pending());
}

#[test]
fn async_scorer_waits_for_the_shell_to_request_and_poll() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, PathUnit::Syllables);
    // 第一次：没分，顺序不动，记下要分的
    assert_eq!(texts(&paths), ["开饭", "开放"]);
    assert!(engine.rescoring_pending());
    assert!(engine.request_rescoring());
    assert!(!engine.rescoring_pending());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, PathUnit::Syllables);
    assert_eq!(texts(&paths), ["开放", "开饭"]);
    // 没有新的要打的就不发
    assert!(!engine.request_rescoring());
}

#[test]
fn a_changed_context_discards_the_cached_scores() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    engine.history_mut().record("今天");
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, PathUnit::Syllables);
    assert!(engine.request_rescoring());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
    // 上屏了别的字，前文变了：缓存作废，又得重新要
    engine.history_mut().record("很好");
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, PathUnit::Syllables);
    assert_eq!(texts(&paths), ["开饭", "开放"]);
    assert!(engine.rescoring_pending());
}

#[test]
fn the_shell_context_wins_over_session_history() {
    let mut engine = engine().with_sentence_scorer(Box::new(Prefers("开放")), None, None, Some(4));
    engine.history_mut().record("本会话上屏的历史");
    assert_eq!(engine.rescoring_context(), "屏的历史");
    engine.set_rescoring_context(Some("应用里光标前的文本".to_owned()));
    assert_eq!(engine.rescoring_context(), "前的文本");
    engine.set_rescoring_context(None);
    assert_eq!(engine.rescoring_context(), "屏的历史");
}

/// 给每个字一个分的假打分器：好字 −1，其余 −5。
struct CharScores(&'static str);

impl SentenceScorer for CharScores {
    fn score(&self, _context: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| {
                t.chars()
                    .map(|c| if self.0.contains(c) { -1.0 } else { -5.0 })
                    .sum()
            })
            .collect()
    }
}

fn words(parts: &[(&str, usize)], score: f64) -> Conversion {
    let words: Vec<SentenceWord> = parts
        .iter()
        .map(|(text, syllables)| SentenceWord {
            text: (*text).to_owned(),
            syllables: vec!["x".to_owned(); *syllables],
            placeholder: false,
        })
        .collect();
    Conversion {
        text: words.iter().map(|w| w.text.as_str()).collect(),
        syllables: words.iter().flat_map(|w| w.syllables.clone()).collect(),
        words,
        score,
        static_score: score,
        penalty: 0.0,
    }
}

/// 第二轮：一条路径改对了前半句、另一条改对了后半句，两处都对的那条不在前几条里，由重排拼出来并排到第一。
/// 异步时拼出来的那条晚一拍：第一拍按第一轮的顺序出，它的分到了再排上来。
#[test]
fn winning_replacements_are_combined_into_a_new_path() {
    let paths = || {
        vec![
            words(&[("次哭", 2), ("的", 1), ("声称", 2)], -20.0),
            words(&[("词库", 2), ("的", 1), ("声称", 2)], -21.0),
            words(&[("次哭", 2), ("的", 1), ("生成", 2)], -22.0),
        ]
    };
    let good = "词库的生成";
    let engine = engine().with_sentence_scorer(Box::new(CharScores(good)), Some(1.0), None, None);
    let mut sync = paths();
    engine.rescore_paths(&mut sync, PathUnit::Syllables);
    assert_eq!(sync[0].text, good);
    assert_eq!(sync.len(), 4);

    let mut engine =
        engine.with_async_sentence_scorer(Box::new(CharScores(good)), Some(1.0), None, None);
    let wait = |engine: &mut Engine| {
        assert!(engine.request_rescoring());
        let started = Instant::now();
        while !engine.poll_rescoring() {
            assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    let mut first = paths();
    engine.rescore_paths(&mut first, PathUnit::Syllables);
    wait(&mut engine);
    // 第一轮的分到了：三条按神经分排，拼出来的那条还没分，记下来等第二拍
    let mut second = paths();
    engine.rescore_paths(&mut second, PathUnit::Syllables);
    assert_eq!(second.len(), 3);
    assert_eq!(second[0].text, "词库的声称");
    assert!(engine.rescoring_pending());
    wait(&mut engine);
    let mut third = paths();
    engine.rescore_paths(&mut third, PathUnit::Syllables);
    assert_eq!(third[0].text, good);
    assert!(!engine.rescoring_pending());
}
