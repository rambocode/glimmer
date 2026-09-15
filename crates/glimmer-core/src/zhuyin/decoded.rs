//! 注音解码结果表示。
use super::layout::map_key;
use super::syllable::ZhuyinSyllable;
use crate::parser::Segmentation;
use crate::parser::Syllable as ParserSyllable;

/// 解碼的單一單元，對應一個注音音節或分隔符
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// 轉換為拼音後的字串
    pub pinyin: String,
    /// 顯示用的注音符號字串（包含聲調）
    pub display: String,
    /// 原始按鍵字串
    pub keys: String,
    /// 是否為完整音節（有聲調、或可以作為結尾）
    pub complete: bool,
}

/// 一段注音鍵解碼的結果：能解的單元 + 解不動的尾巴。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decoded {
    units: Vec<Unit>,
    tail: String,
    pinyin: String,
}

impl Decoded {
    pub fn new(units: Vec<Unit>, tail: String) -> Self {
        let mut pinyin = String::with_capacity(units.len() * 6);
        for unit in &units {
            if unit.pinyin == "'" {
                continue;
            }
            if !pinyin.is_empty() {
                pinyin.push('\'');
            }
            pinyin.push_str(&unit.pinyin);
        }
        Self {
            units,
            tail,
            pinyin,
        }
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    pub fn pinyin(&self) -> &str {
        &self.pinyin
    }

    pub fn tail(&self) -> &str {
        &self.tail
    }

    pub fn is_complete(&self) -> bool {
        self.tail.is_empty() && self.units.iter().all(|u| u.complete || u.pinyin == "'")
    }

    pub fn segmentation(&self) -> Option<Segmentation> {
        let syllables: Vec<ParserSyllable> = self
            .units
            .iter()
            .filter(|u| u.pinyin != "'")
            .map(|u| {
                if u.complete {
                    ParserSyllable::complete(&u.pinyin)
                } else {
                    ParserSyllable::partial(&u.pinyin)
                }
            })
            .collect();
        (!syllables.is_empty()).then_some(Segmentation { syllables })
    }

    pub fn marked(&self) -> String {
        let mut s = String::new();
        for unit in &self.units {
            s.push_str(&unit.display);
        }
        s.push_str(&self.tail);
        s
    }

    pub fn keys_for(&self, pinyin_len: usize) -> usize {
        let mut keys = 0;
        let mut position = 0;
        let mut first = true;
        let mut pending_separators = 0;
        for unit in &self.units {
            if unit.pinyin == "'" {
                pending_separators += unit.keys.len();
                continue;
            }
            let start = if first { 0 } else { position + 1 };
            let end = start + unit.pinyin.len();
            if pinyin_len < end {
                break;
            }
            keys += pending_separators + unit.keys.len();
            pending_separators = 0;
            position = end;
            first = false;
        }
        if keys > 0 {
            keys += pending_separators;
        }
        keys
    }
}

pub fn decode(input: &str) -> Decoded {
    let mut units = Vec::new();
    let mut current = ZhuyinSyllable::new();
    let mut tail = String::new();

    for c in input.chars() {
        if c == '\'' {
            if !current.is_empty() {
                units.push(Unit {
                    pinyin: current.to_pinyin(),
                    display: current.display_string(),
                    keys: current.keys.clone(),
                    complete: current.has_tone(),
                });
                current = ZhuyinSyllable::new();
            }
            units.push(Unit {
                pinyin: "'".to_string(),
                display: "'".to_string(),
                keys: "'".to_string(),
                complete: true,
            });
            continue;
        }

        if let Some(comp) = map_key(c) {
            if !current.push(comp, c) {
                // 不相容，將 current 推入 units，並開啟新音節
                if !current.is_empty() {
                    units.push(Unit {
                        pinyin: current.to_pinyin(),
                        display: current.display_string(),
                        keys: current.keys.clone(),
                        complete: current.has_tone(),
                    });
                }
                current = ZhuyinSyllable::new();
                if !current.push(comp, c) {
                    tail.push(c);
                }
            }
        } else {
            // 無法解析的字元，直接放到尾巴或當前就中斷
            if !current.is_empty() {
                units.push(Unit {
                    pinyin: current.to_pinyin(),
                    display: current.display_string(),
                    keys: current.keys.clone(),
                    complete: current.has_tone(),
                });
                current = ZhuyinSyllable::new();
            }
            tail.push(c);
        }
    }

    if !current.is_empty() {
        units.push(Unit {
            pinyin: current.to_pinyin(),
            display: current.display_string(),
            keys: current.keys.clone(),
            complete: current.has_tone(),
        });
    }

    Decoded::new(units, tail)
}
