//! 会话的事件函数：按键、焦点、面板操作（翻页 / 移动高亮 / 点选 / 切中英）、轮询，以及应用报来的前文、光标矩形、私密状态。
//! 每个事件都转给前端状态机的同名方法，返回要执行的指令。

use std::ffi::{CStr, c_char, c_int};

use glimmer_linux::frontend::MODE_PROPERTY;
use glimmer_linux::key::modifiers::{
    CONTROL_MASK, LOCK_MASK, MOD1_MASK, MOD4_MASK, RELEASE_MASK, SHIFT_MASK, SUPER_MASK,
};

use super::{GlimmerSession, run, with};
use crate::outputs::GlimmerOutputs;

/// Fcitx5 `KeyStates` 里与 IBus 修饰键掩码同位同义的那几位（`fcitx-utils/keysym.h`：Shift / CapsLock / Ctrl / Alt 是低四位，
/// Super（Mod4）是 1<<6，Super2 是 1<<26）。其余位（NumLock、Repeat、Virtual 等）Fcitx5 另有用途，丢掉免得误判。
const SHARED_STATE_BITS: u32 =
    SHIFT_MASK | LOCK_MASK | CONTROL_MASK | MOD1_MASK | MOD4_MASK | SUPER_MASK;

/// 把 C 字符串读成 `String`；空指针为 `None`。
///
/// # Safety
/// `text` 为空或指向以 NUL 结尾的字符串。
unsafe fn string(text: *const c_char) -> Option<String> {
    // SAFETY: 见函数的 Safety 约定
    (!text.is_null()).then(|| {
        unsafe { CStr::from_ptr(text) }
            .to_string_lossy()
            .into_owned()
    })
}

/// 按键：`keysym` 是 X keysym（Fcitx5 的 `KeySym` 同值），`states` 是 Fcitx5 的 `KeyStates`。
/// `handled` 非空时写入吃不吃（1 / 0）。会话为空时返回空指针、`handled` 写 0。
///
/// # Safety
/// `session` 为空或是有效会话；`handled` 为空或可写。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_key(
    session: *mut GlimmerSession,
    keysym: u32,
    states: u32,
    is_release: c_int,
    handled: *mut c_int,
) -> *mut GlimmerOutputs {
    let mut eaten = false;
    let state = (states & SHARED_STATE_BITS) | if is_release != 0 { RELEASE_MASK } else { 0 };
    // SAFETY: 见函数的 Safety 约定
    let outputs = unsafe {
        run("glimmer_session_key", session, |session| {
            let (consumed, outputs) = session.process_key(keysym, state);
            eaten = consumed;
            outputs
        })
    };
    // SAFETY: 见函数的 Safety 约定
    if let Some(handled) = unsafe { handled.as_mut() } {
        *handled = c_int::from(eaten && !outputs.is_null());
    }
    outputs
}

/// 获焦：`program` 是应用名（Fcitx5 `InputContext::program()`），空指针或空串算不知道。
///
/// # Safety
/// `session` 为空或是有效会话；`program` 为空或指向以 NUL 结尾的字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_focus_in(
    session: *mut GlimmerSession,
    program: *const c_char,
) -> *mut GlimmerOutputs {
    // SAFETY: 见函数的 Safety 约定
    let program = unsafe { string(program) }.filter(|program| !program.is_empty());
    // SAFETY: 见函数的 Safety 约定
    unsafe { run("glimmer_session_focus_in", session, |s| s.focus_in(program)) }
}

/// 失焦 / 切走输入法：清掉组句、关协议会话。
///
/// # Safety
/// `session` 为空或是有效会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_focus_out(
    session: *mut GlimmerSession,
) -> *mut GlimmerOutputs {
    // SAFETY: 见函数的 Safety 约定
    unsafe { run("glimmer_session_focus_out", session, |s| s.focus_out()) }
}

/// 应用要求重置：清掉组句，会话保留。
///
/// # Safety
/// `session` 为空或是有效会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_reset(
    session: *mut GlimmerSession,
) -> *mut GlimmerOutputs {
    // SAFETY: 见函数的 Safety 约定
    unsafe { run("glimmer_session_reset", session, |s| s.reset()) }
}

/// 组句中取一次异步结果（云联想 / 整句重排）；没变化时指令为空。
///
/// # Safety
/// `session` 为空或是有效会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_poll(session: *mut GlimmerSession) -> *mut GlimmerOutputs {
    // SAFETY: 见函数的 Safety 约定
    unsafe { run("glimmer_session_poll", session, |s| s.poll()) }
}

/// 面板翻页：`next` 非零下一页，否则上一页。
///
/// # Safety
/// `session` 为空或是有效会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_page(
    session: *mut GlimmerSession,
    next: c_int,
) -> *mut GlimmerOutputs {
    // SAFETY: 见函数的 Safety 约定
    unsafe { run("glimmer_session_page", session, |s| s.page(next != 0)) }
}

/// 面板移动高亮：`down` 非零下移，否则上移。
///
/// # Safety
/// `session` 为空或是有效会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_move_cursor(
    session: *mut GlimmerSession,
    down: c_int,
) -> *mut GlimmerOutputs {
    // SAFETY: 见函数的 Safety 约定
    unsafe {
        run("glimmer_session_move_cursor", session, |s| {
            s.move_cursor(down != 0)
        })
    }
}

/// 点选本页第 `index` 个候选（从 0 起）。
///
/// # Safety
/// `session` 为空或是有效会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_select(
    session: *mut GlimmerSession,
    index: u32,
) -> *mut GlimmerOutputs {
    // SAFETY: 见函数的 Safety 约定
    unsafe {
        run("glimmer_session_select", session, |s| {
            s.candidate_clicked(index)
        })
    }
}

/// 状态区点了中 / 英：切换模式（组着的拼音先原样上屏）。
///
/// # Safety
/// `session` 为空或是有效会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_toggle_mode(
    session: *mut GlimmerSession,
) -> *mut GlimmerOutputs {
    // SAFETY: 见函数的 Safety 约定
    unsafe {
        run("glimmer_session_toggle_mode", session, |s| {
            s.property_activate(MODE_PROPERTY)
        })
    }
}

/// 应用报来光标周围的文字，`cursor` 是光标处的字符（Unicode 标量）下标。
///
/// # Safety
/// `session` 为空或是有效会话；`text` 为空或指向以 NUL 结尾的字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_set_surrounding(
    session: *mut GlimmerSession,
    text: *const c_char,
    cursor: u32,
) {
    // SAFETY: 见函数的 Safety 约定
    let text = unsafe { string(text) }.unwrap_or_default();
    // SAFETY: 见函数的 Safety 约定
    unsafe {
        with("glimmer_session_set_surrounding", session, (), |s| {
            s.set_surrounding_text(&text, cursor)
        })
    };
}

/// 应用报来光标矩形（屏幕坐标）。
///
/// # Safety
/// `session` 为空或是有效会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_set_cursor_rect(
    session: *mut GlimmerSession,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
) {
    // SAFETY: 见函数的 Safety 约定
    unsafe {
        with("glimmer_session_set_cursor_rect", session, (), |s| {
            s.set_cursor_location(x, y, width, height)
        })
    };
}

/// 输入框是不是私密的（密码框 / 敏感输入）：私密时不学习、不发云端。
///
/// # Safety
/// `session` 为空或是有效会话。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_set_private(session: *mut GlimmerSession, private: c_int) {
    // SAFETY: 见函数的 Safety 约定
    unsafe {
        with("glimmer_session_set_private", session, (), |s| {
            s.set_private(private != 0)
        })
    };
}
