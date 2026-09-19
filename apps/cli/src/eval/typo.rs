//! 注错：给评测句的全拼故意敲错一键，量敲错纠正救不救得回来。
//!
//! 干净的句子集只量得出「误纠」（没敲错却被改了），量不出「召回」；真实日志里纠错生效的上屏只有几十条，不够调常数。
//! 这里按句子文本的哈希定位置与错法，同一份句子集每次注出来的错一样，两次评测才能比。

use glimmer_core::correction::typo::adjacent;

/// 给 `pinyin` 注一处敲错：相邻两键敲反，或一键敲到旁边的键（各半）。`text` 只用来定随机数种子。
/// 太短（不到 4 个字母）或凑不出与原串不同的结果时返回 `None`。
pub fn inject(text: &str, pinyin: &str) -> Option<String> {
    let letters = pinyin.as_bytes();
    if letters.len() < 4 || !letters.iter().all(u8::is_ascii_lowercase) {
        return None;
    }
    let seed = fnv1a(text.as_bytes());
    let index = (seed % letters.len() as u64) as usize;
    let transpose = (seed >> 32) & 1 == 0;
    // 先试抽到的错法，不行（两键相同、到了末尾）再试另一种
    let attempts = if transpose {
        [swap(letters, index), neighbor(letters, index, seed)]
    } else {
        [neighbor(letters, index, seed), swap(letters, index)]
    };
    attempts.into_iter().flatten().next()
}

/// 第 `index` 与 `index + 1` 键换位；两键相同或越界为 `None`。
fn swap(letters: &[u8], index: usize) -> Option<String> {
    let next = *letters.get(index + 1)?;
    if next == letters[index] {
        return None;
    }
    let mut typed = letters.to_vec();
    typed.swap(index, index + 1);
    String::from_utf8(typed).ok()
}

/// 第 `index` 键换成键盘上挨着它的某个字母。
fn neighbor(letters: &[u8], index: usize, seed: u64) -> Option<String> {
    let around: Vec<u8> = (b'a'..=b'z')
        .filter(|&letter| adjacent(letters[index] as char, letter as char))
        .collect();
    let replacement = *around.get((seed >> 16) as usize % around.len().max(1))?;
    let mut typed = letters.to_vec();
    typed[index] = replacement;
    String::from_utf8(typed).ok()
}

/// FNV-1a：只要稳定，不要密码学强度。
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_one_stable_typo() {
        let typed = inject("我想去", "woxiangqu").unwrap();
        assert_ne!(typed, "woxiangqu");
        assert_eq!(typed.len(), "woxiangqu".len());
        let different = typed
            .bytes()
            .zip("woxiangqu".bytes())
            .filter(|(a, b)| a != b)
            .count();
        assert!((1..=2).contains(&different));
        // 同一句每次注出来一样
        assert_eq!(inject("我想去", "woxiangqu").unwrap(), typed);
        assert_eq!(inject("好", "hao"), None);
    }
}
