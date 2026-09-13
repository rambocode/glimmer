//! 柔和阴影：取内容的圆角轮廓当遮罩，三遍盒式模糊近似高斯，染色后垫在内容底下，方向权重让光来自上方。
//!
//! macOS 壳用系统窗口阴影，这里的参数照它调；Windows 的分层窗口没有系统阴影，靠这个。

use tiny_skia::Rect;

use crate::canvas::Canvas;
use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    /// 模糊半径（点）。
    pub blur: f32,

    /// 向下偏移（点）。
    pub offset_y: f32,

    /// 颜色（含不透明度）。
    pub color: Color,
}

impl Shadow {
    /// 接近 macOS 弹出面板的系统阴影。
    pub const fn mac_panel() -> Self {
        Self {
            blur: 12.0,
            offset_y: 6.0,
            color: Color::gray(0, 90),
        }
    }

    /// 阴影在内容四周要留多宽的边（点）。
    pub(crate) fn margin(&self) -> f32 {
        self.blur * 2.0 + self.offset_y
    }

    /// 在 `canvas` 上给 `content` 这块圆角内容画阴影；坐标都是像素。
    pub(crate) fn paint(&self, canvas: &mut Canvas, content: Rect, radius: f32, scale: f32) {
        let (w, h) = (canvas.width(), canvas.height());
        let Ok(mut mask_canvas) = Canvas::new(w, h) else {
            return;
        };
        let dy = self.offset_y * scale;
        mask_canvas.fill_round_rect(
            content.x(),
            content.y() + dy,
            content.width(),
            content.height(),
            radius,
            Color::rgb(0, 0, 0),
        );
        let mut mask: Vec<u8> = mask_canvas
            .into_pixmap()
            .pixels()
            .iter()
            .map(|p| p.alpha())
            .collect();
        let radius_px = (self.blur * scale / 2.0).round().max(1.0) as usize;
        for _ in 0..3 {
            box_blur_horizontal(&mut mask, w as usize, h as usize, radius_px);
            box_blur_vertical(&mut mask, w as usize, h as usize, radius_px);
        }
        canvas.blend_mask(0, 0, w, h, &mask, self.color);
    }
}

fn box_blur_horizontal(data: &mut [u8], width: usize, height: usize, radius: usize) {
    let mut row_buf = vec![0u8; width];
    for row in 0..height {
        let line = &data[row * width..(row + 1) * width];
        box_blur_line(line, &mut row_buf, radius);
        data[row * width..(row + 1) * width].copy_from_slice(&row_buf);
    }
}

fn box_blur_vertical(data: &mut [u8], width: usize, height: usize, radius: usize) {
    let mut column = vec![0u8; height];
    let mut blurred = vec![0u8; height];
    for col in 0..width {
        for row in 0..height {
            column[row] = data[row * width + col];
        }
        box_blur_line(&column, &mut blurred, radius);
        for row in 0..height {
            data[row * width + col] = blurred[row];
        }
    }
}

/// 一维盒式模糊，窗口 `2r + 1`，边界外当 0；滑动窗口 O(n)。
fn box_blur_line(src: &[u8], dst: &mut [u8], radius: usize) {
    let n = src.len();
    let window = (2 * radius + 1) as u32;
    let mut sum: u32 = src.iter().take(radius + 1).map(|&v| u32::from(v)).sum();
    for i in 0..n {
        dst[i] = ((sum + window / 2) / window) as u8;
        if i + radius + 1 < n {
            sum += u32::from(src[i + radius + 1]);
        }
        if i >= radius {
            sum -= u32::from(src[i - radius]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_spreads_and_preserves_mass() {
        let src = [0, 0, 255, 0, 0];
        let mut dst = [0u8; 5];
        box_blur_line(&src, &mut dst, 1);
        assert_eq!(dst, [0, 85, 85, 85, 0]);
    }
}
