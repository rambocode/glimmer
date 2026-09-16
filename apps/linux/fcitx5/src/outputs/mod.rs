//! 事件函数的返回值：一串按顺序执行的指令（[`GlimmerOutputs`]），以及 C 侧读它的访问器。
//! 通用字段（种类、文字、中英）在这里，preedit 分段在 `preedit`，候选页在`panel`。

mod entry;
mod panel;
mod preedit;

use std::ffi::{c_char, c_int};

use glimmer_linux::frontend::Output;

pub use entry::Entry;
pub use panel::{
    glimmer_outputs_candidate_count, glimmer_outputs_candidate_label,
    glimmer_outputs_candidate_text, glimmer_outputs_comment, glimmer_outputs_has_next,
    glimmer_outputs_has_prev, glimmer_outputs_highlight, glimmer_outputs_vertical,
};
pub use preedit::{
    glimmer_outputs_cursor, glimmer_outputs_segment_count, glimmer_outputs_segment_dimmed,
    glimmer_outputs_segment_text,
};

use crate::guard;

/// 上屏：`text` 是要上屏的文字。
pub const GLIMMER_OUTPUT_COMMIT: c_int = 0;

/// 更新 preedit：`text` 为空指针表示收起；否则看 `cursor` 与分段。
pub const GLIMMER_OUTPUT_PREEDIT: c_int = 1;

/// 更新候选页：`candidate_count` 为 0 表示收起。
pub const GLIMMER_OUTPUT_CANDIDATES: c_int = 2;

/// 更新辅助文字（候选上方一行）：`text` 为空指针表示收起。
pub const GLIMMER_OUTPUT_AUXILIARY: c_int = 3;

/// 中英模式（含初次登记）：看 `english`。
pub const GLIMMER_OUTPUT_MODE: c_int = 4;

/// 一次事件产出的全部指令，C 侧按下标顺序执行，用完 `glimmer_outputs_free`。
#[derive(Debug, Default)]
pub struct GlimmerOutputs {
    /// 指令，按顺序。
    entries: Vec<Entry>,
}

impl GlimmerOutputs {
    /// 把前端会话的指令换成 C 形状，装箱交出所有权。
    pub fn boxed(outputs: Vec<Output>) -> *mut GlimmerOutputs {
        Box::into_raw(Box::new(GlimmerOutputs {
            entries: outputs.into_iter().map(Entry::from).collect(),
        }))
    }
}

/// 取第 `index` 条指令；空指针或越界为 `None`。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
pub(crate) unsafe fn entry<'a>(outputs: *const GlimmerOutputs, index: usize) -> Option<&'a Entry> {
    // SAFETY: 调用方保证非空时指针有效，且在返回的引用用完之前不释放
    unsafe { outputs.as_ref() }?.entries.get(index)
}

/// 字符串字段转成 C 指针；没有为空指针。
pub(crate) fn text_pointer(text: Option<&std::ffi::CString>) -> *const c_char {
    text.map_or(std::ptr::null(), |text| text.as_ptr())
}

/// 指令条数；空指针为 0。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_len(outputs: *const GlimmerOutputs) -> usize {
    guard("glimmer_outputs_len", 0, || {
        // SAFETY: 见函数的 Safety 约定
        unsafe { outputs.as_ref() }.map_or(0, |outputs| outputs.entries.len())
    })
}

/// 第 `index` 条的种类（`GLIMMER_OUTPUT_*`）；空指针或越界为 -1。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_kind(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> c_int {
    guard("glimmer_outputs_kind", -1, || {
        // SAFETY: 见函数的 Safety 约定
        unsafe { entry(outputs, index) }.map_or(-1, |entry| entry.kind)
    })
}

/// 第 `index` 条的文字（UTF-8，活到 `glimmer_outputs_free`）；收起、无此字段、空指针或越界为空指针。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_text(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> *const c_char {
    guard("glimmer_outputs_text", std::ptr::null(), || {
        // SAFETY: 见函数的 Safety 约定
        text_pointer(unsafe { entry(outputs, index) }.and_then(|entry| entry.text.as_ref()))
    })
}

/// 中英模式指令里现在是不是英文（1 / 0）；其余为 0。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_english(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> c_int {
    guard("glimmer_outputs_english", 0, || {
        // SAFETY: 见函数的 Safety 约定
        c_int::from(unsafe { entry(outputs, index) }.is_some_and(|entry| entry.english))
    })
}

/// 释放一次事件的指令；空指针无事发生。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针；释放后不能再用。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_free(outputs: *mut GlimmerOutputs) {
    if outputs.is_null() {
        return;
    }
    guard("glimmer_outputs_free", (), || {
        // SAFETY: 指针来自 `GlimmerOutputs::boxed` 的 `Box::into_raw`，调用方保证只释放一次
        drop(unsafe { Box::from_raw(outputs) });
    });
}
