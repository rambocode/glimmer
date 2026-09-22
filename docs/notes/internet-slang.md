# 网络用语与聊天短语并入词库（2026-09-22）

[候选表](../../assets/lexicon/04_internet_slang/candidates.tsv) 1,294 条维护者整表通过，词库没有的 752 条全部并入，
并带语料全量重跑了词库与语言模型。这篇记做法、条数、评测前后、顺带修掉的两个根因与复现命令。
候选表怎么起草的见 [目录说明](../../assets/lexicon/04_internet_slang/README.md)，词库生成全流程见 [GLIMMER.md](../../assets/lexicon/GLIMMER.md)，
三元模型的参数见 [language-model.md](language-model.md)。

结论先说：**候选表自己的冷查首选 353 → 548 条、前三 357 → 701 条（共 752 条），
干净集与回放都没掉**（干净集 29.7% / 72.7% → 29.7% / 72.8%，回放词 8880 → 8885）。
上一次增量 `supplement` 翻不过来的 摆烂 / 避雷 / 睡会儿 这次**还是没翻过来**，原因见最后一节。

## 条数

候选表 1,294 条里 542 条词库已有（原样不动），752 条没有：

| | 条数 |
|---|---|
| 汉字词 → `assets/lexicon/04_internet_slang/slang_words.tsv` | 670 |
| 中英混杂词 → 追加进 `assets/lexicon/mixed_words.tsv`（原有 15 条 → 97 条） | 82 |

候选表 README 的表里写「缺 738」，是只数了「词库现状」列以 `缺（` 开头的行；另有 14 条写成「缺词库，模型有」，
词库里同样没有，也要并。拆表的 awk 按「建议次数不是 `-`」取，出来就是 670 + 82 = 752。

## 做法：全量重跑，不走增量

上一次验证走的是增量 `supplement`，README 记下了两条限制：同音的整句候选不吃词频这条杠杆（把 摆烂 的词频抬到 5 万首选也不变），
`supplement` 跳过已有词、改不了 躺平 / 内卷 / 摸鱼 这类偏低的次数。两条都只有**带语料全量重跑**能解，所以这次跑的是
`lexicon` + `bigram` 各两遍（GLIMMER.md 第 4 步的「两遍」：先按现有一元表建词库 → 统计语料 → 用新一元表再建一遍 → 再统计一次）。

### 补回两个前置

- `data/unihan/Unihan_Readings.txt` 本机没有，从 unicode.org 重新下（Unicode License v3）。**下到的是比建库当时新的一版**，
  `桲` / `饹` 两个字的读音顺序因此变了（见下面「词库 diff」）。
- `data/generated/pinyin-llm.jsonl`（LLM 标的多音字词读音）本机没有，**数据 Release 里也没有**：
  `data-bundle.sh` 只在文件存在时才打 `glimmer-llm-intermediates.tar.gz`，data-v1 到 data-v4 都没有这个附件。
  重跑一遍 `gloss-gen pinyin` 要 7 万多次 LLM 调用，于是**从已发布的词库里把标注反推回来**：
  `lexicon --emit-ambiguous` 出 72,352 个待标注的多音字词，逐个在 `assets/lexicon/dict.tsv`（基础词库）
  与旧的 `dicts/*.qj`（11 本领域词库，导出成 TSV）里取词频最高的那条读音写成 JSONL。
  「取词频最高」正好还原标注：标注与常用词表不一致时，`lexicon` 把标注读音按全额词频写、原表读音降到八分之一。
  72,352 个里恢复了 72,351 个，重跑时 `rejected_annotations = 0`（每条都过了 Unihan 逐字校验）。

**这份 `pinyin-llm.jsonl` 要跟着下一版数据包一起发**，否则下次全量重跑还得再反推一次。

### 命令

