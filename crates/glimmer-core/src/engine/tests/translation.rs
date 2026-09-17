//! 译词读音开关：关掉后 `annotate` 只留译词，音标 / 假名不进候选。

use super::engine;
use crate::Language;
use crate::candidate::{PartOfSpeech, Sense, Translation};
use crate::engine::Translator;

/// 给「开发」回一条带音标的英文译词。
struct ReadingTranslator;

impl Translator for ReadingTranslator {
    fn language(&self) -> Language {
        Language::English
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        (text == "开发").then(|| {
            Translation::new(
                Language::English,
                vec![Sense {
                    part_of_speech: Some(PartOfSpeech::Verb),
                    text: "develop".into(),
                    reading: Some("dɪˈveləp".into()),
                    fresh: false,
                }],
            )
        })
    }
}

/// 缺省带读音；关掉开关后同一条译词的读音被去掉，译词本身还在。
#[test]
fn translation_reading_can_be_switched_off() {
    let mut engine = engine().with_translator(Box::new(ReadingTranslator));
    let reading = |engine: &mut crate::Engine| {
        engine.set_input("kaifa");
        let mut query = engine.query().unwrap();
        engine.annotate(&mut query.candidates);
        let translation = query.candidates.items[0].translation.clone().unwrap();
        let sense = &translation.senses()[0];
        (sense.text.clone(), sense.reading.clone())
    };
    assert!(engine.translation_reading());
    assert_eq!(
        reading(&mut engine),
        ("develop".to_owned(), Some("dɪˈveləp".to_owned()))
    );
    engine.set_translation_reading(false);
    assert_eq!(reading(&mut engine), ("develop".to_owned(), None));
}
