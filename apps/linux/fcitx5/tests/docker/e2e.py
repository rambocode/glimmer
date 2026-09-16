#!/usr/bin/env python3
"""微明 Fcitx5 插件的端到端测试：起真的 fcitx5（只开 dbus 前端与本插件），用 Gio D-Bus 扮演应用驱动真 Router。

要在一条会话总线里跑（`dbus-run-session -- python3 e2e.py`）。两种模式：
- 开发模式：设 `GLIMMER_FCITX5_STAGE=<cmake --install 的 DESTDIR>`，插件与配置从暂存目录加载；
  资源根由 `GLIMMER_DATA_ROOT` 传给插件（仓库根时用 `assets/sample/` 的样例词库）。
- 装机模式：不设 `GLIMMER_FCITX5_STAGE`，用系统装的插件与 `/usr/lib/glimmer` 的产品词库（装 deb 后跑）。

断言：敲 n i h a o 被吃掉、preedit 是拼音、候选含「你好」、空格上屏「你好」；数字键选词；Esc 清空不上屏；
单击 Shift 切英文后字母进英文组句、回车上屏，再单击切回中文。
"""

import glob
import os
import subprocess
import sys
import tempfile
import time

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

TIMEOUT = 20.0
FCITX_SERVICE = "org.fcitx.Fcitx5"
IM_PATH = "/org/freedesktop/portal/inputmethod"
IM_INTERFACE = "org.fcitx.Fcitx.InputMethod1"
IC_INTERFACE = "org.fcitx.Fcitx.InputContext1"
CONTROLLER_INTERFACE = "org.fcitx.Fcitx.Controller1"

# fcitx::CapabilityFlag
CAP_PREEDIT = 1 << 1
CAP_FORMATTED_PREEDIT = 1 << 4
CAP_SURROUNDING_TEXT = 1 << 6
CAP_CLIENT_SIDE_INPUT_PANEL = 1 << 39

# fcitx::KeyState / X keysym
SHIFT_STATE = 1 << 0
KEY_SHIFT_L = 0xFFE1
KEY_SPACE = 0x20
KEY_ESCAPE = 0xFF1B
KEY_RETURN = 0xFF0D

PROFILE = """[Groups/0]
Name=Default
Default Layout=us
DefaultIM=glimmer

[Groups/0/Items/0]
Name=glimmer
Layout=

[GroupOrder]
0=Default
"""


def fail(message):
    print(f"FAIL: {message}", flush=True)
    raise SystemExit(1)


def check(condition, message):
    if not condition:
        fail(message)
    print(f"ok   {message}", flush=True)


def pump(seconds=0.2):
    """跑一会儿主循环，让信号送达。"""
    context = GLib.MainContext.default()
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        while context.iteration(False):
            pass
        time.sleep(0.01)


def wait_for(predicate, what, timeout=TIMEOUT):
    """等条件成立，超时就失败。"""
    context = GLib.MainContext.default()
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        while context.iteration(False):
            pass
        if predicate():
            return
        time.sleep(0.02)
    fail(f"等待超时：{what}")


class Recorder:
    """收集输入上下文的信号：上屏、preedit、客户端面板（候选）。"""

    def __init__(self, bus, path):
        self.commits = []
        self.preedits = []
        self.panels = []
        bus.signal_subscribe(None, IC_INTERFACE, None, path, None, Gio.DBusSignalFlags.NONE, self.on_signal)

    def on_signal(self, _bus, _sender, _path, _interface, name, parameters):
        values = parameters.unpack()
        if name == "CommitString":
            self.commits.append(values[0])
        elif name == "UpdateFormattedPreedit":
            self.preedits.append("".join(text for text, _ in values[0]))
        elif name == "UpdateClientSideUI":
            preedit, _cursor, _aux_up, _aux_down, candidates, index, _layout, has_prev, has_next = values
            self.panels.append(
                {
                    "preedit": "".join(text for text, _ in preedit),
                    "candidates": [text for _, text in candidates],
                    "labels": [label for label, _ in candidates],
                    "index": index,
                }
            )

    def last_candidates(self):
        return self.panels[-1]["candidates"] if self.panels else []


