# 五笔支持方案

## 状态（2026-09-16）

- 第 1–4 步已完成：`assets/wubi/` 三个文件与 `dict-convert wubi`（字符集过滤、stem 核对，136,239 条 → 73,647 条）；Core `wubi` 模块、`query_wubi`（全码 / 前缀 / 简码固定序 / 空码）、
  `take_auto_commit`（四码 / 顶字）、`z` 反查注编码、`[wubi] hint`、自动造词按 `encoder` 编码进用户词，学习数据按方案分目录（`wubi86/`），CLI `--wubi 86` 与回放按日志的 `scheme` 切换；
  行为规格以 `crates/glimmer-core/src/engine/tests/wubi/` 为准。
- 第 5–6 步（macOS 壳、Windows 壳）在做，第 7 步文档与之同批。
- 第二阶段未开始（98 码表、`z` 中间位通配、对已上屏历史造词、双拼反查），待办在 `todo.md`「一、日常输入体验」。
- 与起草时的出入：王 的全码是 `gggg`（`ggll` 是 一）；字集过滤在码表生成时做（`dict-convert wubi --extended`），没有运行时的增广字集开关，要出全部字得换一份码表；
  五笔下组句联想（含 `?` 问字）不发云端，只有译词兜底与翻译选中文字照常。

2026-09-16 起草。目标：在现有拼音引擎旁边加一套**码表输入**（五笔 86 优先、98 其次），复用词频学习、翻译 annotation、emoji、云联想、候选窗与偏好设置，
不动拼音那条路的任何行为。选型原则：码表用许可干净的开源成熟数据，行为参考 librime `table_translator` + fcitx5 `table` 引擎的缺省组合，自己只写引擎逻辑（Rust 生态没有可直接依赖的码表引擎 crate，librime 只能 FFI 链 C++，不合「Core 平台无关、单二进制」的约束）。

## 一、数据选型（许可证已到各仓库 LICENSE 核对）

