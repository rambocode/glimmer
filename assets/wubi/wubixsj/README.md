# 新世纪五笔码表

随包的新世纪五笔码表，来自 [GuoBinyong/wubixinshiji](https://github.com/GuoBinyong/wubixinshiji) 的 `wubixinshiji.dict.yaml`
（郭斌勇从《五笔字型大一统普及版 + 专业版新世纪整合词库》转成 Rime 格式，带词频与一级简码的构词码列）。
输入法与 CLI 用的是它转出来的 `data/generated/wubixsj.qj`，本目录只放上游原文与许可。三个方案的索引见 [../README.md](../README.md)。

## 文件

| 文件 | 内容 |
| --- | --- |
| `wubixinshiji.dict.yaml` | 上游码表原文，未改动。YAML 头记录列序（`text` / `code` / `weight` / `stem`）与造词规则，表体每行 `字词\t编码\t词频[\t构词码]`；CRLF 行尾，无 BOM |
| `AUTHORS` | 上游 `AUTHORS`，署名与许可声明 |

上游**没有 LICENSE 文件**，LGPL 全文不在本目录重复放一份，直接引用 [`../wubi86/LICENSE.LGPL-3.0`](../wubi86/LICENSE.LGPL-3.0)。

## 来源与版本

- 仓库：<https://github.com/GuoBinyong/wubixinshiji>，分支 `master`
- 提交：`09093429180d3a84b1002342d695e208c0d3c55d`（2022-08-31）
- 码表版本：YAML 头 `version: "0.6"`，`name: wubixinshiji`
- 取回日期：2026-09-17，两个文件都从 `https://raw.githubusercontent.com/GuoBinyong/wubixinshiji/master/` 直接下载
- `wubixinshiji.dict.yaml` SHA256：`3eeaaa17891aabf4148264ae73b4bc258d31cbffbb674fc7688aa0a599e6dd27`

## 许可与署名

**上游仓库没有 LICENSE 文件**，许可只在 `AUTHORS` 里声明：

> 郭斌勇整理的新世纪版五笔词库 | 该词库由尊敬的王永民先生发明
>   wubixinshiji.dict.yaml
>
> 郭斌勇 <guobinyong@qq.com> : (LGPL)
>   wubixinshiji.*.yaml

按 `AUTHORS` 的声明当 **LGPL-3.0** 处理，全文引用 `../wubi86/LICENSE.LGPL-3.0`（同一份 GNU LGPL 3.0）。
词库出自王永民《五笔字型大一统普及版 + 专业版新世纪整合词库》，由郭斌勇转换并格式化成 Rime 字典。
新世纪五笔的字根与编码规则本身已进入公有领域，这里要署名的是码表编者。分发时随包附 LGPL 全文与本页署名。

## 规模

| 项 | 数量 |
| --- | --- |
| 文件总行数 | 112,137 |
| 表体条目 | 112,082（其中 609 条是编码含 `z` 的符号与注音行，`z` 键留给反查，转换时跳过） |
| 只有两列（`词\t编码`，**没有词频**）的行 | 111,448（占 99.4%，见下面「补词频」） |
| 单字 | 32,627 |
| 词 | 79,455（二字 51,048、三字 10,174、四字 16,008、五字及以上 2,225） |
| 注释掉的表体行 | 22（`#子 b`、`#又 c` 这类键名字，上游用 `#` 关掉了一码位，与 86 的做法一致） |
| 带 `stem` 列的行 | 25（只有一级简码那 25 个字有，写的是该字全码的前两码） |
| 转换后（常用字集） | 91,816 条，单字 8,013、有四码全码的 7,175 |

转换日志：读入 111,473、写出 91,816、过滤 19,632、合并 25、非法 0、跳过 `z` 码 609、`in_code_order_weights=true`；`stem` 与单字全码不一致 **4 处**
（`上 hh≠hgd`、`不 gi≠dhi`、`民 na≠nnav`、`为 yl≠ytny`）。这 4 条是上游把 86 的 `stem` 原样留着没跟着新世纪的编码改，
只影响 Core 反查表的自检日志，不影响候选与上屏。

一级简码 25 字与 86、98 **完全相同**。

## 补词频

这份表 99.4% 的行只有 `词\t编码` 两列，没有 weight 列，原样转出来全表词频都是 0。
Core 排同码候选时词频打平就退到按文本比大小（`crates/glimmer-core/src/engine/query/wubi.rs` 的 `WubiHit::key`），
候选顺序会变成 Unicode 码位序，跟上游排好的常用度完全无关（`aaaa` 下 `恭恭敬敬` 会掉到 `工戈草头右框七` 后面）。

所以 `dict-convert wubi` 在「一半以上的行没有 weight 列」时补词频，日志里 `in_code_order_weights=true`。
补法是**同一编码下按出现顺序编名次 r（0 起），词频取 `1_000_000 / (r + 1)`**（整数除法，最小 1）：

| r | 0 | 1 | 2 | 3 | 4 |
| --- | --- | --- | --- | --- | --- |
| 词频 | 1,000,000 | 500,000 | 333,333 | 250,000 | 200,000 |

两点都得这么做，缺一不可：

- **名次倒数，不是线性递减。** Core 的分数是 `ln(词频 / 总频)` 再 `(score * 1000).round()` 取整比较。整表线性递减时
  相邻两条只差 1，对数差远小于千分之一，取整后完全打平，等于没补。倒数让前几名的对数差拉开到取整分得清。
- **按编码分组，不是整表连号。** 上游文件除了开头 25 行一级简码，整体是按编码排序的，「文件顺序 = 常用度」
  只在同一编码组内成立，跨编码组比大小没有意义。所以每组各自从 1,000,000 起算，组与组之间不可比。

86（99.9% 有 weight）与 98（全有）判据不成立，一行不动，输出与改动前逐字节相同。
补出来的词频只在本表内部有意义，不能跨方案比大小；一个 `.qj` 单独用，所以没问题。

实测（`RUST_LOG=warn cargo run -q -p glimmer-cli -- --wubi xsj <码>`）：`gggg` 首选「王」，`aaaa` 是「工 / 恭恭敬敬 / 工戈草头右框七」，与上游文件顺序一致。

## 怎么用

```sh
cargo run --release -p glimmer-dict-convert -- wubi --from rime assets/wubi/wubixsj/wubixinshiji.dict.yaml \
  --out data/generated/wubixsj.tsv --charset data/generated/dict.qj
cargo run --release -p glimmer-dict-convert -- pack dict --input data/generated/wubixsj.tsv --output wubixsj.qj \
  --name "新世纪五笔码表" --license LGPL-3.0 \
  --attribution "GuoBinyong/wubixinshiji（郭斌勇）/ 王永民" \
  --source "https://github.com/GuoBinyong/wubixinshiji" --data-version "0.6 (0909342, 2022-08-31)"
```

生成的 `data/generated/wubixsj.qj` 约 3.2 MB，91,816 条。
这份表上游没给词频，转换时会按上面「补词频」那一套补上（同一编码下 `1_000_000 / (名次 + 1)`），不用额外加参数。
