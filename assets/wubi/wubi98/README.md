# 五笔 98 码表

随包的五笔 98 码表，来自 [yanhuacuo/98wubi](https://github.com/yanhuacuo/98wubi) 的 `wubi98_ci.dict.yaml`
（`ci` = 含词表，单字 + 词组齐全，每个单字都带该字的四码全码 `stem`）。
输入法与 CLI 用的是它转出来的 `data/generated/wubi98.qj`，本目录只放上游原文与许可。三个方案的索引见 [../README.md](../README.md)。

同一作者另有 [yanhuacuo/98wubi-tables](https://github.com/yanhuacuo/98wubi-tables) 仓库的 `wubi98_U.dict.yaml`（Unlicense）。
那是**纯单字表**，10.9 万条全是单字、一条词组都没有，按常用字集过滤后只剩 1.3 万条，敲多字词只能逐字上屏，所以不用它。

## 文件

| 文件 | 内容 |
| --- | --- |
| `wubi98_ci.dict.yaml` | 上游码表原文，未改动。YAML 头记录列序（`text` / `code` / `weight` / `stem`）与造词规则，表体每行 `字词\t编码\t词频\t全码`；四列齐全，CRLF 行尾，无 BOM |
| `LICENSE.LGPL-3.0` | 上游 `LICENSE.txt`，GNU LGPL 3.0 全文 |

上游没有 `AUTHORS` 文件，署名信息只在码表文件头的注释里（`# 2023-11-26`、`# key-word 潀-ilww`）。

## 来源与版本

- 仓库：<https://github.com/yanhuacuo/98wubi>，分支 `master`
- 提交：`7fa4a71f62af840c182a7a5417ba5090d0054bf8`（2025-07-10，提交信息 `Add files via upload`）
- 码表版本：YAML 头 `version: "3.0"`，`name: wubi98_ci`
- 码表文件头的注释另写了一个修订日期 `# 2023-11-26`，那是码表内容的修订日期。`--data-version` 里跟 86、新世纪一样记**提交日期**，所以是 2025-07-10
- 取回日期：2026-09-17，两个文件都从 `https://raw.githubusercontent.com/yanhuacuo/98wubi/master/` 直接下载
- `wubi98_ci.dict.yaml` SHA256：`770bead050a4aae461e2de99854e89405d50c970c7434841becc900e99ed6478`

## 许可与署名

许可证 **LGPL-3.0**（仓库 `LICENSE.txt`）。码表整理者 yanhuacuo（98五笔小筑），五笔字型与 98 字根方案原作者 **王永民**。
五笔 98 的字根与编码规则本身已进入公有领域，这里要署名的是码表编者。分发时随包附 `LICENSE.LGPL-3.0` 与本页署名。

## 规模

| 项 | 数量 |
| --- | --- |
| 文件总行数 | 112,616 |
| 表体条目 | 112,592（全部四列；其中 8 条是编码含 `z` 的标点行，`z` 键留给反查，转换时跳过） |
| 单字 | 36,442（含扩展区生僻字） |
| 词 | 76,150（二字 59,032、三字 6,899、四字 9,424、五字及以上 795） |
| 带 `stem` 列的行 | 36,442（**每个单字都有**，写的是该字的四码全码，不是 86 那样只有一级简码的前两码） |
| 转换后（常用字集） | 89,103 条，单字 13,169、词 75,934；单字全码 8,037 个，其中有四码全码的 7,059 |

转换日志：读入 112,584、写出 89,103、过滤 23,481、合并 0、非法 0、跳过 `z` 码 8、`in_code_order_weights=false`（上游词频齐全，一行不动）；
`stem` 与单字全码**全部一致，0 处 warn**。

一级简码 25 字与 86、新世纪**完全相同**。98 额外把 25 个键名字也编在一码上（`a戈 b子 c又 d大 e月 f土 g王 h目 i水 j日 k口 l田 m山 n已 o火 p之 q金 r白 s木 t禾 u立 v女 w几 x母 y言`），
词频比对应的一级简码低 1，排在简码之后；86 与新世纪把其中 22 个键名字用 `#` 注释掉了（`a` / `w` / `x` 三键的键名字在那两套里本来就不占一码位）。

## 怎么用

```sh
cargo run --release -p glimmer-dict-convert -- wubi --from rime assets/wubi/wubi98/wubi98_ci.dict.yaml \
  --out data/generated/wubi98.tsv --charset data/generated/dict.qj
cargo run --release -p glimmer-dict-convert -- pack dict --input data/generated/wubi98.tsv --output wubi98.qj \
  --name "五笔 98 码表" --license LGPL-3.0 \
  --attribution "yanhuacuo/98wubi（LGPL-3.0）/ 字根 王永民" \
  --source "https://github.com/yanhuacuo/98wubi" --data-version "3.0 (7fa4a71, 2025-07-10)"
```

生成的 `data/generated/wubi98.qj` 约 3.0 MB，89,103 条。