```bash
# 0. 拆表（README「怎么接进去」的两条 awk）
cd assets/lexicon/04_internet_slang
awk -F'\t' 'NR>5 && $3!="mixed" && $5!="-" { print $1 "\t" $5 "\t" $2 }' candidates.tsv > slang_words.tsv   # 再手工补表头注释
awk -F'\t' 'NR>5 && $3=="mixed" && $5!="-" { print $1 "\t" $5 "\t" $2 }' candidates.tsv >> ../mixed_words.tsv

# 1. 反推 LLM 标注（没有 pinyin-llm.jsonl 时）
cargo run --release -p glimmer-dict-convert -- --out-dir /tmp/lex-pass1 lexicon \
  --unihan data/unihan/Unihan_Readings.txt --emit-ambiguous /tmp/lex-pass1/lexicon-ambiguous.txt <下面的 --extra-words>
# 逐词在 assets/lexicon/dict.tsv 与旧 dicts/*.qj 里取词频最高的读音，写成 {"word":…,"pinyin":[…]} 的 JSONL

# 2. lexicon 第一遍（用现有的 lm-unigram.tsv）
cargo run --release -p glimmer-dict-convert -- lexicon \
  --unihan data/unihan/Unihan_Readings.txt --pinyin data/generated/pinyin-llm.jsonl \
  --frequency data/generated/lm-unigram.tsv \
  --extra-words assets/lexicon/mined_words.tsv --extra-words assets/lexicon/phrases.tsv \
  --extra-words assets/lexicon/brand.tsv --extra-words assets/lexicon/domain_words.tsv \
  --extra-words assets/lexicon/mixed_words.tsv --extra-words assets/lexicon/common_words.tsv \
  --extra-words assets/lexicon/04_internet_slang/slang_words.tsv

# 3. bigram 第一遍（全量语料，三元）
cargo run --release -p glimmer-dict-convert -- --out-dir /tmp/bigram-pass1 bigram \
  --dict data/generated/dict.tsv \
  --phrases assets/lexicon/phrases.tsv --phrases assets/lexicon/domain_words.tsv \
  --phrases assets/lexicon/common_words.tsv --phrases assets/lexicon/04_internet_slang/slang_words.tsv \
  --brand assets/lexicon/brand.tsv --brand assets/lexicon/mixed_words.tsv \
  --max-trigram-entries 160000000 --min-trigram-count 5 data/corpus/zhwiki.txt data/corpus/lccc.txt

# 4. lexicon 第二遍（--frequency 换成第一遍统计出的一元表），再 5. bigram 第二遍（同第 3 步，去掉 --out-dir）

# 6. 打包：三元按随包定稿的阈值 25 过一遍再打
cargo run --release -p glimmer-dict-convert -- pack dict --name 微明基础词库 --license "MIT AND Unicode-3.0 AND CC-BY-SA-4.0"
awk -F'\t' 'NR==1 || $4>=25' data/generated/lm-trigram.tsv > /tmp/lm-min25/lm-trigram.tsv
cargo run --release -p glimmer-dict-convert -- pack lm \
  --input data/generated/lm-unigram.tsv data/generated/lm-bigram.tsv /tmp/lm-min25/lm-trigram.tsv \
  --name "微明语言模型（中文维基 + LCCC 全量，三元，微明词库分词）" --license "CC-BY-SA-4.0 AND MIT" \
  --attribution "中文维基百科（CC BY-SA 4.0）；LCCC（清华大学 CoAI，MIT）"
cargo run --release -p glimmer-dict-convert -- ai --exclude data/generated/dict.qj \
  --corpus assets/lexicon/ai/corpus.txt --corpus assets/lexicon/ai/development.txt
cp data/generated/dict.tsv assets/lexicon/dict.tsv
```

## 顺带修掉的两个根因

并入过程里发现含字母的词与 `嗯` 根本进不了词库，都是 `dict-convert` 的问题，不是词表的问题。

**含拉丁字母 / 数字的词一直被整条丢掉。** `lexicon` 的读音全来自 Unihan，`C` / `0` 在 Unihan 里没有条目，
于是「读音推不出来」→ `dropped`。GLIMMER.md 第 4e 步写的「`mixed_words.tsv` 与 phrases / domain_words 一样走 `lexicon --extra-words`」
从来没成立过：旧 `dict.tsv` 里那 15 条混杂词是提交 bb5c54b 手工插进文件的，只要有人全量重跑 `lexicon` 就会全部消失。
`supplement` 同样卡在 `is_syllable`，README 里「混杂词只能等下一次全量重跑」说的就是这条。
修法是新建 `tools/dict-convert/src/syllable.rs`：拉丁字母 / 数字自成一个音节，读音就是它的小写形式，
两条并入路径共用同一个判定。**放行只对非汉字生效**——单个字母对汉字放行会出事，`儿` 的 Unihan 读音里有儿化的 `r`，
放进去会顶掉 `er` 成主读音。给定读音则以词表为准（`U盘` 读 `you pan` 而不是 `u pan`）。

