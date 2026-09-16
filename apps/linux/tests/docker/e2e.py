#!/usr/bin/env python3
"""微明 IBus 引擎的端到端测试：用 libibus 的 Python 绑定扮演应用，经真的 ibus-daemon 驱动 glimmer-ibus（回显后端）。

断言：敲 a b 空格收到上屏 AB、中途收到 preedit 与带译文的候选表；单击 Shift 后中英属性变成「英」且字母放行；
点属性切回中文；重置与失焦时屏上的 preedit 落进应用、引擎的组句随之清空。
"""

import sys
import time

import gi

gi.require_version("IBus", "1.0")
from gi.repository import GLib, IBus  # noqa: E402

ENGINE = "glimmer"
TIMEOUT = 10.0


class Recorder:
    """收集输入上下文发来的信号。"""

    def __init__(self, context):
        self.commits = []
        self.preedits = []
        self.tables = []
        self.properties = {}
        context.connect("commit-text", self.on_commit)
        context.connect("update-preedit-text", self.on_preedit)
        context.connect("update-lookup-table", self.on_table)
        context.connect("register-properties", self.on_register)
        context.connect("update-property", self.on_property)

    def on_commit(self, _context, text):
        self.commits.append(text.get_text())

    def on_preedit(self, _context, text, cursor, visible):
        self.preedits.append((text.get_text(), cursor, visible))

    def on_table(self, _context, table, visible):
        rows = []
        for index in range(table.get_number_of_candidates()):
            label = table.get_label(index)
            rows.append((label.get_text() if label else None, table.get_candidate(index).get_text()))
        self.tables.append((rows, visible))

    def on_register(self, _context, props):
        index = 0
        while True:
            prop = props.get(index)
            if prop is None:
                break
            self.on_property(None, prop)
            index += 1

    def on_property(self, _context, prop):
        self.properties[prop.get_key()] = (prop.get_label().get_text(), int(prop.get_state()))


def pump(seconds=0.3):
    """跑一会儿主循环，让信号送达。"""
    context = GLib.MainContext.default()
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        while context.iteration(False):
            pass
        time.sleep(0.01)


def wait_for(predicate, what):
    """等条件成立，超时就失败。"""
    context = GLib.MainContext.default()
    deadline = time.monotonic() + TIMEOUT
    while time.monotonic() < deadline:
        while context.iteration(False):
            pass
        if predicate():
            return
        time.sleep(0.02)
    fail(f"等待超时：{what}")


def fail(message):
    print(f"FAIL: {message}", flush=True)
    sys.exit(1)


def check(condition, message):
    if not condition:
        fail(message)
    print(f"ok   {message}", flush=True)


def tap(context, keyval, state=0):
    """按下再松开；返回按下时引擎吃不吃。"""
    handled = context.process_key_event(keyval, 0, state)
    context.process_key_event(keyval, 0, state | IBus.ModifierType.RELEASE_MASK)
    pump(0.1)
    return handled


def main():
    IBus.init()
    bus = IBus.Bus()
    wait_for(bus.is_connected, "连上 ibus-daemon")
    names = [engine.get_name() for engine in bus.list_engines()]
    check(ENGINE in names, f"daemon 登记了引擎 {ENGINE}（{names}）")

    context = bus.create_input_context("glimmer-e2e")
    capabilities = (
        IBus.Capabilite.PREEDIT_TEXT
        | IBus.Capabilite.AUXILIARY_TEXT
        | IBus.Capabilite.LOOKUP_TABLE
        | IBus.Capabilite.FOCUS
        | IBus.Capabilite.PROPERTY
    )
    context.set_capabilities(capabilities)
    recorder = Recorder(context)
    context.focus_in()
    context.set_engine(ENGINE)
    wait_for(lambda: context.get_engine() is not None and context.get_engine().get_name() == ENGINE, "切到 glimmer 引擎")
    wait_for(lambda: "InputMode" in recorder.properties, "引擎登记中英属性")
    check(recorder.properties["InputMode"] == ("中", 0), f"初始是中文模式 {recorder.properties['InputMode']}")

    check(tap(context, IBus.KEY_a), "a 被吃掉")
    wait_for(lambda: any(text == "a" and visible for text, _, visible in recorder.preedits), "preedit 显示 a")
    check(tap(context, IBus.KEY_b), "b 被吃掉")
    wait_for(lambda: ("ab", 2, True) in recorder.preedits, "preedit 显示 ab，光标在末尾")
    wait_for(lambda: any(rows == [("1", "AB echo")] and visible for rows, visible in recorder.tables), "候选表是「1 AB echo」")
    check(tap(context, IBus.KEY_space), "空格被吃掉")
    wait_for(lambda: "AB" in recorder.commits, "上屏 AB")
    check(recorder.preedits[-1][2] is False, f"上屏后 preedit 收起 {recorder.preedits[-1]}")

    check(not tap(context, IBus.KEY_Return), "空闲时回车放行")

    context.process_key_event(IBus.KEY_Shift_L, 0, 0)
    context.process_key_event(IBus.KEY_Shift_L, 0, IBus.ModifierType.SHIFT_MASK | IBus.ModifierType.RELEASE_MASK)
    wait_for(lambda: recorder.properties["InputMode"] == ("英", 1), "单击 Shift 后属性变成「英」")
    check(not tap(context, IBus.KEY_a), "英文模式下字母放行")

    context.property_activate("InputMode", 0)
    wait_for(lambda: recorder.properties["InputMode"] == ("中", 0), "点属性切回「中」")

    context.process_key_event(IBus.KEY_Shift_L, 0, 0)
    tap(context, IBus.KEY_A, IBus.ModifierType.SHIFT_MASK)
    context.process_key_event(IBus.KEY_Shift_L, 0, IBus.ModifierType.SHIFT_MASK | IBus.ModifierType.RELEASE_MASK)
    pump()
    check(recorder.properties["InputMode"] == ("中", 0), "Shift+A 不切模式")

    tap(context, IBus.KEY_q)
    context.reset()
    wait_for(lambda: "q" in recorder.commits, "重置时 preedit q 由 daemon 落定")
    check(not tap(context, IBus.KEY_BackSpace), "重置后引擎不在组句，退格放行")

    tap(context, IBus.KEY_x)
    tap(context, IBus.KEY_y)
    context.focus_out()
    wait_for(lambda: "xy" in recorder.commits, "失焦时 preedit xy 由 daemon 落定")

    print("PASS", flush=True)


if __name__ == "__main__":
    main()
