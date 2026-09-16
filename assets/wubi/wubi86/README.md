# 五笔 86 码表

随包的五笔 86 码表，来自 [rime/rime-wubi](https://github.com/rime/rime-wubi) 的 `wubi86.dict.yaml`（极点五笔 6.0 血统，带词频与一级简码的构词码列）。
输入法与 CLI 用的是它转出来的 `data/generated/wubi86.qj`，本目录只放上游原文与许可。三个方案的索引见 [../README.md](../README.md)。

## 文件

| 文件 | 内容 |
| --- | --- |
| `wubi86.dict.yaml` | 上游码表原文，未改动。YAML 头记录列序（`text` / `code` / `weight` / `stem`）与造词规则，表体每行 `字词\t编码\t词频[\t构词码]` |
| `LICENSE.LGPL-3.0` | 上游仓库 `LICENSE`，GNU LGPL 3.0 全文 |
| `AUTHORS` | 上游 `AUTHORS`，署名与血统 |

## 来源与版本

- 仓库：<https://github.com/rime/rime-wubi>，分支 `master`
- 提交：`152a0d3f3efe40cae216d1e3b338242446848d07`（2023-10-25，「wubi86.dict.yaml: 添加遗漏的《通用规范汉字表》字」）
- 码表版本：YAML 头 `version: "0.7"`
- 取回日期：2026-09-16，三个文件都从 `https://raw.githubusercontent.com/rime/rime-wubi/master/` 直接下载
- `wubi86.dict.yaml` SHA256：`f833d86b72341fe82e069a425b6625f29ef85f1bc0f34f6fb7975fe514888b5a`

## 许可与署名

许可证 **LGPL-3.0**（仓库 `LICENSE`）。`AUTHORS` 写明的血统：

> Derived from ibus-table by acevery (Yu Yuwei) : (LGPL)
> and based on original work by 王永民 / Wang Yongmin : (Public Domain)

文件头的更新记录：原表作者 Wozy（极点五笔），Chen Xing 从极点码表转出，Yu Yuwei 升到极点 6、并入 Google 词频、恢复原版一二级简码，Gong Chen 转成 Rime 格式。
五笔 86 的字根与编码规则本身已进入公有领域，这里要署名的是码表编者。分发时随包附 `LICENSE.LGPL-3.0` 与本页署名。

## 规模

| 项 | 数量 |
| --- | --- |
| 文件总行数 | 136,981 |
| 表体条目 | 136,916（其中 677 条是编码含 `z` 的符号，`z` 键留给反查，转换时跳过） |
| 单字 | 75,696（含大量扩展 A / B 区生僻字） |
| 词 | 61,220（二字 48,458、三字 4,520、四字 7,934、五字及以上 308） |
| 转换后（常用字集） | 73,647 条，单字 8,236、有四码全码的 7,335 |

编码只用 `a`–`y`，最长四码；`stem` 列只有 25 个一级简码字有，是该字全码的前两码（`工 a → aa`、`了 b → bn`）。

## 怎么用

`dict-convert wubi` 把它转成微明词库 TSV（`词\t编码\t词频`，编码整个当一个音节），缺省只保留每个字都在常用字集
（GB2312 一二级汉字 + 主词库里出现过的字）里的条目，`--extended` 保留全部；再用 `pack dict --output` 打成 `.qj`：

```sh
cargo run --release -p glimmer-dict-convert -- wubi --from rime assets/wubi/wubi86/wubi86.dict.yaml \
  --out data/generated/wubi86.tsv --charset data/generated/dict.qj
cargo run --release -p glimmer-dict-convert -- pack dict --input data/generated/wubi86.tsv --output wubi86.qj \
  --name "五笔 86 码表（极点）" --license LGPL-3.0 \
  --attribution "rime-wubi（Gong Chen、Yu Yuwei）/ 极点五笔 Wozy、Chen Xing / 字根 王永民" \
  --source "https://github.com/rime/rime-wubi" --data-version "0.7 (152a0d3, 2023-10-25)"
```

转换时会打印读入 / 写出 / 过滤 / 跳过的条数，以及单字全码统计（每个单字取最长码，与 `stem` 列核对，不一致只记 warn）。
