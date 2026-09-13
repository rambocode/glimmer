//! Windows 的字体文件清单：Segoe UI、微软雅黑、Yu Gothic、Segoe UI Emoji（COLRv0）。

use std::path::PathBuf;

fn fonts_dir() -> PathBuf {
    std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("Fonts")
}

pub(super) fn ui_fonts() -> Vec<PathBuf> {
    vec![
        fonts_dir().join("segoeui.ttf"),
        fonts_dir().join("arial.ttf"),
    ]
}

pub(super) fn script_fonts(locale: &str) -> Vec<PathBuf> {
    let dir = fonts_dir();
    let mut fonts = Vec::new();
    if locale.starts_with("zh-TW") || locale.starts_with("zh-HK") {
        fonts.push(dir.join("msjh.ttc"));
    } else if !locale.starts_with("ja") {
        fonts.push(dir.join("msyh.ttc"));
    }
    fonts.push(dir.join("YuGothR.ttc"));
    fonts.push(dir.join("yugothm.ttc"));
    fonts
}

pub(super) fn emoji_fonts() -> Vec<PathBuf> {
    vec![fonts_dir().join("seguiemj.ttf")]
}
