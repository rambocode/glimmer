# 五笔码表

随包的三套五笔码表原文，一个方案一个子目录。上游都是 Rime 的 `.dict.yaml` 四列格式，同一个 `dict-convert wubi` 直接吃。
本目录只放上游原文与许可，输入法与 CLI 用的是转出来的 `data/generated/wubi<方案>.qj`。

## 三个方案

| 方案 | 子目录 | 上游 | 许可 | 规模（表体 / 单字 / 词） |
| --- | --- | --- | --- | --- |
| 五笔 86（极点） | [`wubi86/`](wubi86/README.md) | [rime/rime-wubi](https://github.com/rime/rime-wubi) `wubi86.dict.yaml` | LGPL-3.0 | 136,916 / 75,696 / 61,220 |
| 五笔 98（含词表） | [`wubi98/`](wubi98/README.md) | [yanhuacuo/98wubi](https://github.com/yanhuacuo/98wubi) `wubi98_ci.dict.yaml` | LGPL-3.0 | 112,592 / 36,442 / 76,150 |
| 新世纪五笔 | [`wubixsj/`](wubixsj/README.md) | [GuoBinyong/wubixinshiji](https://github.com/GuoBinyong/wubixinshiji) `wubixinshiji.dict.yaml` | LGPL-3.0（见 AUTHORS，上游无 LICENSE 文件） | 112,082 / 32,627 / 79,455 |

转出来的 `.qj`：`wubi86.qj` 73,647 条 / 2.6 MB，`wubi98.qj` 89,103 条 / 3.0 MB，`wubixsj.qj` 91,816 条 / 3.2 MB。

三份码表的**一级简码 25 字完全相同**：`a工 b了 c以 d在 e有 f地 g一 h上 i不 j是 k中 l国 m同 n民 o为 p这 q我 r的 s要 t和 u产 v发 w人 x经 y主`。
98 额外把 25 个键名字（`a戈 b子 c又 d大 e月 f土 g王 h目 i水 j日 k口 l田 m山 n已 o火 p之 q金 r白 s木 t禾 u立 v女 w几 x母 y言`）
也编在一码上，词频比一级简码低一位，排在简码之后。

## 怎么用

`dict-convert wubi` 把 Rime 码表转成微明词库 TSV（`词\t编码\t词频`，编码整个当一个音节），再用 `pack dict --output` 打成 `.qj`：

```sh
cargo run --release -p glimmer-dict-convert -- wubi --from rime assets/wubi/<方案>/<码表>.dict.yaml \
  --out data/generated/wubi<方案>.tsv --charset data/generated/dict.qj
cargo run --release -p glimmer-dict-convert -- pack dict --input data/generated/wubi<方案>.tsv \
  --output wubi<方案>.qj --name "…" --license "…" --attribution "…" --source "…" --data-version "…"
```

各方案的完整命令（含元数据）写在各自的 README 里。要点：

- `--out` 必填，不给缺省值，免得几个方案互相覆盖。
- 缺省只保留每个字都在**常用字集**（GB2312 一二级汉字 + `--charset` 主词库里出现过的字）里的条目，`--extended` 保留全部。
- 编码只取 `a`–`y`、最长四码；含 `z` 的行（符号表、字根表）全部跳过，`z` 键留给反查。
- 转换时打印读入 / 写出 / 过滤 / 跳过条数，以及单字全码统计（每个单字取最长码，与 `stem` 列核对，不一致只记 warn）。
- 上游文件有 CRLF 行尾与 UTF-8 BOM 的，解析器都会剥掉。
- 一半以上的行没有 weight 列时（新世纪就是这样），按**同一编码下的出现顺序**补词频：名次 r（0 起）取
  `1_000_000 / (r + 1)`，组与组之间不可比，日志里 `in_code_order_weights=true`；否则一行不动。
  为什么是名次倒数而不是线性递减，见 [`wubixsj/README.md`](wubixsj/README.md) 的「补词频」。
