//! 候选窗口的一行：[`Candidate`] → 渲染器的 [`Row`]（序号、候选词、annotation 片段），与 macOS 端 `candidates/row.rs` 一致。
//! GDI 画法也用同一个类型。

use glimmer_core::{Candidate, CandidateKind};
use glimmer_render::{Row, Tone};

/// `position` 是页内下标（从 0 起）。
pub(crate) fn from_candidate(position: usize, candidate: &Candidate) -> Row {
    let mut annotation = Vec::new();
    if let Some(reading) = &candidate.reading {
        annotation.push((reading.clone(), Tone::Gloss));
    }
    if let Some(translation) = &candidate.translation {
        for (i, sense) in translation.senses().iter().enumerate() {
            if i > 0 || !annotation.is_empty() {
                annotation.push((" · ".to_owned(), Tone::Faint));
            }
            if let Some(pos) = sense.part_of_speech {
                annotation.push((format!("{pos} "), Tone::Faint));
            }
            let tone = if sense.fresh {
                Tone::Fresh
            } else {
                Tone::Gloss
            };
            for segment in sense.furigana() {
                annotation.push((segment.text, tone));
                if let Some(reading) = segment.reading {
                    annotation.push((format!("({reading})"), Tone::Faint));
                }
            }
        }
    }
    Row {
        index: (position + 1).to_string(),
        text: candidate.text.clone(),
        annotation,
        cloud: candidate.kind == CandidateKind::Cloud,
    }
}