def system_addon_dir():
    """系统 fcitx5 的插件目录（多架构路径）。"""
    for path in glob.glob("/usr/lib/*/fcitx5") + ["/usr/lib/fcitx5"]:
        if os.path.exists(os.path.join(path, "libdbusfrontend.so")):
            return path
    fail("找不到系统 fcitx5 插件目录（libdbusfrontend.so）")


def start_fcitx5(home):
    """写好 profile、按模式配环境变量，起 fcitx5；返回（进程，日志路径）。"""
    config_dir = os.path.join(home, ".config", "fcitx5")
    os.makedirs(config_dir, exist_ok=True)
    with open(os.path.join(config_dir, "profile"), "w", encoding="utf-8") as profile:
        profile.write(PROFILE)
    env = dict(os.environ)
    env.update(
        {
            "HOME": home,
            "XDG_CONFIG_HOME": os.path.join(home, ".config"),
            "XDG_DATA_HOME": os.path.join(home, ".local", "share"),
            "XDG_CACHE_HOME": os.path.join(home, ".cache"),
        }
    )
    for name in ("DISPLAY", "WAYLAND_DISPLAY"):
        env.pop(name, None)
    stage = os.environ.get("GLIMMER_FCITX5_STAGE")
    system_addons = system_addon_dir()
    if stage:
        staged = glob.glob(os.path.join(stage, "usr/lib/*/fcitx5/glimmer.so")) + glob.glob(
            os.path.join(stage, "usr/lib/fcitx5/glimmer.so")
        )
        check(len(staged) == 1, f"暂存目录里有 glimmer.so（{staged}）")
        env["FCITX_ADDON_DIRS"] = f"{os.path.dirname(staged[0])}:{system_addons}"
        env["FCITX_DATA_DIRS"] = f"{os.path.join(stage, 'usr/share/fcitx5')}:/usr/share/fcitx5"
        print(f"开发模式：插件 {staged[0]}，资源根 {env.get('GLIMMER_DATA_ROOT', '（编译期缺省）')}", flush=True)
    else:
        env.pop("GLIMMER_DATA_ROOT", None)
        env.pop("GLIMMER_ECHO", None)
        check(os.path.exists(os.path.join(system_addons, "glimmer.so")), f"系统插件目录里有 glimmer.so（{system_addons}）")
        print("装机模式：系统插件与 /usr/lib/glimmer", flush=True)
    log_path = os.path.join(home, "fcitx5.log")
    log = open(log_path, "w", encoding="utf-8")
    process = subprocess.Popen(
        ["fcitx5", "--disable=all", "--enable=keyboard,dbus,dbusfrontend,glimmer"],
        env=env,
        stdout=log,
        stderr=subprocess.STDOUT,
    )
    return process, log_path


def name_has_owner(bus, name):
    reply = bus.call_sync(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameHasOwner",
        GLib.Variant("(s)", (name,)),
        GLib.VariantType("(b)"),
        Gio.DBusCallFlags.NONE,
        -1,
        None,
    )
    return reply.unpack()[0]


def call(bus, path, interface, method, parameters=None, reply_type=None):
    reply = bus.call_sync(
        FCITX_SERVICE,
        path,
        interface,
        method,
        parameters,
        GLib.VariantType(reply_type) if reply_type else None,
        Gio.DBusCallFlags.NONE,
        10000,
        None,
    )
    return reply.unpack() if reply is not None else None


class Context:
    """一个 fcitx5 输入上下文的客户端。"""

    def __init__(self, bus, program):
        self.bus = bus
        path, _uuid = call(
            bus,
            IM_PATH,
            IM_INTERFACE,
            "CreateInputContext",
            GLib.Variant("(a(ss))", ([("program", program)],)),
            "(oay)",
        )
        self.path = path
        self.recorder = Recorder(bus, path)

    def method(self, name, parameters=None, reply_type=None):
        return call(self.bus, self.path, IC_INTERFACE, name, parameters, reply_type)

    def key(self, keyval, state=0, release=False):
        (handled,) = self.method(
            "ProcessKeyEvent", GLib.Variant("(uuubu)", (keyval, 0, state, release, 0)), "(b)"
        )
        return handled

    def tap(self, keyval, state=0):
        """按下再松开；返回按下时吃不吃。"""
        handled = self.key(keyval, state)
        self.key(keyval, state, release=True)
        pump(0.05)
        return handled

    def type_letters(self, letters):
        for letter in letters:
            check(self.tap(ord(letter)), f"{letter} 被吃掉")


