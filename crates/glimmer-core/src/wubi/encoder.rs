//! 自动造词的编码规则（librime `encoder.rules` 与 fcitx5 `[Rule]` 一致）：
//! 二字 `AaAbBaBb`、三字 `AaBaCaCb`、四字及以上 `AaBaCaZa`。
//! 大写是第几个字（Z 为末字），小写是取该字全码的第几位；某个字全码不够长就造不出。

use super::Reverse;

/// 按字数选规则，把词的每个字换成全码里的那一位；少于两个字、有字查不到全码或全码不够长返回 `None`。
pub fn encode(chars: &[char], reverse: &Reverse) -> Option<String> {
    let last = chars.len().checked_sub(1)?;
    // (字的下标, 全码第几位)，下标从 0 起
    let rule: &[(usize, usize)] = match chars.len() {
        0 | 1 => return None,
        2 => &[(0, 0), (0, 1), (1, 0), (1, 1)],
        3 => &[(0, 0), (1, 0), (2, 0), (2, 1)],
        _ => &[(0, 0), (1, 0), (2, 0), (last, 0)],
    };
    let mut code = String::with_capacity(rule.len());
    for &(index, position) in rule {
        let full = reverse.code(chars[index])?;
        code.push(full.as_bytes().get(position).copied()? as char);
    }
    Some(code)
}
