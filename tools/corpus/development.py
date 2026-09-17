#!/usr/bin/env python3
"""从固定提交的 Vue/Rust 中文文档归档提取开发正文，记录逐段出处。

仅接收本地归档，按 sources.json 的 SHA256 校验；不自动下载或读取项目私有文档。
输出到 assets/lexicon/ai，保留原作者署名与许可证，不采集图片或代码块。
"""

import argparse
import hashlib
import html
import json
import re
import tarfile
from pathlib import Path


def paragraphs(markdown: str) -> list[str]:
    """清除代码、HTML、Markdown 标记；只留下至少 30 个汉字的正文段落。"""
    markdown = re.sub(r"\A---\n.*?\n---\n", "", markdown, flags=re.S)
    markdown = re.sub(r"<!--.*?-->", "", markdown, flags=re.S)
    markdown = re.sub(r"```.*?```|~~~.*?~~~", "", markdown, flags=re.S)
    markdown = re.sub(r"<(script|style)\b[^>]*>.*?</\1>", "", markdown, flags=re.S)
    result = []
    for block in re.split(r"\n\s*\n", markdown):
        block = block.strip()
        if not block or block.startswith(("#", "!", "|", "<", ":::", "[")):
            continue
        block = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", block)
        block = re.sub(r"\[([^\]]+)\]\[[^\]]*\]", r"\1", block)
        block = re.sub(r"<[^>]+>", "", block)
        block = re.sub(r"\{#[^}]*\}", "", block)
        block = html.unescape(block.replace("`", "").replace("**", ""))
        block = " ".join(block.split())
        if len(re.findall(r"[\u4e00-\u9fff]", block)) >= 30:
            result.append(block)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path("assets/lexicon/ai"))
    parser.add_argument("--vue", type=Path, required=True)
    parser.add_argument("--rust-book", type=Path, required=True)
    args = parser.parse_args()
    manifest = json.loads((args.source / "sources.json").read_text())
    seen: set[str] = set()
    corpus, provenance = [], []
    for source, path in [("vue", args.vue), ("rust-book", args.rust_book)]:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != manifest[source]["sha256"]:
            raise ValueError(f"archive checksum mismatch: {source}")
        with tarfile.open(path) as archive:
            for member in sorted(archive.getmembers(), key=lambda item: item.name):
                if not member.isfile() or not member.name.endswith(".md"):
                    continue
                relative = member.name.split("/", 1)[1]
                if source == "vue" and not relative.startswith(("src/guide/", "src/api/")):
                    continue
                if source == "rust-book" and not relative.startswith("src/ch"):
                    continue
                stream = archive.extractfile(member)
                if stream is None:
                    continue
                for block in paragraphs(stream.read().decode("utf-8")):
                    if block in seen:
                        continue
                    seen.add(block)
                    corpus.append(block)
                    provenance.append(f"{len(corpus)}\t{source}\t{relative}")
    if not corpus:
        raise ValueError("no development paragraphs extracted")
    (args.source / "development.txt").write_text("\n".join(corpus) + "\n")
    (args.source / "development-sources.tsv").write_text(
        "# development.txt 行号\t来源 ID（固定提交见 sources.json）\t原文件\n"
        + "\n".join(provenance) + "\n"
    )
    print(f"development corpus: {len(corpus)} paragraphs")


if __name__ == "__main__":
    main()