**`嗯` 不在词库里。** Unihan 给 `嗯` 的 kHanyuPinlu / kXHC1983 全是 `ń` / `ňg` / `ǹg` 这类不是合法拼音音节的读音，
唯一能用的 `en` 藏在 `kMandarin: ǹg en` 的第二个值里，而解析只取了第一个（`value.split_whitespace().next()`）。
于是这个字一个合法音节都不剩、整条丢掉，`嗯` / `嗯嗯` / `嗯呢` 都打不出来。改成取全部值、按顺序当次要读音：
`all()` 里 kMandarin 排在 kHanyuPinlu 之后且去重，多出来的读音只会追加在末尾，不会改变任何字的主读音。

## 词库 diff

对着 `data/generated-backup-data-v4/`（data-v4 那一版）比：

| | 旧 | 新 |
|---|---|---|
| `dict.tsv` 不同词形 | 93,276 | 105,694 |
| `dict.qj` 词条 | 94,182 | 106,623 |
| `dict.qj` 大小 | 3.6 MB | 4.2 MB |
| `dicts/*.tsv` 领域词库 | 只有 `ai.tsv` | 12 本，129,419 条 |

**一个词都没删。** 新增 12,439 个词形，拆开看：

| 来源 | 条数 |
|---|---|
| `slang_words.tsv`（其余 11 条落在 `dicts/ai.tsv` 与 `it_computing.tsv` 里） | 659 |
| `mixed_words.tsv`（15 条原有 + 82 条新增，这次才真的由 `lexicon` 并进来） | 82 |
| 从领域词库升进基础词库 | 11,712 |
| `嗯` | 1 |

那 11,712 条是 `--domain-keep-min 50`（语料里 ≥ 50 次的领域词留在基础词库）的连带结果：
随包那份词库是用**只统计了约两成维基**的一元表建的，很多领域词次数不够、被拆了出去；这次用全量语料的一元表，它们够了。
反方向有 21 个词从基础词库落回领域词库（三医院 / 九中 / 华科 / 图吧 这类），在 `dicts/places.tsv`、`dicts/it_computing.tsv` 里，没丢。

**主读音只变了 3 个词**，都不是这次改动引起的：

| 词 | 旧 | 新 | 为什么 |
|---|---|---|---|
| 桲 | po | bo | 新下的 Unihan 比建库当时多列了读音 |
| 饹 | le | ge | 同上 |
| 书归正传 | shu gui zheng zhuan | shu gui zheng chuan | 语料里没有这个词，两条读音都落到底值 1、并列，`max` 取谁都行 |

词频整体按全量语料重算，**倍率中位数 2.07**（词频 > 1000 的 20,267 个词）：长江 5,378 → 24,330、中国 193,762 → 971,256、
你好 71,960 → 73,446（对话语料本来就统计全了，所以基本不动）。这是语料从两成维基换成全量维基的功劳，不是这次补词的。

## 语言模型

`dicts/*.tsv` 回来之后分词词表不再缺领域词，[language-model.md](language-model.md)「已知偏差」那一节记的问题解决了：

| | 旧（data-v4 随包） | 新 |
|---|---|---|
| 分词词表 | `dict.tsv` + `dicts/ai.tsv`，101,449 词 | `dict.tsv` + 12 本领域词库，232,291 词 |
| 模型词表 | 94,880 | 168,157 |
| 二元 | 5,645,898 | 5,646,636 |
| 三元（统计，`--min-trigram-count 5`） | 9,681,684 | 9,572,503 |
| 三元（随包，阈值 25） | 1,627,692 | 1,603,945 |
| `lm.qj` | 84.8 MB | 88.3 MB |

三元条数略降是因为词表变大、同样的语料被切成更长的词，三元上下文更稀疏。`lm.qj` 仍在 150 MB 预算内。

## 评测

同一份冻结句子集（9,808 句）与输入日志，旧数据用 `--dict data/generated-backup-data-v4/dict.qj --lm …/lm.qj` 跑。
两边都不带 `--extra-dict`（CLI 不自动加载 `dicts/`），可比。

