//! 五笔按键之后的自动上屏：四码全码命中、顶字、满四码空码丢弃。

use crate::engine::Engine;
use crate::wubi::{MAX_CODE_LEN, REVERSE_LOOKUP_KEY, is_code_key};

impl Engine {
    /// 五笔按键之后要不要自动上屏（`c` 是刚敲的键）：
    /// - 作用域正好四码且首选是全码命中、`auto_select` 开着 → 首选待上屏；
    /// - 上一段已满四码却还留着（空码，或关了自动上屏）：有首选就顶字上屏，没有（空码）就整段丢掉、缓冲区只剩新键；
    /// - 新键接上后既无全码也无前缀命中、而旧段有首选 → 顶字：旧段首选待上屏，新键先拿掉、旧段上屏后再补回；
    /// - 其余（空码还没满四码、正常有命中）什么都不做。光标不在末尾、直输段、`z` 反查都不管。
    ///
    /// 整句输入（`[wubi] sentence`）开着时全都不做：缓冲区要能越过四码接着长，上屏由空格定。
    pub(in crate::engine) fn check_wubi_auto_commit(&mut self, c: char) {
        let Some(scheme) = &self.wubi else {
            return;
        };
        // 英文模式（Caps Lock）下缓冲区里是英文字母，`query_inner` 也不走五笔，这里同样不介入
        if self.english_mode {
            return;
        }
        if scheme.options().sentence {
            return;
        }
        let auto_select = scheme.options().auto_select;
        if self.composition.cursor() != self.composition.text().len() {
            return;
        }
        let scope = self.composition.text().to_owned();
        if !scope.chars().all(is_code_key) || scope.starts_with(REVERSE_LOOKUP_KEY) {
            return;
        }
        let old = &scope[..scope.len() - c.len_utf8()];
        if old.len() >= MAX_CODE_LEN {
            let first = self.wubi_candidates(old).into_iter().next();
            self.composition.backspace();
            match first {
                Some(candidate) => {
                    self.pending_auto_commit = Some(candidate);
                    self.deferred_key = Some(c);
                }
                None => {
                    self.composition.clear();
                    self.composition.push(c);
                    self.chain.leave_buffer();
                }
            }
            return;
        }
        let items = self.wubi_candidates(&scope);
        if items.is_empty() {
            if let Some(first) = self.wubi_candidates(old).into_iter().next() {
                self.composition.backspace();
                self.pending_auto_commit = Some(first);
                self.deferred_key = Some(c);
            }
            return;
        }
        if scope.len() == MAX_CODE_LEN
            && auto_select
            && items[0]
                .syllables
                .first()
                .is_some_and(|code| *code == scope)
        {
            self.pending_auto_commit = items.into_iter().next();
        }
    }
}
