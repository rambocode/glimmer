//! 注音音节拼写规则与声调处理。
use super::layout::Component;

/// 暫存單個注音音節的組件
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZhuyinSyllable {
    /// 聲母（如 ㄅ、ㄆ）
    pub initial: Option<char>,
    /// 介母（如 ㄧ、ㄨ、ㄩ）
    pub medial: Option<char>,
    /// 韻母（如 ㄚ、ㄛ、ㄜ）
    pub final_: Option<char>,
    /// 聲調（如 1、2、3、4，以及對應的字元如 ˊ、ˇ、ˋ、˙）
    pub tone: Option<(u8, char)>,
    /// 記錄產生此音節的原始按鍵
    pub keys: String,
}

impl ZhuyinSyllable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.initial.is_none()
            && self.medial.is_none()
            && self.final_.is_none()
            && self.tone.is_none()
    }

    /// 加入一個新組件，若已滿或不相容則回傳 false，表示該開啟新音節。
    /// （註：大千配置中，如果連打兩個聲母，後者通常覆蓋前者，或者視為新音節。這裡採取「遇到相同類型則開啟新音節」或「遇到聲母且已有其他組件則開啟新音節」的策略）
    pub fn push(&mut self, comp: Component, key: char) -> bool {
        match comp {
            Component::Initial(c) => {
                // 若已經有聲母、介音、韻母，或者有「非輕聲」的聲調，皆視為新音節
                if self.initial.is_some() || self.medial.is_some() || self.final_.is_some() {
                    return false;
                }
                if let Some((num, _)) = self.tone
                    && num != 5
                {
                    return false;
                }
                self.initial = Some(c);
            }
            Component::Medial(c) => {
                if self.medial.is_some() || self.final_.is_some() {
                    return false;
                }
                if let Some((num, _)) = self.tone
                    && num != 5
                {
                    return false;
                }
                self.medial = Some(c);
            }
            Component::Final(c) => {
                if self.final_.is_some() {
                    return false;
                }
                if let Some((num, _)) = self.tone
                    && num != 5
                {
                    return false;
                }
                self.final_ = Some(c);
            }
            Component::Tone(num, c) => {
                if self.tone.is_some() {
                    return false;
                }
                // 如果是第一聲 (空格)，且前面完全沒有東西，則不應該視為聲調（或者視為無效）
                // 為了相容輕聲先打，只有當前面沒東西且不是輕聲時才拒絕
                if self.initial.is_none()
                    && self.medial.is_none()
                    && self.final_.is_none()
                    && num != 5
                {
                    return false;
                }
                self.tone = Some((num, c));
            }
        }
        self.keys.push(key);
        true
    }

    /// 將目前的注音音節轉換為標準漢語拼音字串，例如 `ㄅㄨ` -> `bu`
    pub fn to_pinyin(&self) -> String {
        // 這是一個簡化的注音轉拼音實作。
        // 未來可加入更嚴格的防呆，例如不可能出現的組合。

        let i_str = match self.initial {
            Some('ㄅ') => "b",
            Some('ㄆ') => "p",
            Some('ㄇ') => "m",
            Some('ㄈ') => "f",
            Some('ㄉ') => "d",
            Some('ㄊ') => "t",
            Some('ㄋ') => "n",
            Some('ㄌ') => "l",
            Some('ㄍ') => "g",
            Some('ㄎ') => "k",
            Some('ㄏ') => "h",
            Some('ㄐ') => "j",
            Some('ㄑ') => "q",
            Some('ㄒ') => "x",
            Some('ㄓ') => "zh",
            Some('ㄔ') => "ch",
            Some('ㄕ') => "sh",
            Some('ㄖ') => "r",
            Some('ㄗ') => "z",
            Some('ㄘ') => "c",
            Some('ㄙ') => "s",
            _ => "",
        };

        let has_i = self.initial.is_some();
        let is_jqx = matches!(self.initial, Some('ㄐ') | Some('ㄑ') | Some('ㄒ'));

        let m_str = self.medial.unwrap_or('\0');
        let f_str = self.final_.unwrap_or('\0');

        let mut pinyin = String::new();

        // 特殊處理：ㄓㄔㄕㄖㄗㄘㄙ 單獨出現時，拼音有 'i'，
        // 但由於查詞引擎對於前綴也能查，如果只有聲母，直接回傳聲母即可，
        // 除非它是一個完整的音節且後面有聲調。如果使用者輸入 'ㄓ' (zh)，
        // 給引擎 'zh' 即可查出 'zhe', 'zhi' 等等。
        if m_str == '\0' && f_str == '\0' {
            if has_i {
                // 如果已經有聲調，代表使用者打完了，必須補上 i（針對ㄓㄔㄕㄖㄗㄘㄙ）
                // 不過大千注音中，打完 ㄓ 必須按 空白鍵，才會變成 zhi
                if self.tone.is_some()
                    && matches!(
                        self.initial,
                        Some('ㄓ')
                            | Some('ㄔ')
                            | Some('ㄕ')
                            | Some('ㄖ')
                            | Some('ㄗ')
                            | Some('ㄘ')
                            | Some('ㄙ')
                    )
                {
                    return format!("{}i", i_str);
                }
                return i_str.to_string();
            }
            return String::new(); // empty
        }

        pinyin.push_str(i_str);

        // 韻母轉換邏輯
        match (m_str, f_str) {
            // 無介音
            ('\0', 'ㄚ') => pinyin.push('a'),
            ('\0', 'ㄛ') => pinyin.push('o'),
            ('\0', 'ㄜ') => pinyin.push('e'),
            ('\0', 'ㄝ') => pinyin.push('e'),
            ('\0', 'ㄞ') => pinyin.push_str("ai"),
            ('\0', 'ㄟ') => pinyin.push_str("ei"),
            ('\0', 'ㄠ') => pinyin.push_str("ao"),
            ('\0', 'ㄡ') => pinyin.push_str("ou"),
            ('\0', 'ㄢ') => pinyin.push_str("an"),
            ('\0', 'ㄣ') => pinyin.push_str("en"),
            ('\0', 'ㄤ') => pinyin.push_str("ang"),
            ('\0', 'ㄥ') => pinyin.push_str("eng"),
            ('\0', 'ㄦ') => pinyin.push_str("er"),

            // 介音 ㄧ
            ('ㄧ', '\0') => pinyin.push_str(if has_i { "i" } else { "yi" }),
            ('ㄧ', 'ㄚ') => pinyin.push_str(if has_i { "ia" } else { "ya" }),
            ('ㄧ', 'ㄛ') => pinyin.push_str(if has_i { "io" } else { "yo" }),
            ('ㄧ', 'ㄝ') => pinyin.push_str(if has_i { "ie" } else { "ye" }),
            ('ㄧ', 'ㄞ') => pinyin.push_str(if has_i { "iai" } else { "yai" }),
            ('ㄧ', 'ㄠ') => pinyin.push_str(if has_i { "iao" } else { "yao" }),
            ('ㄧ', 'ㄡ') => pinyin.push_str(if has_i { "iu" } else { "you" }),
            ('ㄧ', 'ㄢ') => pinyin.push_str(if has_i { "ian" } else { "yan" }),
            ('ㄧ', 'ㄣ') => pinyin.push_str(if has_i { "in" } else { "yin" }),
            ('ㄧ', 'ㄤ') => pinyin.push_str(if has_i { "iang" } else { "yang" }),
            ('ㄧ', 'ㄥ') => pinyin.push_str(if has_i { "ing" } else { "ying" }),

            // 介音 ㄨ
            ('ㄨ', '\0') => pinyin.push_str(if has_i { "u" } else { "wu" }),
            ('ㄨ', 'ㄚ') => pinyin.push_str(if has_i { "ua" } else { "wa" }),
            ('ㄨ', 'ㄛ') => pinyin.push_str(if has_i { "uo" } else { "wo" }),
            ('ㄨ', 'ㄞ') => pinyin.push_str(if has_i { "uai" } else { "wai" }),
            ('ㄨ', 'ㄟ') => pinyin.push_str(if has_i { "ui" } else { "wei" }),
            ('ㄨ', 'ㄢ') => pinyin.push_str(if has_i { "uan" } else { "wan" }),
            ('ㄨ', 'ㄣ') => pinyin.push_str(if has_i { "un" } else { "wen" }),
            ('ㄨ', 'ㄤ') => pinyin.push_str(if has_i { "uang" } else { "wang" }),
            ('ㄨ', 'ㄥ') => pinyin.push_str(if has_i { "ong" } else { "weng" }),

            // 介音 ㄩ
            ('ㄩ', '\0') => pinyin.push_str(if !has_i {
                "yu"
            } else if is_jqx {
                "u"
            } else {
                "v"
            }),
            ('ㄩ', 'ㄝ') => pinyin.push_str(if !has_i {
                "yue"
            } else if is_jqx {
                "ue"
            } else {
                "ve"
            }),
            ('ㄩ', 'ㄢ') => pinyin.push_str(if !has_i {
                "yuan"
            } else if is_jqx {
                "uan"
            } else {
                "van"
            }),
            ('ㄩ', 'ㄣ') => pinyin.push_str(if !has_i {
                "yun"
            } else if is_jqx {
                "un"
            } else {
                "vn"
            }),
            ('ㄩ', 'ㄥ') => pinyin.push_str(if !has_i {
                "yong"
            } else if is_jqx {
                "iong"
            } else {
                "vong"
            }), // Actually jiong, qiong, xiong. No l/n yong.

            _ => {}
        }

        pinyin
    }

    /// 取得原始顯示字串（例如 "ㄅㄨˋ"）
    pub fn display_string(&self) -> String {
        let mut s = String::new();
        if let Some(c) = self.initial {
            s.push(c);
        }
        if let Some(c) = self.medial {
            s.push(c);
        }
        if let Some(c) = self.final_ {
            s.push(c);
        }
        if let Some((num, c)) = self.tone
            && num != 1
        {
            s.push(c); // 一聲空白不顯示
        }
        s
    }

    pub fn has_tone(&self) -> bool {
        self.tone.is_some()
    }
}