| 方案 | 来源 | 许可证 | 规模 | 结论 |
|---|---|---|---|---|
| 86 五笔 | [rime/rime-wubi](https://github.com/rime/rime-wubi) `wubi86.dict.yaml`（极点 6.0 血统，带词频与 stem 列） | LGPL-3.0（仓库 `LICENSE`；`AUTHORS` 写明派生自 ibus-table 极点表 + 王永民公有领域字根） | ≈ 13.7 万条 | **首选**。GPL-3.0-or-later 项目可直接分发，附许可与署名 |
| 86 五笔（备选） | [mike-fabian/ibus-table-chinese](https://github.com/mike-fabian/ibus-table-chinese) `tables/wubi-jidian/wubi-jidian86.txt` | 文件头 `Freely redistributable without restriction` | ≈ 13.7 万条 | 与上面同源，只作核对 |
| 98 五笔 | [yanhuacuo/98wubi-tables](https://github.com/yanhuacuo/98wubi-tables) `wubi98_U.dict.yaml` | Unlicense（公有领域声明） | ≈ 11 万条 | **第二阶段**。fcitx5-table-extra 的 `wubi98.txt` 同源，声明 public domain |
| 新世纪五笔 | GuoBinyong/wubixinshiji、myshiqiqi/rime-wubi、06wb 资源库 | 无 LICENSE，王码公司仍主张权利 | — | **不做**，等出现许可干净的源再说 |
| 海峰五笔（wubi-large） | ibus-table `wubi-haifeng`（BSD-3）/ fcitx `wubi-large.txt`（声明 public domain） | 两处声明不一致，README 有「不得商用」旧文 | 15 万 | 不用，86 用极点表已够 |
| QQ 五笔合并表 | missdeer/wubi-tables | Apache-2.0 但 QQ 码表来源存疑 | — | 不用 |

86 版字根与编码本身的专利（1985 申请）早已届满，维基百科明确「编码进入开源领域」；风险只在**码表编者的版权**，所以按仓库许可挑即可。
98 与新世纪版王码公司仍有主张，98 社区自称「已无版权问题」但无文书，放第二阶段并在 `assets/wubi/README.md` 写明来源。

数据进仓库：`assets/wubi/wubi86.dict.yaml`（≈ 3 MB）+ `LICENSE.LGPL-3.0` + `README.md`（来源 URL、提交哈希、AUTHORS 摘录）。
不用 submodule（构建与 CI 都要它，且要定版）。随包数据 `wubi86.qj` 走现有 `data` Release 流程（`tools/release/data-bundle.sh`）。

## 二、行为规范（照 fcitx5 `wbx.conf` 缺省 + librime `wubi86.schema.yaml`）

五笔用户的肌肉记忆是产品的硬约束，下面每条都是两家实现的共同缺省，不自创：

| 行为 | 规则 | 出处 |
|---|---|---|
| 编码键 | `a`–`y` 是编码，最长 4 码 | 码表 `Length=4` / `max_code_length` |
| 候选来源 | 全码命中在前（按词频 + 用户词频），再接前缀命中（逐键提示，`enable_completion`）；短码优先 | fcitx `SortByCodeLength`、librime completion quality −1 |
| 简码不调频 | 输入 ≤ 2 码时只按码表静态顺序，不按用户词频重排（一级 / 二级简码位置固定） | fcitx `NoSortInputLength=2` |
| 四码上屏 | 敲满 4 码且有全码命中 → 首选自动上屏（可关） | fcitx `AutoSelect` + `AutoSelectLength=-1`、librime `speller/auto_select` |
| 顶字上屏 | 缓冲区里已有编码，新键接上后**没有任何命中**（含前缀）→ 先把旧段首选上屏，新键开新段 | fcitx `NoMatchAutoSelectLength=-1` |
| 空码 | 4 码无命中：候选空，缓冲区保留可退格；再敲字母则清掉旧码从新键开始；`Space` / `Enter` 把原码原样上屏 | fcitx `CommitRawInput=false` + `auto_clear` |
| `z` 键 | 在第一位 = 拼音反查（`z` + 全拼，候选右侧注五笔码）；在其他位 = 通配符（第二阶段） | fcitx `PinyinKey=z` + `MatchingKey=z`；rime `reverse_lookup.prefix: z` |
| 编码提示 | 反查候选与逐键提示候选右侧显示完整编码，普通候选可选显示 | fcitx `Hint=True`、rime `comment_format` |
| 自动造词 | 连着上屏的词合成 2–4 字新词，编码按规则生成：二字 `AaAbBaBb`、三字 `AaBaCaCb`、四字及以上 `AaBaCaZa`（大写 = 第几个字，小写 = 该字全码第几位，Z = 末字）；某字全码不够长就不造 | rime `encoder.rules`、fcitx `[Rule]` 段两家一致 |
| 字符集 | 缺省只出常用字集（GB2312 + 主词库出现过的字），过滤在码表生成时做（`--extended` 保留全部） | rime `enable_charset_filter` + `extended_charset` |
| 中 / 英 | 不变，仍是布尔切换；标点、翻页、选词键与拼音完全一致 | 本仓库「输入方案是配置项不是模式」 |

不做的：整句转换（五笔无需）、神经重排（关掉）、纠错与模糊音（无意义）、`;` `'` 二三候选键（fcitx 98 有，先不做，冲突现有 `'` 分隔与标点约定）。

## 三、架构

```text
crates/glimmer-core/src/wubi/            # 新模块，与 shuangpin / zhuyin 平级
├── mod.rs          # mod 声明与 re-export
├── scheme.rs       # Scheme：码表 + 反查表 + 造词规则 + 选项，Engine 里是 Option<Scheme>
├── encoder.rs      # 造词规则（AaAbBaBb 三条）：&[char] → Option<String>
├── reverse.rs      # 反查表 char → 全码（从码表单字条目建：同字取最长码，等长取词频高的；与 stem 列核对）
├── options.rs      # Options：auto_select / hint / fixed_order_length
└── tests.rs
```

- **码表复用 `glimmer-dictionary::Dictionary`，编码当一个「音节」。** TSV 就是 `王\tgggg\t9000`。前缀查询 `lookup_pattern(&[SyllablePattern{text:"gg", complete:false}])`
  天然就是逐键提示，全码用 `lookup_exact`；`.qj` 打包、mmap、META 许可证、附加词库装配全部现成。不新建 crate。
- **Engine 加一个字段 `wubi: Option<wubi::Scheme>`**，`query_inner` 在 `english_mode` 判断之后分派到 `query_wubi`（新文件 `engine/query/wubi.rs`），
  拼音那条路一行不动。`is_raw` / `scheme_key` / `marked` 各加一个分支（`scheme_key` 返回 `wubi86`，输入日志与回放据此走五笔）。
- **候选**：`Candidate { text, kind: Chinese, syllables: vec![code], reading: Some(code) 仅提示时, translation }`。`reading` 字段两个壳已经画在最前（问字模式的读音在用），不加新字段。
- **上屏消耗**：`commit::consumed_by` 五笔分支直接吃 `syllables[0].len()` 个字节（反查候选吃整段作用域）；`choice_key` 用的输入串就是编码。
- **自动上屏**：`push` 不改签名。`Engine` 新增 `pending_auto_commit: Option<Candidate>`，`query_wubi` 按上表规则置位；
  新公开方法 `take_auto_commit() -> Option<Candidate>`，两个壳在各自 `refresh` 的开头取到就走现有 `commit_highlighted` 同一条上屏路径（学习、日志、统计全复用），
  再刷新剩余缓冲区。顶字上屏时 Core 先把旧段上屏、新键留在缓冲区，壳看到的仍是「上屏一个候选 + 继续组句」。
- **学习**：词频（`record` / `weight`，按文本）、个人 n-gram、云联想、翻译、emoji 全按文本键，直接共用。
  按**输入串**记的三张表（用户词 `user-words`、选择记录 `choices`、敲错表）在五笔下编码与拼音会撞键（`a` 是拼音音节也是 工 的一级简码），
  `FrequencyLearner` 按方案分目录：五笔的写到 `<数据目录>/wubi86/`。壳装配时按 `[general] wubi` 给路径，Core 不感知文件。
- **自动造词**：复用 `chain` 上已有的自动造词判定（同段两次 / 分段三次），五笔下把「音节」换成 `encoder` 生成的编码，生成失败不造。
- **排序**：`ranking` 加一条「固定顺序」开关：码长 ≤ `fixed_order_length` 时不叠用户权重。
- **配置**（`glimmer-platform`）：`[general] wubi = ""|"86"|"98"`，与 `shuangpin` / `zhuyin` 互斥，同时设了以五笔为准并记 warn；
  新分节 `[wubi]`：`auto_select = true`、`hint = true`、`fixed_order_length = 2`。
- **数据工具**：`dict-convert wubi --from rime assets/wubi/wubi86.dict.yaml --out data/generated/wubi86.tsv`（解析 YAML 表体 `text\tcode\tweight[\tstem]`，按字符集过滤，
  `--extended` 保留全部）；`dict-convert pack dict data/generated/wubi86.tsv --out data/generated/wubi86.qj`（META 写 LGPL-3.0 与署名）。98 版同一子命令、同一格式。

## 四、实施步骤

所有改动在 worktree 里做，不碰主工作树：

```sh
git worktree add ../glimmer-wubi -b wubi
```

1. **数据（0.5 天）**：`assets/wubi/` 三个文件；`dict-convert wubi` 子命令 + 字符集过滤 + 单测（反查表、stem 核对、规模断言）；生成 `wubi86.qj`；`crate-notes.md` 记生成命令。
2. **Core 基本可用（2–3 天）**：`wubi/` 模块；`Engine::set_wubi`；`query_wubi`（全码 + 前缀、简码固定序、空码）；`consumed_by` 五笔分支；`take_auto_commit`（四码 / 顶字）；
   学习目录分方案；CLI `--wubi 86`。测试：`gggg` → 王、`a` → 工、`aaaa` → 工、`gg` 只出前缀提示、四码自动上屏、`ggggx` 顶字后缓冲区剩 `x`、空码退格。
   验证：`cargo run -p glimmer-cli -- --wubi 86 gggg`，`--typing` 逐键计时（目标每键 < 1 ms，前缀区间小）。
3. **反查与提示（1 天）**：`z` 前缀走拼音路径、候选 `reading` 注编码；逐键提示候选注剩余码；`[wubi] hint`。
4. **自动造词（0.5 天）**：`encoder` 三条规则接进 `chain` 的自动造词；用户词落 `wubi86/user-words.tsv`。
5. **macOS 壳（1 天）**：配置读取与热加载、偏好设置「通用」页「五笔」弹出菜单（关 / 86 / 98）、`refresh` 开头接 `take_auto_commit`、菜单栏方案名、`bundle.sh` 带 `wubi86.qj`。
   端到端用 `osascript` 往 TextEdit 发 `gggg` 读回 王。
6. **Windows 壳（1–1.5 天 + 真机）**：`[general] wubi` 读取、`dispatch/key/input.rs` 各 `push` 后的刷新处接 `take_auto_commit`、悬浮状态条方案名、设置程序下拉、`glimmer.iss` 带数据；本机只 `cargo check --target x86_64-pc-windows-gnu`。
7. **文档（0.5 天，与各步同提交）**：`docs/user/` 新页「五笔」+ `keys.md` 加五笔行；`design/architecture.md` 模块树；`crate-notes.md`；`README` 数据署名加 rime-wubi。
8. **第二阶段**：98 五笔（同工具、`[general] wubi = "98"`）；`z` 中间位通配；`encode_commit_history`（对已上屏历史造词）；双拼反查。

完成后：

```sh
git worktree remove ../glimmer-wubi
```

## 五、风险与取舍

- **码表许可**：LGPL-3.0 数据随 GPL 程序分发没有问题，但要把 LGPL 全文与 AUTHORS 署名随包（「关于」页数据署名已有清单，加一条）。
- **候选质量**：极点表词频「Add freqs from google」较老；我们的词频学习按文本共用，用户打几天就会盖过静态序。简码固定不调频是有意为之，别「优化」掉。
- **空码体验**：fcitx 缺省是清空，rime 缺省是留着；取「留着可退格、敲字母才清」，比清空少一次挫败。
- **性能**：前缀区间在排好序的键上是连续段，与拼音同一套二分；13.7 万条 `.qj` 约 3 MB，mmap 启动无感。
- **与拼音互斥**：五笔开着时双拼 / 注音配置被忽略；反查先只支持全拼，双拼用户反查放第二阶段。
