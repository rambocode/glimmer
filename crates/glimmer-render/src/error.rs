//! 渲染器的错误。

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// 界面字体一个都没加载到，没法画字。
    #[error("no UI font could be loaded (tried {tried:?})")]
    NoUiFont { tried: Vec<PathBuf> },

    /// 位图尺寸为零或大到 tiny-skia 拒绝分配。
    #[error("invalid bitmap size {width}x{height}")]
    InvalidSize { width: u32, height: u32 },
}
