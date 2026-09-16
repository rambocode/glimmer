//! 上屏：把候选（或原样拼音）交给 Engine、文本插进应用、再按剩余缓冲区刷新。
//! 手动选词、回车原样上屏与五笔的自动上屏都走这里，学习 / 日志 / 统计才一致。

use glimmer_core::Candidate;

use super::GlimmerInputController;
use crate::host;
use crate::imk::TextClient;

impl GlimmerInputController {
    /// 上屏当前高亮的候选（空格 / 回车路径）。
    pub(super) fn commit_highlighted(&self, client: TextClient<'_>) -> bool {
        let index = host::with(|h| h.session.highlighted).unwrap_or(0);
        self.commit_index(index, client)
    }

    /// 上屏第 `index` 个候选；没有候选时上屏拼音本身。上屏后剩余拼音继续组句。
    pub(super) fn commit_index(&self, index: usize, client: TextClient<'_>) -> bool {
        let candidate = host::with(|h| h.session.candidate(index)).flatten();
        let Some(candidate) = candidate else {
            if host::with(|h| index < h.session.layout.len()).unwrap_or(false) {
                return true;
            }
            return self.commit_raw(client);
        };
        self.commit_candidate(&candidate, client)
    }

    /// 把一条候选交给 Engine 上屏（学习、日志、统计都在 Engine 里记），文本插进应用，再按剩余缓冲区刷新。
    /// 手动选词与五笔的自动上屏都走这里，两条路的学习行为才一致。
    pub(super) fn commit_candidate(&self, candidate: &Candidate, client: TextClient<'_>) -> bool {
        let Some(text) = host::with(|h| h.engine.commit(candidate)) else {
            return false;
        };
        tracing::debug!(%text, "commit");
        client.insert_text(&text);
        self.refresh(client);
        true
    }

    /// 中文组句里敲进一个键再刷新。五笔下 Core 在 push 之后可能要求自动上屏（敲满四码命中全码、顶字），
    /// 取到就先按候选上屏路径把它打出去；顶字时新键 Core 已补回缓冲区，上屏后的 refresh 会把它查出来。
    pub(super) fn push_and_refresh(&self, c: char, client: TextClient<'_>) {
        let auto = host::with(|h| {
            h.engine.push(c);
            h.engine.take_auto_commit()
        })
        .flatten();
        match auto {
            Some(candidate) => {
                tracing::debug!(text = %candidate.text, "五笔自动上屏");
                self.commit_candidate(&candidate, client);
            }
            None => self.refresh(client),
        }
    }

    /// 把拼音原样上屏并清空。缓冲区为空时返回 false。
    pub(super) fn commit_raw(&self, client: TextClient<'_>) -> bool {
        let Some(raw) = host::with(|h| h.engine.take_raw()) else {
            return false;
        };
        if raw.is_empty() {
            return false;
        }
        tracing::debug!(%raw, "commit raw");
        client.insert_text(&raw);
        self.refresh(client);
        true
    }
}
