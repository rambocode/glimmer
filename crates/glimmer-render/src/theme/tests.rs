//! 主题的字号缩放。

use super::{DEFAULT_TEXT_SIZE, FontSpec, Theme};
use crate::fonts::FontLibrary;
use crate::frame::{Frame, Row, Tone};
use crate::layout::Layout;
use crate::renderer::Renderer;

#[test]
fn default_text_size_keeps_the_theme_unchanged() {
    assert_eq!(
        Theme::light().with_text_size(DEFAULT_TEXT_SIZE),
        Theme::light()
    );
    assert_eq!(Theme::dark().with_text_size(f32::NAN), Theme::dark());
    assert_eq!(Theme::dark().with_text_size(0.0), Theme::dark());
}

#[test]
fn text_size_scales_all_fonts_and_rounds_line_heights_up() {
    let theme = Theme::light().with_text_size(24.0);
    assert_eq!(theme.text_font, FontSpec::new(24.0, 29.0));
    assert_eq!(theme.annotation_font, FontSpec::new(18.0, 23.0));
    assert_eq!(theme.index_font, FontSpec::new(16.5, 21.0));
    // 间距不跟着变
    assert_eq!(theme.padding, Theme::light().padding);
    assert_eq!(theme.row_padding, Theme::light().row_padding);
    // 再换一次按目标字号算，不叠乘
    assert_eq!(theme.with_text_size(16.0).text_font.size, 16.0);
}

#[test]
fn larger_text_makes_the_window_taller_without_clipping_rows() {
    // 没有系统字体的环境（CI 容器）跳过
    let Ok(library) = FontLibrary::system("zh-CN") else {
        return;
    };
    let mut renderer = Renderer::new(library);
    let frame = Frame {
        rows: vec![
            Row {
                annotation: vec![("hello".to_owned(), Tone::Gloss)],
                ..Row::plain(0, "你好")
            },
            Row::plain(1, "你号"),
        ],
        highlighted: Some(0),
        ..Frame::default()
    };
    let height = |renderer: &mut Renderer, theme: &Theme| {
        renderer
            .render(&frame, Layout::Vertical, theme, 1.0, None)
            .unwrap()
            .content_height as f32
    };
    let small = height(&mut renderer, &Theme::light());
    let large_theme = Theme::light().with_text_size(24.0);
    let large = height(&mut renderer, &large_theme);
    assert!(large > small, "{small} → {large}");
    // 两行都放得下放大后的行高（加上下留白与窗口内边距）
    let rows = 2.0 * (large_theme.text_font.line_height + 2.0 * large_theme.row_padding);
    assert!(
        large >= rows + 2.0 * large_theme.padding,
        "{large} < {rows}"
    );
}
