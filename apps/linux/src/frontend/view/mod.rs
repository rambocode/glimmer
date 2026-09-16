//! 把协议的 [`Frame`] 摊成 IBus 能直接画的视图：preedit 行（[`PreeditView`]）、候选表（[`CandidateView`]）、辅助文字。
//! 这一层只做「帧 → 要显示什么」，不碰 D-Bus 编码；编码在 `ibus::variant`。

mod candidates;
mod preedit;
mod row;

use glimmer_core::Candidate;
use glimmer_platform::LayoutMode;
use glimmer_platform::protocol::{Frame, PreeditKind};

pub use candidates::CandidateView;
pub use preedit::PreeditView;
pub use row::CandidateRow;

/// preedit 行：跳过被纠错划掉的原字母（IBus 的属性没有删除线），光标按跳过后的字符数重算；
/// 光标之后「只显示不参与候选」的拼音记成淡色区间。没有内容返回 `None`。
pub fn preedit_view(frame: &Frame) -> Option<PreeditView> {
    let mut text = String::new();
    let mut dimmed = Vec::new();
    let mut cursor = frame.cursor;
    let mut consumed = 0;
    for segment in &frame.preedit {
        let chars = segment.text.chars().count();
        if segment.kind == PreeditKind::Corrected {
            // 被跳过的字母若在光标之前，光标跟着前移。
            cursor -= chars.min(frame.cursor.saturating_sub(consumed));
            consumed += chars;
            continue;
        }
        let start = text.chars().count();
        text.push_str(&segment.text);
        if segment.kind == PreeditKind::Rest {
            dimmed.push(start..start + chars);
        }
        consumed += chars;
    }
    if text.is_empty() {
        return None;
    }
    let cursor = cursor.min(text.chars().count());
    Some(PreeditView {
        text,
        cursor,
        dimmed,
    })
}

/// 当前页候选；空页返回 `None`。
pub fn candidate_view(frame: &Frame) -> Option<CandidateView> {
    if frame.candidates.items.is_empty() {
        return None;
    }
    let rows = frame.candidates.items.iter().map(row).collect();
    Some(CandidateView {
        rows,
        highlight: frame.highlight,
        vertical: frame.layout == LayoutMode::Vertical,
    })
}

/// 辅助文字（候选表上方那一行）：删候选后的提示优先，其次整句补全（按 Tab 上屏）。
pub fn auxiliary_text(frame: &Frame) -> Option<String> {
    if let Some(notice) = &frame.notice {
        return Some(notice.clone());
    }
    frame
        .sentence
        .as_ref()
        .map(|sentence| format!("{sentence}  Tab"))
}

/// 一个候选：译文的各条释义用「; 」连起来当注解。
fn row(candidate: &Candidate) -> CandidateRow {
    let annotation = candidate.translation.as_ref().and_then(|translation| {
        let senses: Vec<&str> = translation
            .senses()
            .iter()
            .map(|sense| sense.text.as_str())
            .collect();
        (!senses.is_empty()).then(|| senses.join("; "))
    });
    CandidateRow {
        text: candidate.text.clone(),
        annotation,
    }
}

#[cfg(test)]
mod tests {
    use glimmer_platform::protocol::PreeditSegment;

    use super::*;

    fn segment(text: &str, kind: PreeditKind) -> PreeditSegment {
        PreeditSegment {
            text: text.to_owned(),
            kind,
        }
    }

    #[test]
    fn corrected_letters_are_skipped_and_cursor_shifts() {
        let frame = Frame {
            preedit: vec![
                segment("ni'", PreeditKind::Typed),
                segment("g", PreeditKind::Corrected),
                segment("hao", PreeditKind::Typed),
                segment("ma", PreeditKind::Rest),
            ],
            cursor: 7,
            ..Frame::default()
        };
        let view = preedit_view(&frame).unwrap();
        assert_eq!(view.text, "ni'haoma");
        assert_eq!(view.cursor, 6);
        assert_eq!(view.dimmed, vec![6..8]);
    }

    #[test]
    fn empty_frame_has_no_views() {
        let frame = Frame::default();
        assert!(preedit_view(&frame).is_none());
        assert!(candidate_view(&frame).is_none());
        assert!(auxiliary_text(&frame).is_none());
    }

    #[test]
    fn notice_wins_over_sentence() {
        let frame = Frame {
            sentence: Some("你好世界".to_owned()),
            ..Frame::default()
        };
        assert_eq!(auxiliary_text(&frame).as_deref(), Some("你好世界  Tab"));
        let frame = Frame {
            notice: Some("已删除".to_owned()),
            ..frame
        };
        assert_eq!(auxiliary_text(&frame).as_deref(), Some("已删除"));
    }
}
