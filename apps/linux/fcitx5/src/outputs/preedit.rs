//! preedit 指令的访问器：光标与按样式分的段。

use std::ffi::{c_char, c_int};

use super::{GlimmerOutputs, entry, text_pointer};
use crate::guard;

/// preedit 光标（UTF-8 字节偏移）；其余种类、空指针或越界为 0。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_cursor(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> c_int {
    guard("glimmer_outputs_cursor", 0, || {
        // SAFETY: 见函数的 Safety 约定
        unsafe { entry(outputs, index) }.map_or(0, |entry| entry.cursor)
    })
}

/// preedit 的段数；收起或其余种类为 0。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_segment_count(
    outputs: *const GlimmerOutputs,
    index: usize,
) -> usize {
    guard("glimmer_outputs_segment_count", 0, || {
        // SAFETY: 见函数的 Safety 约定
        unsafe { entry(outputs, index) }.map_or(0, |entry| entry.segments.len())
    })
}

/// 第 `segment` 段的文字；越界为空指针。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_segment_text(
    outputs: *const GlimmerOutputs,
    index: usize,
    segment: usize,
) -> *const c_char {
    guard("glimmer_outputs_segment_text", std::ptr::null(), || {
        // SAFETY: 见函数的 Safety 约定
        let segment =
            unsafe { entry(outputs, index) }.and_then(|entry| entry.segments.get(segment));
        text_pointer(segment.map(|segment| &segment.text))
    })
}

/// 第 `segment` 段是否画淡（1 / 0）；越界为 0。
///
/// # Safety
/// `outputs` 为空或是事件函数返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_outputs_segment_dimmed(
    outputs: *const GlimmerOutputs,
    index: usize,
    segment: usize,
) -> c_int {
    guard("glimmer_outputs_segment_dimmed", 0, || {
        // SAFETY: 见函数的 Safety 约定
        let segment =
            unsafe { entry(outputs, index) }.and_then(|entry| entry.segments.get(segment));
        c_int::from(segment.is_some_and(|segment| segment.dimmed))
    })
}
