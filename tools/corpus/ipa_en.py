# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""把 ipa-dict 的美音表（CMUdict 机器转的 IPA）改写成课本 / 词典（朗文、牛津美音）的写法，
给 `gloss-gen ipa` 用。程序里不再做任何换写，改写规则只在这里。

输入 `data/ipa/en_US.txt`（`词\\t/音标/, /音标/`，https://github.com/open-dict-data/ipa-dict，MIT），
输出同格式、每词只留第一种读法：`data/ipa/en_US-textbook.txt`。

CMUdict 风格与课本的差别，逐条对应下面的规则：

    ɹ → r、ɫ → l                     辅音写法
    ɛ → e                            very: ˈvɛɹi → ˈveri
    ɑ → ɑː、ɔ → ɔː、u → uː            hard: ˈhɑɹd → hɑːrd；law → lɔː
    i → iː（非重读的词尾 i 不加长音）     see → siː；city → ˈsɪti
    重读音节的 ə → ʌ                   cup: ˈkəp → kʌp；about: əˈbaʊt 不变
    重读音节的 ɝ / ɚ → ɜːr，非重读 → ər  bird → bɜːrd；butter: ˈbətɝ → ˈbʌtər
    单音节词去掉重音符                  good: ˈɡʊd → ɡʊd
    -ing 词有带 ŋ 的读法就选它           going: ˈɡoʊɪn, ˈɡoʊɪŋ → ˈɡoʊɪŋ

用法：
    uv run tools/corpus/ipa_en.py                     # 读写缺省路径
    uv run tools/corpus/ipa_en.py --show hard very good butter bird about
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

DEFAULT_INPUT = Path("data/ipa/en_US.txt")
DEFAULT_OUTPUT = Path("data/ipa/en_US-textbook.txt")

# 单音节元音符号（换写前的 CMUdict 写法）；双元音的第二个成分 ɪ / ʊ 不单独算音节
VOWELS = set("aeiouæɑɔəɛɝɚɪʊ")
STRESS = {"ˈ", "ˌ"}


def rewrite(ipa: str) -> str:
    """一条 CMUdict 风格音标 → 课本写法。"""
    out: list[str] = []
    stressed = False  # 当前音节是否带主重音（遇到 ˈ 置位，吃掉一个元音后清零）
    chars = list(ipa)
    for index, c in enumerate(chars):
        prev = chars[index - 1] if index else ""
        is_last = index == len(chars) - 1
        if c in STRESS:
            stressed = c == "ˈ"
            out.append(c)
            continue
        if c == "ɹ":
            out.append("r")
        elif c == "ɫ":
            out.append("l")
        elif c == "ɛ":
            out.append("e")
        elif c == "ɑ":
            out.append("ɑː")
        elif c == "ɔ":
            # ɔɪ 是双元音，不加长音
            nxt = chars[index + 1] if not is_last else ""
            out.append("ɔ" if nxt == "ɪ" else "ɔː")
        elif c == "u":
            out.append("uː")
        elif c == "i":
            out.append("i" if is_last and not stressed else "iː")
        elif c == "ə":
            out.append("ʌ" if stressed else "ə")
        elif c in "ɝɚ":
            out.append("ɜːr" if stressed else "ər")
        elif c in "ɪʊ" and prev in "aeoɔ":
            # 双元音 aɪ eɪ ɔɪ aʊ oʊ 的后半段，不是新音节，直接写
            out.append(c)
            continue
        else:
            out.append(c)
        if c in VOWELS:
            stressed = False
    text = "".join(out)
    if syllables(ipa) <= 1:
        text = text.replace("ˈ", "").replace("ˌ", "")
    return text


def pick(word: str, readings: str) -> str:
    """几种读法里选一种：缺省第一种；-ing 词 CMUdict 常把口语的 ɪn 排在前面，有带 ŋ 的就选它。"""
    options = [r.strip().strip("/") for r in readings.split(",")]
    options = [r for r in options if r]
    if not options:
        return ""
    if word.endswith("ing"):
        for option in options:
            if option.endswith("ŋ"):
                return option
    return options[0]


def syllables(ipa: str) -> int:
    """按元音数目数音节，双元音算一个。"""
    count = 0
    prev = ""
    for c in ipa:
        if c in VOWELS and not (c in "ɪʊ" and prev in "aeoɔ"):
            count += 1
        prev = c
    return count


def convert(source: Path, target: Path) -> tuple[int, int]:
    """整张表换写，返回（读入行数, 写出行数）。"""
    read = written = 0
    with source.open(encoding="utf-8") as fin, target.open("w", encoding="utf-8") as fout:
        for line in fin:
            line = line.rstrip("\n")
            if not line or "\t" not in line:
                continue
            read += 1
            word, readings = line.split("\t", 1)
            first = pick(word, readings)
            if not first:
                continue
            fout.write(f"{word}\t/{rewrite(first)}/\n")
            written += 1
    return read, written


def main() -> None:
    parser = argparse.ArgumentParser(description="ipa-dict 美音表 → 课本写法")
    parser.add_argument("--input", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--show", nargs="*", help="只打印这几个词换写前后的样子，不写文件")
    args = parser.parse_args()
    if args.show:
        table = {}
        with args.input.open(encoding="utf-8") as fin:
            for line in fin:
                word, _, readings = line.rstrip("\n").partition("\t")
                if word in args.show:
                    table[word] = pick(word, readings)
        for word in args.show:
            raw = table.get(word)
            print(f"{word}\t{raw}\t{rewrite(raw) if raw else '-'}")
        return
    read, written = convert(args.input, args.output)
    print(f"读 {read} 行，写 {written} 行 → {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
