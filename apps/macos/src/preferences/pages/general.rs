//! 「通用」页：学习语言、每页候选数、双拼 / 五笔方案、英文模式候选。

use glimmer_core::{Language, PunctuationMode, ShuangpinScheme};
use glimmer_platform::{Config, MAX_PAGE_SIZE};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton};

use crate::preferences::controls::{
    checkbox, language_label, note, row_checkbox, row_popup, select, set_checked,
};
use crate::preferences::layout::Layout;
use crate::preferences::setting::{Setting, WUBI_VARIANTS};
use crate::preferences::target::PreferencesTarget;

pub struct GeneralPage {
    /// 学习语言。
    learning_language: Retained<NSPopUpButton>,

    /// 每页候选数。
    page_size: Retained<NSPopUpButton>,

    /// 双拼方案（第 0 项是关）。
    shuangpin: Retained<NSPopUpButton>,

    /// 五笔版本（第 0 项是关）；开着时双拼菜单置灰。
    wubi: Retained<NSPopUpButton>,

    /// 五笔：敲满四码命中全码就自动上屏。
    wubi_auto_select: Retained<NSButton>,

    /// 五笔：逐键提示候选右侧显示完整编码。
    wubi_hint: Retained<NSButton>,

    /// 英文模式也给候选。
    english: Retained<NSButton>,

    /// 终端 / 编辑器里不给英文候选。
    english_off_in_apps: Retained<NSButton>,

    /// 中英混输时中文候选排在英文词前。
    chinese_first: Retained<NSButton>,

    /// 学习语言弹出菜单里各项对应的语言。
    languages: Vec<Language>,

    /// 默认中文标点模式。
    punctuation: Retained<NSPopUpButton>,

    /// 组句中敲标点怎么办。
    punctuation_mode: Retained<NSPopUpButton>,
}

impl GeneralPage {
    /// `languages` 是打进包里的释义表语言。
    pub fn build(
        layout: &mut Layout,
        mtm: MainThreadMarker,
        target: &PreferencesTarget,
        languages: &[Language],
    ) -> Self {
        // 最后一项是关
        let language_titles: Vec<String> = languages
            .iter()
            .map(|l| language_label(*l).to_owned())
            .chain(std::iter::once("不显示译文".to_owned()))
            .collect();
        let learning_language = row_popup(
            layout,
            mtm,
            "学习语言",
            &language_titles,
            Setting::LearningLanguage,
            target,
        );
        note(
            layout,
            mtm,
            "候选词右侧显示哪种语言的译词，只列出安装了释义表的语言；「不显示译文」同时关掉生词标记与释义兜底。",
        );
        let page_size_titles: Vec<String> = (1..=MAX_PAGE_SIZE).map(|n| n.to_string()).collect();
        let page_size = row_popup(
            layout,
            mtm,
            "每页候选数",
            &page_size_titles,
            Setting::PageSize,
            target,
        );
        let shuangpin_titles: Vec<String> = std::iter::once("关（全拼）".to_owned())
            .chain(ShuangpinScheme::ALL.iter().map(|s| s.label().to_owned()))
            .collect();
        let shuangpin = row_popup(
            layout,
            mtm,
            "双拼",
            &shuangpin_titles,
            Setting::Shuangpin,
            target,
        );
        note(
            layout,
            mtm,
            "开双拼后 v、u、i 是音节键，表达式与问字模式只能用 ? 开头进；微软、搜狗方案的 ; 键是 ing。",
        );
        let wubi_titles: Vec<String> = std::iter::once("关".to_owned())
            .chain(WUBI_VARIANTS.iter().map(|v| v.label().to_owned()))
            .collect();
        let wubi = row_popup(layout, mtm, "五笔", &wubi_titles, Setting::Wubi, target);
        note(
            layout,
            mtm,
            "开五笔后按码表出字，双拼与注音设置不再生效；z 开头是拼音反查，候选右侧注五笔码。",
        );
        let wubi_auto_select = checkbox(mtm, "四码自动上屏", Setting::WubiAutoSelect, target);
        row_checkbox(layout, &wubi_auto_select);
        let wubi_hint = checkbox(mtm, "显示编码提示", Setting::WubiHint, target);
        row_checkbox(layout, &wubi_hint);
        note(
            layout,
            mtm,
            "敲满四码且命中全码时首选直接上屏，不用按空格；编码提示是逐键提示候选右侧的完整编码。",
        );
        let punctuation = row_popup(
            layout,
            mtm,
            "默认中文标点",
            &["全角（，；：）".to_owned(), "半角（,;:）".to_owned()],
            Setting::FullWidthPunctuation,
            target,
        );
        note(
            layout,
            mtm,
            "仅影响标点，字母和数字保持半角；自定义短语原样输出。设置会保存。 ",
        );
        let mode_titles: Vec<String> = PunctuationMode::ALL
            .iter()
            .map(|m| m.label().to_owned())
            .collect();
        let punctuation_mode = row_popup(
            layout,
            mtm,
            "打拼音时敲标点",
            &mode_titles,
            Setting::PunctuationMode,
            target,
        );
        note(
            layout,
            mtm,
            "「进入英文直输」能直接打 hello, world 这样带标点的英文；「先上屏候选再出标点」是 nihao, 出「你好，」；「自动」按拼音切不切得开来定，切不开的 hello 直输、切得开的 nihao 上屏。翻页键不受影响。",
        );
        let english = checkbox(mtm, "英文模式也给候选", Setting::EnglishCandidates, target);
        row_checkbox(layout, &english);
        note(
            layout,
            mtm,
            "Tab 或方向键选词；空格、回车、标点仍原样上屏敲的字母，不选词时与直接打字一样。",
        );
        let english_off_in_apps = checkbox(
            mtm,
            "但在终端和代码编辑器里不给",
            Setting::EnglishCandidatesOffInApps,
            target,
        );
        row_checkbox(layout, &english_off_in_apps);
        note(
            layout,
            mtm,
            "终端、iTerm、Warp、Ghostty、VS Code、Cursor、Zed、JetBrains、Xcode 等，那里的候选窗口会挡住应用自己的补全；名单可在配置文件里改。",
        );
        let chinese_first = checkbox(
            mtm,
            "输入拼音时中文候选排在英文词前面",
            Setting::ChineseFirst,
            target,
        );
        row_checkbox(layout, &chinese_first);
        note(
            layout,
            mtm,
            "勾上后整段输入是英文词时（hello、key）英文词排第二，空格上屏的仍是中文；不勾（缺省）拼音不成立的输入英文词排第一。",
        );
        Self {
            learning_language,
            page_size,
            shuangpin,
            wubi,
            wubi_auto_select,
            wubi_hint,
            english,
            english_off_in_apps,
            chinese_first,
            languages: languages.to_vec(),
            punctuation,
            punctuation_mode,
        }
    }