| 尺子 | 旧 | 新 |
|---|---|---|
| 干净集 首选 / 字 | 29.7% / 72.7% | **29.7% / 72.8%** |
| 　1–5 字（4177 句） | 40.8% / 70.2% | 40.7% / 70.4% |
| 　6–10 字（4042 句） | 24.6% / 73.0% | 24.7% / 73.2% |
| 　11–15 字（1311 句） | 14.0% / 73.6% | 13.7% / 73.5% |
| 　16–20 字（278 句） | 11.5% / 76.9% | 11.5% / 76.7% |
| 分段输入（2 词）首选 / 字 | 54.8% / 75.1% | 54.4% / 75.0% |
| 注错集 首选 / 字 | 16.0% / 31.2% | 16.1% / 31.3% |
| 开本地模型 首选 / 字 | 41.3% / 79.8% | **41.8% / 79.9%** |
| 回放 词命中 | 8880 / 9575 | **8885 / 9575** |
| 回放 词「不在候选」 | 69 | **55** |
| 回放 整句命中 | 263 / 474 | 266 / 474 |
| 回放 英文命中 | 79 / 88 | 79 / 88 |
| 回放 当时纠错生效、现在仍纠 | 120 / 121 | 120 / 121 |

分段输入那一栏句数从 22,970 变成 22,831：分段是按语言模型切词切的，词表变了句数就变，两个数字不严格可比。
其余几把尺子句数一样。

**候选表自己的冷查**（752 条各用自己的拼音，冷启动）：

| 类别 | 条数 | 旧 首选 / 前三 | 新 首选 / 前三 |
|---|---|---|---|
| `chat` 聊天口语 | 211 | — | 195 / 210 |
| `slang` 网络流行语 | 219 | — | 145 / 212 |
| `work` 职场与技术圈 | 138 | — | 100 / 131 |
| `life` 电商外卖出行社交 | 102 | — | 69 / 95 |
| `mixed` 中英混杂 | 82 | — | 39 / 53 |
| 合计 | 752 | 353 / 357 | **548 / 701** |

上一次走增量 `supplement` 时是 523 / 645（那次只算 670 条汉字词，含不了混杂词）。

**用户点名的例子**：

| 输入 | 旧首选 | 新首选 |
|---|---|---|
| `mohuiyu` | 末毁于 | **摸会鱼** |
| `mohuieryu` | 莫会儿与 | **摸会儿鱼** |
| `shesi` | 设施 | **社死** |
| `xianyanbao` | 鲜艳包 | **显眼包** |
| `songchigan` | 宋词干 | **松弛感** |
| `lagehui` | 拉格会 | **拉个会** |
| `bailan` | 白兰 | 白兰（摆烂 第 2） |
| `bilei` | 比勒 | 比勒（避雷 第 3） |
| `shuihuier` | 谁会儿 | 谁会儿（睡会儿 第 2） |

**词库没坏**：`changjiang` `yinhang` `chongqing` `nihao` `zhongguo` `shijian` `gongsi` `wenti` `gongzuo` `xuesheng`
`dianhua` `pengyou` `shenme` `xianzai` `keyi` `kaifa` `jisuanji` `beijing` `xiexie` `womenquxianwanba` 共 20 条常用输入，
首选**一条没变**（`womenquxianwanba` 新旧都出 我们去先玩吧，是本来就有的问题）。

## 遗留

- **摆烂 / 避雷 / 睡会儿 的首选还是同音整句。** 候选表 README 把这条记成「要靠语料里的真实二元，也就是带语料重跑」，
  这次带语料重跑了，**没翻过来**：语料是中文维基加 2020 年前后的 LCCC，这几个词在里面出现次数是零，
  重跑变不出不存在的证据。真要翻得补一份现代对话语料。三个词都稳定在第 2、3 位，选一次个人学习就顶上去。
- **混杂词冷查最弱**（82 条里首选 39、前三 53）。`PUA` / `CP` / `GG` / `diss` / `MBTI` / `KPI` / `TODO` / `ROI` 这些
  输入码就是英文单词本身，中文模式下被英文词候选压着。这是英文词表与混杂词共用输入码的结构问题，不是次数能解决的。
- **`P0` / `P1` 的输入码里有数字**（`p 0` / `p 1`）。词库与 `.qj` 都收下了，但数字键在输入法里通常是选词键，
  实际能不能敲出来要在壳里验，这次没验。
- **`pinyin-llm.jsonl` 是反推出来的**，与当初 LLM 标的那一份不保证逐条相同（重跑时 Unihan 校验全过、
  主读音只差上面那 3 个词，可用）。它现在在 `data/generated/`，下一版 `data-bundle.sh` 会打进
  `glimmer-llm-intermediates.tar.gz`（1.0 MB），**记得发出去**，否则下次全量重跑还要再反推。
- 数据包本身没发：`tools/release/data-bundle.sh --pack` 能打出 `glimmer-data.tar.gz` 58 MB、
  `glimmer-llm-intermediates.tar.gz` 1.0 MB、`model.qjm` 53 MB，上传由维护者自己做。
