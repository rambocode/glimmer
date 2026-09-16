//! 悬浮状态条（Windows）：几格并排的小条 `[中 / 英][，。/ ,.][⚙]`，每格文字居中、格间一条细线，圆角背景加阴影。
//! macOS 用菜单栏状态项，没有这一块。

mod cell;
mod rendered;

pub use cell::StatusCell;
pub use rendered::RenderedStatus;

use super::{Metrics, Rendered, Renderer};
use crate::canvas::Canvas;
use crate::error::RenderError;
use crate::gear::draw_gear;
use crate::shadow::Shadow;
use crate::theme::Theme;

/// 齿轮图标边长（点）。
const GEAR_SIZE: f32 = 15.0;

/// 格间细线的宽度（点）。
const SEPARATOR_WIDTH: f32 = 1.0;

impl Renderer {
    /// 画状态条。每格宽 = 内容宽 + 两侧内边距，高 = 候选词行高 + 内边距；返回位图与各格右边界（供点击命中）。
    pub fn render_status(
        &mut self,
        cells: &[StatusCell],
        theme: &Theme,
        scale: f32,
        shadow: Option<&Shadow>,
    ) -> Result<RenderedStatus, RenderError> {
        let metrics = Metrics { theme, scale };
        let padding = metrics.padding();
        let line_height = metrics.px(theme.text_font.line_height);
        let mut widths: Vec<f32> = cells
            .iter()
            .map(|cell| self.status_cell_width(cell, &metrics) + padding * 2.0)
            .collect();
        let total: f32 = widths.iter().sum();
        let content_width = total.ceil();
        // 取整多出来的零头给最后一格，让最后一格的右边界正好是内容宽
        if let Some(last) = widths.last_mut() {
            *last += content_width - total;
        }
        let content_height = (line_height + padding).ceil();
        let margin = shadow.map_or(0.0, |s| metrics.px(s.margin()));
        let width = (content_width + margin * 2.0).ceil();
        let height = (content_height + margin * 2.0).ceil();
        let mut canvas = Canvas::new(width as u32, height as u32)?;
        let radius = metrics.corner_radius();
        if let Some(shadow) = shadow
            && let Some(content) =
                tiny_skia::Rect::from_xywh(margin, margin, content_width, content_height)
        {
            shadow.paint(&mut canvas, content, radius, scale);
        }
        canvas.fill_round_rect(
            margin,
            margin,
            content_width,
            content_height,
            radius,
            theme.colors.background,
        );
        let inset = padding / 2.0;
        let mut x = margin;
        let mut edges = Vec::with_capacity(cells.len());
        for (i, (cell, width)) in cells.iter().zip(&widths).enumerate() {
            if i > 0 {
                canvas.fill_rect(
                    x,
                    margin + inset,
                    metrics.px(SEPARATOR_WIDTH),
                    content_height - inset * 2.0,
                    theme.colors.pos,
                );
            }
            let slot = (x, margin, *width, content_height);
            self.draw_status_cell(&mut canvas, cell, &metrics, slot);
            x += width;
            edges.push(x - margin);
        }
        Ok(RenderedStatus {
            rendered: Rendered {
                pixmap: canvas.into_pixmap(),
                content_x: margin as u32,
                content_y: margin as u32,
                content_width: content_width as u32,
                content_height: content_height as u32,
                scale,
            },
            cell_edges: edges,
        })
    }

    /// 一格内容的宽度（像素，不含内边距）。
    fn status_cell_width(&mut self, cell: &StatusCell, m: &Metrics) -> f32 {
        match cell {
            StatusCell::Text { text, .. } => self.measure(text, &m.text_style()).width,
            StatusCell::Gear => m.px(GEAR_SIZE),
        }
    }

    /// 在 `slot = (x, y, 宽, 高)` 的格子里居中画一格。
    fn draw_status_cell(
        &mut self,
        canvas: &mut Canvas,
        cell: &StatusCell,
        m: &Metrics,
        slot: (f32, f32, f32, f32),
    ) {
        let (x, y, width, height) = slot;
        match cell {
            StatusCell::Text { text, emphasized } => {
                let color = if *emphasized {
                    m.theme.colors.cloud
                } else {
                    m.theme.colors.gloss
                };
                let style = m.style(m.theme.text_font, color);
                let size = self.measure(text, &style);
                let left = x + (width - size.width) / 2.0;
                let top = y + (height - size.height) / 2.0;
                self.draw_text(canvas, text, &style, left, top);
            }
            StatusCell::Gear => {
                let size = m.px(GEAR_SIZE);
                draw_gear(
                    canvas,
                    x + (width - size) / 2.0,
                    y + (height - size) / 2.0,
                    size,
                    m.theme.colors.gloss,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StatusCell;
    use crate::fonts::FontLibrary;
    use crate::renderer::Renderer;
    use crate::shadow::Shadow;
    use crate::theme::Theme;

    #[test]
    fn cells_have_increasing_edges_ending_at_content_width() {
        // 没有系统字体的环境（CI 容器）跳过
        let Ok(library) = FontLibrary::system("zh-CN") else {
            return;
        };
        let mut renderer = Renderer::new(library);
        let cells = [
            StatusCell::text("中 · 小鹤", true),
            StatusCell::text(",.", false),
            StatusCell::Gear,
        ];
        let out = renderer
            .render_status(&cells, &Theme::light(), 2.0, Some(&Shadow::mac_panel()))
            .unwrap();
        assert_eq!(out.cell_edges.len(), 3);
        assert!(out.cell_edges.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            out.cell_edges.last().map(|edge| edge.round() as u32),
            Some(out.rendered.content_width)
        );
        assert!(out.rendered.pixmap.width() > out.rendered.content_width);
        assert!(out.rendered.content_x > 0);
    }
}
