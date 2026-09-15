//! 注音大千键盘布局。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    Initial(char),
    Medial(char),
    Final(char),
    Tone(u8, char),
}

impl Component {
    pub fn is_tone(&self) -> bool {
        matches!(self, Component::Tone(_, _))
    }

    pub fn display_char(&self) -> char {
        match self {
            Component::Initial(c) => *c,
            Component::Medial(c) => *c,
            Component::Final(c) => *c,
            Component::Tone(_, c) => *c,
        }
    }
}

/// 解析大千鍵盤的 ASCII 按鍵，轉為注音符號組件
pub fn map_key(key: char) -> Option<Component> {
    match key {
        // 聲母 (Initials)
        '1' => Some(Component::Initial('ㄅ')),
        'q' => Some(Component::Initial('ㄆ')),
        'a' => Some(Component::Initial('ㄇ')),
        'z' => Some(Component::Initial('ㄈ')),
        '2' => Some(Component::Initial('ㄉ')),
        'w' => Some(Component::Initial('ㄊ')),
        's' => Some(Component::Initial('ㄋ')),
        'x' => Some(Component::Initial('ㄌ')),
        'e' => Some(Component::Initial('ㄍ')),
        'd' => Some(Component::Initial('ㄎ')),
        'c' => Some(Component::Initial('ㄏ')),
        'r' => Some(Component::Initial('ㄐ')),
        'f' => Some(Component::Initial('ㄑ')),
        'v' => Some(Component::Initial('ㄒ')),
        '5' => Some(Component::Initial('ㄓ')),
        't' => Some(Component::Initial('ㄔ')),
        'g' => Some(Component::Initial('ㄕ')),
        'b' => Some(Component::Initial('ㄖ')),
        'y' => Some(Component::Initial('ㄗ')),
        'h' => Some(Component::Initial('ㄘ')),
        'n' => Some(Component::Initial('ㄙ')),

        // 介音 (Medials)
        'u' => Some(Component::Medial('ㄧ')),
        'j' => Some(Component::Medial('ㄨ')),
        'm' => Some(Component::Medial('ㄩ')),

        // 韻母 (Finals)
        '8' => Some(Component::Final('ㄚ')),
        'i' => Some(Component::Final('ㄛ')),
        'k' => Some(Component::Final('ㄜ')),
        ',' => Some(Component::Final('ㄝ')),
        '9' => Some(Component::Final('ㄞ')),
        'o' => Some(Component::Final('ㄟ')),
        'l' => Some(Component::Final('ㄠ')),
        '.' => Some(Component::Final('ㄡ')),
        '0' => Some(Component::Final('ㄢ')),
        'p' => Some(Component::Final('ㄣ')),
        ';' => Some(Component::Final('ㄤ')),
        '/' => Some(Component::Final('ㄥ')),
        '-' => Some(Component::Final('ㄦ')),

        // 聲調 (Tones)
        ' ' => Some(Component::Tone(1, ' ')), // 第一聲預設為空白不顯示，或依需求處理
        '6' => Some(Component::Tone(2, 'ˊ')),
        '3' => Some(Component::Tone(3, 'ˇ')),
        '4' => Some(Component::Tone(4, 'ˋ')),
        '7' => Some(Component::Tone(5, '˙')),

        _ => None,
    }
}
