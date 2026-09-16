//! 壳的帧类型 → 渲染器的帧类型。两边字段一一对应，spike 定型后壳直接用渲染器的类型，这层就没了。

use crate::candidates::frame::Frame;
use crate::candidates::preedit::{Preedit, PreeditStyle};
use crate::candidates::row::{Row, Tone};

pub(super) fn frame(frame: &Frame) -> glimmer_render::Frame {
    glimmer_render::Frame {
        preedit: frame.preedit.as_ref().map(preedit),
        rows: frame.rows.iter().map(row).collect(),
        highlighted: Some(frame.highlighted),
        footer: frame.footer.clone(),
        sentence: frame.sentence.clone(),
        status: frame.status.clone(),
    }
}

fn preedit(preedit: &Preedit) -> glimmer_render::Preedit {
    glimmer_render::Preedit {
        segments: preedit
            .segments
            .iter()
            .map(|segment| glimmer_render::PreeditSegment {
                text: segment.text.clone(),
                style: match segment.style {
                    PreeditStyle::Typed => glimmer_render::PreeditStyle::Typed,
                    PreeditStyle::Rest => glimmer_render::PreeditStyle::Rest,
                    PreeditStyle::Struck => glimmer_render::PreeditStyle::Struck,
                },
            })
            .collect(),
        cursor: preedit.cursor,
    }
}

fn row(row: &Row) -> glimmer_render::Row {
    glimmer_render::Row {
        index: row.index.clone(),
        text: row.text.clone(),
        annotation: row
            .annotation
            .iter()
            .map(|(text, tone)| {
                let tone = match tone {
                    Tone::Gloss => glimmer_render::Tone::Gloss,
                    Tone::Fresh => glimmer_render::Tone::Fresh,
                    Tone::Faint => glimmer_render::Tone::Faint,
                };
                (text.clone(), tone)
            })
            .collect(),
        cloud: row.cloud,
    }
}
