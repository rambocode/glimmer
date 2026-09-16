#!/usr/bin/env python3
"""装好 deb 之后的端到端测试：经真的 ibus-daemon 驱动装机的 glimmer-ibus（真 Router + 随包词库）。

断言：敲 nihao 时 preedit 是拼音、候选表里有「你好」，空格上屏「你好」；数字键选第 N 个候选；
单击 Shift 切英文后回车原样上屏；Esc 清空组句不上屏。
"""

import sys

from e2e import ENGINE, Recorder, check, pump, tap, wait_for

import gi

gi.require_version("IBus", "1.0")
from gi.repository import IBus  # noqa: E402


def type_letters(context, letters):
    """逐个敲小写字母，每个都应被吃掉。"""
    for letter in letters:
        check(tap(context, IBus.keyval_from_name(letter)), f"{letter} 被吃掉")


def main():
    IBus.init()
    bus = IBus.Bus()
    wait_for(bus.is_connected, "连上 ibus-daemon")
    names = [engine.get_name() for engine in bus.list_engines()]
    check(ENGINE in names, f"daemon 从系统组件目录登记了引擎 {ENGINE}")

    context = bus.create_input_context("glimmer-e2e-router")
    context.set_capabilities(
        IBus.Capabilite.PREEDIT_TEXT
        | IBus.Capabilite.AUXILIARY_TEXT
        | IBus.Capabilite.LOOKUP_TABLE
        | IBus.Capabilite.FOCUS
        | IBus.Capabilite.PROPERTY
    )
    recorder = Recorder(context)
    context.focus_in()
    context.set_engine(ENGINE)
    # 真 Router 要装词库与语言模型，首次拉起比回显后端慢
    wait_for(lambda: "InputMode" in recorder.properties, "引擎起来并登记中英属性")

    type_letters(context, "nihao")
    wait_for(
        lambda: any("你好" in text for rows, visible in recorder.tables if visible for _, text in rows),
        "候选表里有「你好」",
    )
    wait_for(lambda: any(visible and text.replace("'", "").replace(" ", "") == "nihao" for text, _, visible in recorder.preedits), "preedit 是拼音 nihao")
    print(f"     候选表：{recorder.tables[-1][0][:5]}", flush=True)
    check(tap(context, IBus.KEY_space), "空格被吃掉")
    wait_for(lambda: "你好" in recorder.commits, "空格上屏「你好」")

    type_letters(context, "shijie")
    wait_for(lambda: recorder.tables and recorder.tables[-1][1], "shijie 有候选表")
    rows = recorder.tables[-1][0]
    second = rows[1][1].split(" ")[0] if len(rows) > 1 else None
    check(second is not None, f"候选至少两条 {rows[:3]}")
    check(tap(context, IBus.KEY_2), "数字 2 被吃掉")
    wait_for(lambda: recorder.commits and recorder.commits[-1] == second, f"数字 2 上屏第二个候选「{second}」")

    type_letters(context, "abc")
    commits_before = len(recorder.commits)
    check(tap(context, IBus.KEY_Escape), "Esc 被吃掉")
    pump(0.5)
    check(len(recorder.commits) == commits_before, "Esc 清空组句不上屏")
    check(not tap(context, IBus.KEY_BackSpace), "清空后退格放行")

    context.process_key_event(IBus.KEY_Shift_L, 0, 0)
    context.process_key_event(IBus.KEY_Shift_L, 0, IBus.ModifierType.SHIFT_MASK | IBus.ModifierType.RELEASE_MASK)
    wait_for(lambda: recorder.properties["InputMode"] == ("英", 1), "单击 Shift 切到英文")
    # 英文模式下 Router 给英文补全候选（与 Windows 一致），字母进组句；回车原样上屏
    type_letters(context, "ok")
    check(tap(context, IBus.KEY_Return), "回车被吃掉")
    wait_for(lambda: recorder.commits and recorder.commits[-1].strip() == "ok", "英文模式回车上屏 ok")

    print("PASS", flush=True)


if __name__ == "__main__":
    sys.exit(main())
