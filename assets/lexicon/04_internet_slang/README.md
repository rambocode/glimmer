# 网络用语与聊天短语

这个目录原本是数据包留的空壳（`README.txt` 是数据包原文，没动）。现在放一份**候选词表** [candidates.tsv](candidates.tsv)：
1,294 条网络流行语、聊天口语搭配、职场与技术圈用语、电商外卖出行社交用语。

## 2026-09-22 已并入

维护者整表通过。1,294 条里词库已有的 542 条原样不动，**没有的 752 条全部并入**：汉字词 670 条拆成 [slang_words.tsv](slang_words.tsv)，
中英混杂词 82 条追加进 [../mixed_words.tsv](../mixed_words.tsv)。走的是**带语料全量重跑**（`lexicon` + `bigram` 各两遍），
不是增量 `supplement`，所以下面第 2 条限制（改不了已有词的次数）不再成立，已有词的次数都按全量语料重算了。
**第 1 条限制仍然在**：摆烂 / 避雷 / 睡会儿 的首选还是同音整句，它们稳定在第 2、3 位。
原因不是词频不够，而是语料（中文维基 + 2020 年前后的 LCCC）里根本没有这几个词，全量重跑也变不出真实的二元 / 三元证据——
要翻过来得补一份现代对话语料。
做法、数字与复现命令见 [docs/notes/internet-slang.md](../../../docs/notes/internet-slang.md)。

并入时顺带修了 `dict-convert` 的两个根因（都在 `tools/dict-convert/src/syllable.rs` 与 `lexicon/readings.rs`）：
拉丁字母 / 数字在 Unihan 里没有读音，`lexicon` 一直把含字母的词整条丢掉（旧 `dict.tsv` 里那 15 条混杂词是手工插进去的，重跑就会没）；
`嗯` 的 `kMandarin` 是 `ǹg en`，代码只取第一个值，这个字一个合法音节都剩不下，于是 嗯 / 嗯嗯 / 嗯呢 进不了词库。

## 为什么要补

语料是中文维基 + 2020 年前后的 LCCC 微博对话，网络用语与「动词 + 会 + 名词」这类口语搭配没数据。
用户报的例子：`mohuiyu` 想打 摸会鱼，首选是 末毁于 / 莫毁于；`shesi` 出 设施，`songchigan` 出 宋词干，`lagehui` 出 拉格会。

## 来源与许可

词是起草者（Claude）自己的中文互联网知识，**没有抄任何外部词表**，不引入新的许可约束。
现状列（词库有没有、语言模型多少次、合成次数）来自仓库自己的数据：`data/generated/dict.tsv`、`lm-unigram.tsv`、`lm.qj`，
以及 `assets/lexicon/` 下的 `phrases.tsv` / `common_words.tsv` / `domain_words.tsv` / `mixed_words.tsv`。
雾凇拼音（GPL）与五笔码表不当词源，这份表也没用它们。

## 表的列

`词 · 拼音 · 类别 · 说明 · 建议次数 · 词库现状`

| 列 | 说明 |
| --- | --- |
| `词` | 2–6 个字符，能整块打出来的固定说法 |
| `拼音` | 全拼，音节间一个空格，不写声调；ü 写 `v`，儿化的「儿」单独一个音节 `er`；字母每个字母算一个音节（`栓Q` = `shuan q`），与 `mixed_words.tsv` 同一写法 |
| `类别` | `slang` 网络流行语与梗 · `chat` 聊天口语搭配 · `work` 职场与技术圈 · `life` 电商外卖出行社交 · `mixed` 含拉丁字母或数字 |
| `说明` | 一句话用法，20 字以内 |
| `建议次数` | 与 `dict.tsv` 词频同一尺度，见下面「次数档位」；现状为「已有」的写 `-` |
| `词库现状` | `已有（词库 N，模型 M，来自…）` 或 `缺（合成 N）` |

全部 1,294 条的拼音都逐条校验过：音节都在 `dict.tsv` 用过的 416 个音节里（字母除外），音节数与字数一一对应。

## 收词标准

**收**：2020 年以后到现在中文互联网与日常聊天里真的常用、能整块敲出来的固定说法。判断的准绳是「这个组合是不是两个常见词拼在一起、输入法多半会切错」。

**不收**：2015 年前的老梗（除非已沉淀成普通口语）；脏话、性相关、人身攻击黑话、地域歧视、政治敏感词；
明星艺人名、作品名、品牌名、一次性热搜事件名；临时造句；单字。
擦边的贬义词（绿茶 / 舔狗 / 普信 / 恋爱脑 一类）留在表里但说明里标了「贬义」，由维护者定夺。

## 条数

| 类别 | 条数 | 其中词库没有 |
| --- | --- | --- |
| `chat` 聊天口语 | 363 | 211 |
| `slang` 网络流行语 | 329 | 219 |
| `life` 电商外卖出行社交 | 270 | 102 |
| `work` 职场与技术圈 | 250 | 124 |
| `mixed` 中英混杂 | 82 | 82 |
| 合计 | 1,294 | 738 |

