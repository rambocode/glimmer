//! 「候选窗口」页：外观、排布、渲染引擎、字体（可搜索的列表）、文字大小、拼音显示位置。

use glimmer_platform::{
    CandidateRenderer, Config, DEFAULT_FONT_SIZE, LayoutMode, MAX_FONT_SIZE, MIN_FONT_SIZE,
    PreeditMode, ThemeMode,
};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSPopUpButton;

use crate::candidates::available_families;
use crate::preferences::controls::{note, row_popup, select};
use crate::preferences::font_picker::FontPicker;
use crate::preferences::layout::Layout;
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

pub struct CandidatesPage {
    /// 外观：跟随系统 / 浅色 / 深色。
    theme: Retained<NSPopUpButton>,

    /// 竖排 / 横排。
    layout_mode: Retained<NSPopUpButton>,

    /// 微明渲染器 / 系统绘制。
    renderer: Retained<NSPopUpButton>,

    /// 候选窗字体：搜索框 + 列表。
    font: FontPicker,

    /// 文字大小（候选词字号，点）。
    font_size: Retained<NSPopUpButton>,

    /// 拼音显示位置。
    preedit: Retained<NSPopUpButton>,
}

impl CandidatesPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let theme_titles: Vec<String> = ThemeMode::ALL
            .iter()
            .map(|t| t.label().to_owned())
            .collect();
        let theme = row_popup(layout, mtm, "外观", &theme_titles, Setting::Theme, target);
        let layout_titles: Vec<String> = LayoutMode::ALL
            .iter()
            .map(|l| l.label().to_owned())
            .collect();
        let layout_mode = row_popup(layout, mtm, "排布", &layout_titles, Setting::Layout, target);
        note(layout, mtm, "横排时只给高亮的候选显示译词。");
        let renderer_titles: Vec<String> = CandidateRenderer::ALL
            .iter()
            .map(|r| r.label().to_owned())
            .collect();
        let renderer = row_popup(
            layout,
            mtm,
            "渲染引擎",
            &renderer_titles,
            Setting::Renderer,
            target,
        );
        note(layout, mtm, "微明渲染器让候选窗口在各平台一致。");
        let font = FontPicker::build(layout, mtm, "字体", available_families(mtm));
        note(
            layout,
            mtm,
            "只对微明渲染器生效；没装的字体自动回到系统字体。",
        );
        let font_size_titles: Vec<String> = (MIN_FONT_SIZE..=MAX_FONT_SIZE)
            .map(|size| {
                if size == DEFAULT_FONT_SIZE {
                    format!("{size}（缺省）")
                } else {
                    size.to_string()
                }
            })
            .collect();
        let font_size = row_popup(
            layout,
            mtm,
            "文字大小",
            &font_size_titles,
            Setting::FontSize,
            target,
        );
        note(
            layout,
            mtm,
            "候选词的字号，译词与序号按比例跟着变；只对微明渲染器生效。",
        );
        let preedit_titles: Vec<String> = PreeditMode::ALL
            .iter()
            .map(|p| p.label().to_owned())
            .collect();
        let preedit = row_popup(
            layout,
            mtm,
            "拼音显示",
            &preedit_titles,
            Setting::Preedit,
            target,
        );
        note(
            layout,
            mtm,
            "「只在候选窗口」时正在敲的拼音不显示在应用里，终端或行内拼音显示不正常的应用可以选它。",
        );
        Self {
            theme,
            layout_mode,
            renderer,
            font,
            font_size,
            preedit,
        }
    }

    pub fn sync(&self, config: &Config) {
        let general = &config.general;
        select(
            &self.theme,
            ThemeMode::ALL.iter().position(|t| *t == general.theme),
        );
        select(
            &self.layout_mode,
            LayoutMode::ALL.iter().position(|l| *l == general.layout),
        );
        select(
            &self.renderer,
            CandidateRenderer::ALL
                .iter()
                .position(|r| *r == general.renderer),
        );
        self.font.sync(&general.font);
        select(
            &self.font_size,
            Some((general.font_size() - MIN_FONT_SIZE) as usize),
        );
        select(
            &self.preedit,
            PreeditMode::ALL.iter().position(|p| *p == general.preedit),
        );
    }
}