    pub fn sync(&self, config: &Config) {
        let general = &config.general;
        select(
            &self.punctuation,
            Some(usize::from(!general.full_width_punctuation)),
        );
        select(
            &self.learning_language,
            if general.learning_language_off() {
                Some(self.languages.len())
            } else {
                self.languages
                    .iter()
                    .position(|l| l.code() == general.learning_language)
            },
        );
        select(
            &self.punctuation_mode,
            PunctuationMode::ALL
                .iter()
                .position(|m| *m == general.punctuation_mode),
        );
        select(&self.page_size, Some(general.page_size() - 1));
        select(
            &self.shuangpin,
            Some(general.shuangpin().map_or(0, |scheme| {
                ShuangpinScheme::ALL
                    .iter()
                    .position(|s| *s == scheme)
                    .map_or(0, |i| i + 1)
            })),
        );
        // 五笔与双拼互斥：五笔开着时 Core 忽略双拼，菜单也置灰说明这一点（注音只有配置文件能开，界面上没有控件）
        let wubi = general.wubi();
        select(
            &self.wubi,
            Some(wubi.map_or(0, |variant| {
                WUBI_VARIANTS
                    .iter()
                    .position(|v| *v == variant)
                    .map_or(0, |i| i + 1)
            })),
        );
        self.shuangpin.setEnabled(wubi.is_none());
        set_checked(&self.wubi_auto_select, config.wubi.auto_select);
        set_checked(&self.wubi_hint, config.wubi.hint);
        self.wubi_auto_select.setEnabled(wubi.is_some());
        self.wubi_hint.setEnabled(wubi.is_some());
        set_checked(&self.english, general.english_candidates);
        set_checked(
            &self.english_off_in_apps,
            config.apps.has_english_candidates_off(),
        );
        self.english_off_in_apps
            .setEnabled(general.english_candidates);
        set_checked(&self.chinese_first, general.chinese_first);
    }
}