556 条（43.0%）词库或语言模型已经有了，标 `已有`，默认不进（`supplement` 本来就跳过已有词）。

## 次数档位

三档固定值，对着仓库里已有的门槛定，不是拍脑袋：

| 档 | 汉字词 | 中英混杂 | 依据 | 条数（其中缺） |
| --- | --- | --- | --- | --- |
| 高 | 2000 | 8000 | 汉字词对着 `phrases.tsv` 的入选门槛（总次数与对话语料次数都 ≥ 2000）；混杂词对着 `mixed_words.tsv` 里 C盘 的 8000（压过同音的 裁判 5416） | 469（182） |
| 中 | 200 | 2000 | 汉字词对着 `common_words.tsv` 预筛 CC-CEDICT 词的 200 次门槛；混杂词取 B站 40000 与 F盘 300 之间的中间量级 | 732（480） |
| 低 | 20 | 300 | `common_words.tsv` 与 `domain_words.tsv` 的下限「不到 20 按 20」；混杂词对着 `mixed_words.tsv` 里 F盘 / G盘 / B股 的 300 | 93（76） |

`dict-convert gaps` 能按 `lm.qj` 的成分二元合成出真实语料次数的，**合成次数高于档位值时取合成次数**（与 `common_words.tsv` 的做法一致）。
1,294 条里 369 条拿到了合成次数：≥ 2000 的 10 条、200–1999 的 119 条、20–199 的 193 条、< 20 的 47 条。含字母的词 `gaps` 不处理，所以没有合成次数。

**档位值不等于「保证抢到首选」**，见下面的验证。它保证的是「能进前三、选一次个人学习就顶上去」。

## 验证（起草时做的，走增量路径，没有动 `data/`）

下面这一节是**并入之前**的验证，留着当对照。并入实际走的是全量重跑，数字见
[docs/notes/internet-slang.md](../../../docs/notes/internet-slang.md)：752 条冷查首选 353 → 548、前三 357 → 701。

走的是真实的增量接入路径，不是 `--extra-dict`：

```sh
cargo run --release -p glimmer-dict-convert -- --out-dir /tmp/supout \
  supplement --words <缺的 670 条，汉字词> --dict assets/lexicon/dict.tsv --lm data/generated/lm.qj
cargo run --release -p glimmer-cli -- --dict /tmp/supout/dict.tsv --lm /tmp/supout <拼音>…
```

词库加了 651 条（19 条已在 `dicts/` 领域词库里），语言模型多 368 个合成词、9,282 条合成二元。

拿这 670 条自己的拼音逐个冷查（去重后 668 个不同拼音）：

| | 补之前 | 补之后 |
| --- | --- | --- |
| 首选就是该词 | 321 | 523 |
| 前三里有该词 | 321 | 645 |

用户点名的例子：`mohuiyu` 末毁于 → **摸会鱼**，`mohuieryu` 莫会儿与 → **摸会儿鱼**，`shesi` 设施 → **社死**，
`xianyanbao` 鲜艳包 → **显眼包**，`songchigan` 宋词干 → **松弛感**，`lagehui` 拉格会 → **拉个会**。

### 已知的两条限制（起草时提的，并入后的结论见上面「2026-09-22 已并入」）

1. **同音的整句候选不吃词频这条杠杆。** `bailan` 的首选是整句 白兰、`bilei` 是 比勒、`shuihuier` 是 谁会儿，
   把 摆烂 / 避雷 / 睡会儿 的词频从 2000 一路抬到 50000，首选一条没变（摆烂 要到 200000 才翻过来，避雷 / 睡会儿 到 200000 也翻不过来）。
   这三个词补完稳定在第 2、3 位。要真正翻过来得靠语料里的真实二元，也就是把词并入后**带语料重跑 `bigram`**，或者补一份现代对话语料，不是这份表能解决的。
   所以高档给 2000 而不是给一个大数：大数换不来首选，只会把词频尺度搞乱。
2. **`supplement` 跳过已有词，改不了已有词的次数。** 躺平（词库 191）、内卷（80）、摸鱼（332）这类已经在词库、但次数明显偏低的词，
   这份表标了 `已有`、建议次数写 `-`。想提它们的频要走全量 `lexicon` + `bigram` 重跑。
   **这条已解决**：并入时跑的就是全量重跑，所有已有词的次数都按全量语料重算了。
   第 1 条**没解决**——带语料重跑了，摆烂 / 避雷 / 睡会儿 的首选还是同音整句，因为语料里这几个词一次都没出现过。

补完仍不在前三的 23 条，多数是同音强势词压着（麻了 / 马勒、搭子 / 大字、塌了 / 塔勒、灰度 / 恢复、回滚 / 回顾、压测 / 压车、堂食 / 唐诗、值机 / 自己、叫车 / 轿车），
建议维护者对照着决定是抬档位还是直接删掉。

