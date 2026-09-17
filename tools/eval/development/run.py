#!/usr/bin/env python3
"""用真实 CLI 对独立任务集、普通中文回归集及逐键耗时做前后对比。

不读取个人配置或学习日志。输出 JSON 报告及每条验收的候选，失败以非零状态退出。
耗时采用同进程预热后的完整输入查询时间，记录为对比证据，不以单机抖动作为 CI 门禁。
"""
import argparse
import json
import os
import re
import statistics
import subprocess
import tempfile
from pathlib import Path


def duration_us(value: str) -> float:
    number, unit = re.fullmatch(r"([\d.]+)(ns|µs|μs|us|ms|s)", value.strip()).groups()
    return float(number) * {"ns": .001, "µs": 1, "μs": 1, "us": 1, "ms": 1000, "s": 1000000}[unit]


def query(cli: Path, root: Path, config: Path, extra: list[Path],
          mode: str, inputs: list[str]) -> dict[str, dict]:
    command = [str(cli), "--config", str(config), "--dict", str(root / "assets/lexicon/dict.tsv"),
               "--glossary", str(root / "assets/sample/glossary-en.tsv"),
               "--english", str(root / "assets/lexicon/english.tsv"), "--limit", "9"]
    for path in extra:
        command += ["--extra-dict", str(path)]
    if mode == "en":
        command += ["--english-mode"]
    completed = subprocess.run(command + inputs, cwd=root, env={**os.environ, "RUST_LOG": "warn"},
                               capture_output=True, text=True, timeout=180, check=True)
    result = {}
    for part in re.split(r"^> ", completed.stdout, flags=re.M)[1:]:
        code = part.splitlines()[0].strip()
        candidates = []
        for line in part.splitlines():
            match = re.match(r"^\s+\d+\.\s+(.+?)(?:\s{2,}|\s+\[en\]|$)", line)
            if match:
                candidates.append(match.group(1).strip())
        times = re.findall(r" · total (\S+)", part)
        result[code] = {"candidates": candidates, "microseconds": duration_us(times[-1]) if times else None}
    if any(code not in result for code in inputs):
        raise ValueError("CLI output is missing queries")
    return result


def p95(values: list[float]) -> float:
    return sorted(values)[min(len(values) - 1, int(len(values) * .95))]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--extra", type=Path, required=True)
    parser.add_argument("--baseline-extra", type=Path, help="可选：上一版词库，用于回归和耗时基线")
    parser.add_argument("--output", type=Path, default=Path("data/generated/development-eval.json"))
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    fixture = Path(__file__).parent
    rows = [line.split("\t") for line in (fixture / "acceptance.tsv").read_text().splitlines()
            if line and not line.startswith("#")]
    general = (fixture / "general.txt").read_text().splitlines()
    results, metrics, regression = [], {}, []
    baseline_extras = [args.baseline_extra.resolve()] if args.baseline_extra else []
    with tempfile.TemporaryDirectory(prefix="glimmer-eval-") as temp:
        config = Path(temp) / "config.toml"
        config.write_text("[predict]\nenabled = false\n[model]\nenabled = false\n")
        for mode in ["zh", "en"]:
            inputs = list(dict.fromkeys(row[2] for row in rows if row[1] == mode))
            found = query(args.cli.resolve(), root, config, [args.extra.resolve()], mode, inputs)
            for scenario, row_mode, code, word, max_rank in rows:
                if row_mode != mode:
                    continue
                candidates = found[code]["candidates"]
                rank = candidates.index(word) + 1 if word in candidates else None
                results.append({"scenario": scenario, "mode": mode, "input": code, "word": word,
                                "rank": rank, "max_rank": int(max_rank),
                                "pass": rank is not None and rank <= int(max_rank),
                                "candidates": candidates})
        baseline = query(args.cli.resolve(), root, config, baseline_extras, "zh", general)
        enabled = query(args.cli.resolve(), root, config, [args.extra.resolve()], "zh", general)
        for code in general:
            before, after = baseline[code]["candidates"], enabled[code]["candidates"]
            if before[:1] != after[:1]:
                regression.append({"input": code, "before": before[:5], "after": after[:5]})
        # 同一查询进程内先预热一次，再重复普通中文与代表性开发词；交错运行避免固定顺序偏差。
        probes = general + ["jiansuozengqiangshengcheng", "xiangliangzhaohui", "jiejuehebingchongtu", "gitfenzhi"]
        samples = {"baseline": [], "enabled": []}
        for trial in range(3):
            order = [("baseline", baseline_extras), ("enabled", [args.extra.resolve()])]
            if trial % 2:
                order.reverse()
            for label, extras in order:
                data = query(args.cli.resolve(), root, config, extras, "zh", probes + probes)
                samples[label].extend(v["microseconds"] for v in data.values() if v["microseconds"] is not None)
        for label, values in samples.items():
            metrics[label] = {"samples": len(values), "median_us": statistics.median(values), "p95_us": p95(values)}
    zh = [row for row in results if row["mode"] == "zh"]
    en = [row for row in results if row["mode"] == "en"]
    report = {"baseline": str(args.baseline_extra) if args.baseline_extra else "no-extra", "acceptance": {"total": len(results), "passed": sum(r["pass"] for r in results),
                            "chinese_top5_rate": sum(r["pass"] for r in zh) / len(zh),
                            "required_english_all_pass": all(r["pass"] for r in en)},
              "general": {"total": len(general), "first_candidate_changes": regression},
              "timings": metrics, "cases": results}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k != "cases"}, ensure_ascii=False, indent=2))
    for row in results:
        if not row["pass"]:
            print("MISS", row["input"], row["word"], row["candidates"])
    if report["acceptance"]["chinese_top5_rate"] < .95 or not all(r["pass"] for r in en) or regression:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