def run(bus):
    wait_for(lambda: name_has_owner(bus, FCITX_SERVICE), f"fcitx5 占到总线名 {FCITX_SERVICE}")
    context = Context(bus, "glimmer-e2e")
    recorder = context.recorder
    capabilities = CAP_PREEDIT | CAP_FORMATTED_PREEDIT | CAP_SURROUNDING_TEXT | CAP_CLIENT_SIDE_INPUT_PANEL
    context.method("SetCapability", GLib.Variant("(t)", (capabilities,)))
    context.method("FocusIn")
    pump(0.3)
    (current,) = call(bus, "/controller", CONTROLLER_INTERFACE, "CurrentInputMethod", None, "(s)")
    check(current == "glimmer", f"当前输入法是 glimmer（{current}）")

    context.type_letters("nihao")
    wait_for(
        lambda: recorder.preedits and recorder.preedits[-1].replace("'", "").replace(" ", "") == "nihao",
        "preedit 是拼音 nihao",
    )
    print(f"     preedit：{recorder.preedits[-1]!r}", flush=True)
    wait_for(lambda: any(text.split(" ")[0] == "你好" for text in recorder.last_candidates()), "候选含「你好」")
    print(f"     候选：{recorder.last_candidates()}，标签：{recorder.panels[-1]['labels']}", flush=True)
    check(context.tap(KEY_SPACE), "空格被吃掉")
    wait_for(lambda: "你好" in recorder.commits, "空格上屏「你好」")
    wait_for(lambda: recorder.preedits[-1] == "", "上屏后 preedit 清空")

    check(not context.tap(KEY_RETURN), "空闲时回车放行")

    context.type_letters("ni")
    wait_for(lambda: len(recorder.last_candidates()) >= 2, "ni 有至少两个候选")
    second = recorder.last_candidates()[1].split(" ")[0]
    commits = len(recorder.commits)
    check(context.tap(ord("2")), "数字 2 被吃掉")
    wait_for(lambda: len(recorder.commits) > commits, "数字键上屏")
    check(recorder.commits[-1] == second, f"数字 2 上屏第二个候选「{second}」（{recorder.commits[-1]}）")

    context.type_letters("hao")
    wait_for(lambda: recorder.preedits and recorder.preedits[-1] == "hao", "preedit 是 hao")
    commits = len(recorder.commits)
    check(context.tap(KEY_ESCAPE), "Esc 被吃掉")
    wait_for(lambda: recorder.preedits[-1] == "", "Esc 后 preedit 清空")
    wait_for(lambda: not recorder.last_candidates(), "Esc 后候选收起")
    pump()
    check(len(recorder.commits) == commits, "Esc 不上屏")

    check(not context.key(KEY_SHIFT_L), "Shift 按下放行")
    context.key(KEY_SHIFT_L, SHIFT_STATE, release=True)
    pump()
    # 英文模式下 Router 给英文补全候选（与 Windows / IBus 一致），字母进组句；回车原样上屏
    context.type_letters("ok")
    check(context.tap(KEY_RETURN), "英文模式回车被吃掉")
    wait_for(lambda: recorder.commits and recorder.commits[-1].strip() == "ok", "单击 Shift 切英文后回车上屏 ok")

    context.key(KEY_SHIFT_L)
    context.key(KEY_SHIFT_L, SHIFT_STATE, release=True)
    pump()
    context.type_letters("nihao")
    check(context.tap(KEY_SPACE), "空格被吃掉")
    wait_for(lambda: recorder.commits[-1] == "你好", "再单击 Shift 切回中文，nihao 空格上屏「你好」")

    context.method("FocusOut")
    context.method("DestroyIC")
    print("PASS", flush=True)


def main():
    home = tempfile.mkdtemp(prefix="glimmer-fcitx5-e2e-")
    process, log_path = start_fcitx5(home)
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    status = 0
    try:
        run(bus)
    except SystemExit as error:
        status = error.code or 1
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
    if status != 0 or os.environ.get("GLIMMER_E2E_SHOW_LOG"):
        print("---- fcitx5 与插件日志 ----", flush=True)
        with open(log_path, encoding="utf-8", errors="replace") as log:
            print(log.read()[-20000:], flush=True)
    sys.exit(status)


if __name__ == "__main__":
    main()