## 怎么接进去（2026-09-22 实际跑的就是这一节的「有语料时」那条路）

审定的行先按类别拆成两份。**汉字词**（类别不是 `mixed`）写成 `词\t次数\t拼音` 的 `slang_words.tsv`，**含字母的**追加进 `assets/lexicon/mixed_words.tsv`：

```sh
cd assets/lexicon/04_internet_slang
awk -F'\t' 'NR>5 && $3!="mixed" && $5!="-" { print $1 "\t" $5 "\t" $2 }' candidates.tsv > slang_words.tsv
awk -F'\t' 'NR>5 && $3=="mixed" && $5!="-" { print $1 "\t" $5 "\t" $2 }' candidates.tsv >> ../mixed_words.tsv
```

**有语料时**（`data/corpus/` 在本机）全量重跑，照 [GLIMMER.md](../GLIMMER.md) 第 4c / 4e / 4f / 4g 步，只是多带一个 `--extra-words` 与一个 `--phrases`。
两条命令各跑两遍（先按现有一元表建库 → 统计 → 用新一元表再建 → 再统计），`bigram` 的三元参数照
[language-model.md](../../../docs/notes/language-model.md)（`--max-trigram-entries 160000000`、`--min-trigram-count 5`）：

```sh
cargo run --release -p glimmer-dict-convert -- lexicon \
  --pinyin data/generated/pinyin-llm.jsonl --frequency data/generated/lm-unigram.tsv \
  --extra-words assets/lexicon/mined_words.tsv --extra-words assets/lexicon/phrases.tsv \
  --extra-words assets/lexicon/brand.tsv --extra-words assets/lexicon/domain_words.tsv \
  --extra-words assets/lexicon/mixed_words.tsv --extra-words assets/lexicon/common_words.tsv \
  --extra-words assets/lexicon/04_internet_slang/slang_words.tsv
cargo run --release -p glimmer-dict-convert -- bigram \
  --phrases assets/lexicon/phrases.tsv --phrases assets/lexicon/domain_words.tsv \
  --phrases assets/lexicon/common_words.tsv --phrases assets/lexicon/04_internet_slang/slang_words.tsv \
  --brand assets/lexicon/brand.tsv --brand assets/lexicon/mixed_words.tsv data/corpus/*.txt
cp data/generated/dict.tsv assets/lexicon/dict.tsv
```

**没有语料时**走增量（2026-09-22 起 `supplement` 也收含字母的词了，与 `lexicon` 共用同一套音节判定）：

```sh
cargo run --release -p glimmer-dict-convert -- supplement --words assets/lexicon/04_internet_slang/slang_words.tsv
cp data/generated/dict.tsv assets/lexicon/dict.tsv
```

两条路都收尾于打包、重建 AI 领域词库、发数据包：

```sh
cargo run --release -p glimmer-dict-convert -- pack dict --name 微明基础词库 --license "MIT AND Unicode-3.0 AND CC-BY-SA-4.0"
cargo run --release -p glimmer-dict-convert -- pack lm \
  --name "微明语言模型（中文维基 + LCCC，微明词库分词）" --license "CC-BY-SA-4.0 AND MIT" \
  --attribution "中文维基百科（CC BY-SA 4.0）；LCCC（清华大学 CoAI，MIT）"
cargo run --release --locked -p glimmer-dict-convert -- ai --exclude data/generated/dict.tsv \
  --corpus assets/lexicon/ai/corpus.txt --corpus assets/lexicon/ai/development.txt
tools/release/data-bundle.sh
```

并入前跑一遍 `apps/cli` 的回归（排序 / 整句的改动按 [contributing](../../../docs/contributing.md) 都要先跑它），
再把做法与数字记进 `docs/notes/`，词表来源补进 [assets/lexicon/README.md](../README.md) 的 sources 一节。

## 建议维护者重点看的

- **贬义与擦边**：绿茶、舔狗、普信、恋爱脑、小镇做题家、牛马、社畜 一类。能打出来是刚需，但收进随包词库等于背书。
- **有歧义的**：职场语境的 毕业（被裁）、优化（裁员）、上岸、塌房、出道，和它们的本义撞车。
- **易过时**：绝绝子、尊嘟假嘟、栓Q 这类强时效的梗，两年后可能只剩负担；低档 20 次的 93 条基本都是这类。
- **口语写法二选一**：等会 / 等会儿、歇会 / 歇会儿、摸会鱼 / 摸会儿鱼 都收了两种，要不要都留。
- **和 `common_words.tsv` 的边界**：`life` 与 `work` 里有不少已经不算「网络用语」的普通词（下单、退款、周报），
  按目录名它们更该进 `common_words.tsv`，按来路它们是这一批挖出来的。归到哪本由维护者定。
