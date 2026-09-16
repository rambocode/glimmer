//! 候选页指令的访问器：候选的标签 / 文字 / 译词、高亮、排布与翻页可用性。

use std::ffi::{c_char, c_int};

use super::entry::Candidate;
use super::{GlimmerOutputs, entry, text_pointer};
use crate::guard;

/// 第 `index` 条指令里第 `candidate` 个候选；任何一级为空或越界为 `None`。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
unsafe fn candidate<'a>(
    outputs: *const GlimmerOutputs,
    index: usize,
    candidate: usize,
) -> Option<&'a Candidate> {
    // SAFETY: 见函数的 Safety 约定
    unsafe { entry(outputs, index) }.and_then(|entry| entry.candidates.get(candidate))
}

/// 本页候选数；收起或其余种类为 0。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_candidate_count(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> usize {
    guard("glimmer_outputs_candidate_count", 0, || {
        // SAFETY: 见函数的 Safety 约定
        unsafe { entry(outputs, index) }.map_or(0, |entry| entry.candidates.len())
    })
}

/// 候选的数字标签（`1`…`9`，之后为空串）；越界为空指针。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_candidate_label(
    outputs: *const GlimmerOutputs,
    index: usize,
    position: usize,
) -> *const c_char {
    guard("glimmer_outputs_candidate_label", std::ptr::null(), || {
        // SAFETY: 见函数的 Safety 约定
        text_pointer(unsafe { candidate(outputs, index, position) }.map(|c| &c.label))
    })
}

/// 候选文字；越界为空指针。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_candidate_text(
    outputs: *const GlimmerOutputs,
    index: usize,
    position: usize,
) -> *const c_char {
    guard("glimmer_outputs_candidate_text", std::ptr::null(), || {
        // SAFETY: 见函数的 Safety 约定
        text_pointer(unsafe { candidate(outputs, index, position) }.map(|c| &c.text))
    })
}

/// 候选的译词注解；没有译词或越界为空指针。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_comment(
    outputs: *const GlimmerOutputs,
    index: usize,
    position: usize,
) -> *const c_char {
    guard("glimmer_outputs_comment", std::ptr::null(), || {
        // SAFETY: 见函数的 Safety 约定
        text_pointer(
            unsafe { candidate(outputs, index, position) }.and_then(|c| c.comment.as_ref()),
        )
    })
}

/// 高亮的候选下标（页内）；其余为 0。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_highlight(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> c_int {
    guard("glimmer_outputs_highlight", 0, || {
        // SAFETY: 见函数的 Safety 约定
        unsafe { entry(outputs, index) }.map_or(0, |entry| entry.highlight)
    })
}

/// 候选竖排（1）还是横排（0）。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_vertical(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> c_int {
    guard("glimmer_outputs_vertical", 0, || {
        // SAFETY: 见函数的 Safety 约定
        c_int::from(unsafe { entry(outputs, index) }.is_some_and(|entry| entry.vertical))
    })
}

/// 有没有上一页（1 / 0）。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_has_prev(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> c_int {
    guard("glimmer_outputs_has_prev", 0, || {
        // SAFETY: 见函数的 Safety 约定
        c_int::from(unsafe { entry(outputs, index) }.is_some_and(|entry| entry.has_prev))
    })
}

/// 有没有下一页（1 / 0）。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_has_next(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> c_int {
    guard("glimmer_outputs_has_next", 0, || {
        // SAFETY: 见函数的 Safety 约定
        c_int::from(unsafe { entry(outputs, index) }.is_some_and(|entry| entry.has_next))
    })
}
