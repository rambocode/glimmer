# 各 crate 的实现要点

CLAUDE.md 只保留目录地图与规则，每个 crate / app / tool 的实现细节收在这里：入口类型、数据文件、常数、生成命令。
改了实现要同步改这里；与代码冲突时以代码为准。

## crates/glimmer-dictionary

词库（TSV 解析或 `.qj` mmap），键按字节序排好，查询逐音节位置二分收窄（简拼位置按音节块跳扫），
`lookup_pattern`（≥ 模式长度）与 `lookup_exact`（正好等长）同一套实现。词库键以 `v` 表示 ü，
TSV 解析、查询与生成工具把 `lue` / `nue` 统一成 `lve` / `nve`。
旧 `.qj` 含这些键时，加载器建立规范化的内存词库。新 `.qj` 继续使用 mmap。

## crates/glimmer-core

模块：`composition` / `parser` / `correction`（拼写纠错：整段一处编辑的候选纠正 + `typo` 音节级敲错变体表，后者进整句词图当带代价的边）/
`candidate` / `ranking` / `shortcut` / `sentence` / `fuzzy` / `shuangpin`（双拼：四套方案键位表、键 → 全拼解码与消耗换算）/ `zhuyin`（大千注音：键 → 注音符号 → 拼音，`[general] zhuyin` 开关，声调只判音节完整不进查询）/
`wubi`（五笔码表方案，`Engine::set_wubi(Option<Scheme>)`，开着时双拼 / 注音 / 整句 / 纠错 / 模糊音 / 中英混输 / 快捷候选 / 组句联想全部让位，`v` `u` 是编码键，只剩 `?` 问字；
  `Variant`（`Wubi86` / `Wubi98` / `Xinshiji`，`variant.rs`）是版本：配置写法 `86` / `98` / `xsj`（新世纪另认 `06` / `xinshiji`）、方案键 `wubi86` / `wubi98` / `wubixsj`、
  码表文件 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`、界面名；三版行为完全一致，只有字根与码表不同；
  `Scheme` = `Variant` + 码表 `Dictionary`（`词\t编码\t词频`，编码整个是一个音节）+ `Reverse` 字 → 全码（单字取最长码、等长取高频）+ `Options`（`[wubi]`）+ `encode` 造词规则；
  查询在 `engine/query/wubi.rs`：`lookup_pattern` 前缀一次查出，全码命中在前、前缀命中短码在前当逐键提示（`hint` 开着 `reading` 注完整编码），作用域 ≤ `fixed_order_length`（缺省 2）的全码命中只按码表静态词频、不叠用户权重与选择记录；
  `WubiHit::key` 的末项是**码表存放顺序**（`assemble` 已按词频降序、平手保 TSV 原序排好）而不是文本序：补出来的名次词频归一化后 log 概率差在千分位以下、取整会打平，按文本序排会把冷词顶到前面；
  `check_wubi_auto_commit` 在 `push` 之后置 `pending_auto_commit`（四码全码命中且 `auto_select`；新键接上后无任何命中 → 旧段首选顶字、新键存 `deferred_key` 等 `commit` 后补回；满四码空码再敲字母整段丢掉），
  壳每键 `take_auto_commit` 取到就走普通 `commit`；`z` 开头走 `query_pinyin` 反查，候选 `reading` 注 `code_of`、`syllables` 是整段作用域，`typed_display` 为 `z'zhong'guo`；
  上屏消耗在 `engine/commit/wubi.rs`：吃候选编码那么长，自动造词按 `encode` 出编码进用户词（造不出就不造），五笔下连着上屏两次即造（同缓冲区阈值），空码回车 `record_raw` 不学成英文词；
  `scheme_key()` 为 `Variant::key()`（`wubi86` / `wubi98` / `wubixsj`），输入日志与回放据此切方案）/ `emoji` /
`english`（英文模式候选）/ `engine`（`query::EnglishTail`：句末英文词并入整句，`woxiangxuehaorust` → 我想学好rust，尾段也像拼音时按分数与拼音读法比）。
`Engine` 是对外唯一门面，`Translator` / `Learner` trait 在 `engine` 模块；词库是「主词库 + 附加词库（`set_extra_dictionaries`，同步抽取 `@` 编码的英文名称供补全）+ 用户词」的列表；繁体输出（`traditional` 开关与 `traditional_map` 映射）依赖 `ferrous-opencc`（`s2tw`）在出候选与上屏边界转换，内部保持简体。
`Engine` 是对外唯一门面，`Translator` / `Learner` trait 在 `engine` 模块；词库是「主词库 + 附加词库（`set_extra_dictionaries`，同步抽取 `@` 编码的英文名称供补全）+ 用户词」的列表。
- 中英混输的英文词位置：`Engine::set_chinese_first`（配置 `[general] chinese_first`，缺省关）关着时拼音不像话的输入英文排第一（`extras::insert_english`，
  用户老选中文词时仍让中文在前），开着时整句先插、英文词紧随其后排第二（`query_inner` 里两步的先后按开关掉转）；句末英文词并入整句（`EnglishTail`）不受它影响。
  缺省关是回放定的（9241 词 / 269 条英文上屏：缺省开英文首选 82.5% → 7.1%）。
- 整句读哪种切分：`parser::segment` 只按字母排（音节少、残缺少，再同就前面的音节长），零声母边界上会偏向前字吞 n / g（`dangao` 排出 `dang ao`）。
  `Engine::prefer_convertible`（`engine/query/segmentation.rs`）在与第一种同形（音节数、残缺数相同）的切分里按整句得分挑最高的挪到最前；
  查询（在查词之后、插整句之前）、上屏重算 `sentence_words` / `mixed_words`、中英混输比分、纠错的原样得分、云联想的拼音与本地参考都先调它，几处读的是同一种切分。
  只有一种同形切分时不做转换。光标按音节移动 / 按音节删与光标后剩余拼音的显示走 `Engine::preferred_segmentation`（同一种挑法）；
  查询与剩余拼音显示把挑出的切分记在 `preferred_segmentations`（最多 4 条，与 `span_cache` 一起清），方向键直接复用，不重转整句。
- 两个候选开关都在 Core 生效、缺省开：`Engine::set_mixed_english`（配置 `[general] mixed_english_candidates`）关掉时 `insert_english` 直接返回（精确词与补全都不出，句末英文词 `EnglishTail` 照旧）；
  `Engine::set_emoji_candidates`（配置 `[general] emoji_candidates`）关掉时 `insert_emoji` 直接返回，emoji 表照常加载，热重载改开关不用重建 Engine。
- `custom_phrase::merge_replacements` 把平台给的「输入码 → 短语」表（macOS 系统文本替换）并进配置里的自定义短语：每条占该码最靠前的空位（1–9），
  输入码不是小写字母、已有同码同文本、九位都满的跳过；Core 不管数据从哪来。

`EngineSession` 保存可挂起的组句、标点、历史与学习链，`Engine::swap_session` 在同一个引擎里交换输入状态，共用词库与落盘服务。切换上下文时清除查询及异步预测缓存，并由平台恢复各自私密状态。

`Engine::discard_input` / `EngineSession::discard_input` 用于隐私能力变化时无痕清理输入，包括透传缓冲、学习链和暂存词汇曝光；`set_private` 只切换写入开关，保留已输入的组句。

## crates/glimmer-translate

`Glossary`，本地 TSV 释义表（词性 + 译文）；`LevelTable`，词汇等级表（`assets/levels/levels-{en,ja}.tsv`，CEFR A1–C2 / JLPT N5–N1，
`uv run tools/corpus/levels.py` 从 `data/levels/` 的原始 CSV 生成，来源与许可见 `assets/levels/README.md`），「统计」页按级数词汇用，不进候选。

## crates/glimmer-learning

- `FrequencyLearner`：用户选择次数（`user.tsv`）、按输入串记的选择（`user-choices.tsv`，词级排序里同输入串选过的优先）、用户词（`user-words.tsv`，主词库同格式，
  Engine 与主词库一起查）、个人英文词（`user-english.tsv`，回车原样上屏的英文词与选过的英文候选，与随包英文词表一起出候选且在前）、
  个人敲错表（`user-typos.tsv`，接受过的 (敲的, 要的) 音节对，词图敲错边与整段纠错的代价按它打折）与个人 n-gram（`user-ngram.tsv`，Core `sentence::UserNgram`，
  二元 + 三元在线计数，整句转换与词级排序里与静态模型插值；Tab 接受的云端整句按 `sentence::segment_text` 切词后也记；
  连着选出的两个词记够次数自动造词进用户词，一段拼音分几次选完的合成词记两次也造）。
  按输入串记的三张表（`user-words.tsv` / `user-choices.tsv` / `user-typos.tsv`）按方案分目录：`from_path_with_scheme(path, Some("wubi98"))` 把它们放到词频文件同目录的 `wubi98/` 下
  （三个五笔版本各一个子目录：`wubi86/` / `wubi98/` / `wubixsj/`，互不共用）
  （五笔编码与拼音音节撞键，`a` 既是音节也是 工 的简码），按文本记的 `user.tsv` / `user-ngram.tsv` / `user-english.tsv` 各方案共用；子目录首次落盘时建。
- `InputLog`：输入日志（`input-log.jsonl`，每次上屏一行：敲的键、切分、看到的前几个候选、选了第几个、来源、纠错、撤销，
  Core `InputLogger` trait 的落盘实现，`[general] input_log` 缺省开，只写本机，给离线回归评测与个人模型用）。
- `UsageStats`：输入统计（`usage.tsv`，按天记汉字 / 中文词 / 英文词 / 上屏次数，Core `UsageMeter` trait 的实现，Engine 每次上屏 `Usage::of_text` + 按来源定词数，
  整句按 `segment_text` 切词数；与输入日志无关，偏好设置「统计」页显示，`book_scale` 折成几本《某书》）。
- `VocabularyBook`：词汇记录（`user-vocab.tsv`，Core `VocabularyTracker` trait 的实现：学习语言的每条译词看到过几轮 / 上屏过 / ⌥+数字 打出过几次；Core 私密输入统一跳过曝光和提交写入，但仍可读取已有记录用于排序和生词标记；
  Engine `annotate` 据此填 `Sense::fresh`，看到轮次不到 `FRESH_UNTIL` = 3 的译词壳里画橙色；「看到」按上屏那一刻屏幕上那一页算，壳每次画完 `Engine::note_displayed` 告知当前页）。
- 各表落盘走 Core `storage::write_atomic`（临时文件 + fsync + 改名），加载按行容错（坏行警告跳过，真读不了壳退回内存学习），
  壳激活期间每 60 秒 `Engine::flush_learning`；IMK 回调边界 `imk::catch_panic` 拦 panic、缓冲区字母原样上屏（见 architecture.md「崩溃不丢」）。

## crates/glimmer-predict

- `CloudPredictor`：`Predictor` trait 的网络实现（async-openai，OpenAI 兼容接口，默认 DeepSeek），后台线程防抖 / 缓存 / 超时，`submit` / `poll` 非阻塞。
  `PredictConfig` 是配置的 `[predict]` 分节。只在组句中联想，一次请求给云端词（容错校验后补进候选第一页末尾 `[predict] slots` 格，缺省 2，不预留不占位，
  前面的本地候选不挪；排布在 Core `CandidateLayout`）和整句补全（preedit 右侧，Tab）；上屏后不联想，本地历史不进请求。
- `CloudGlossFiller`：释义兜底（Core `GlossFiller` trait，与 Predictor 分开的线程与通道，攒 1.5 秒 / 8 个词发一次，问过不再问）：
  随包释义表没有的词库词 / 云端词上屏后入队，结果壳每秒 `Engine::poll_glosses` 经 `Translator::learn` 写进 `glimmer-translate::PersonalGlossary`
  （`user-glossary-<语言>.tsv`，`LayeredTranslator` 个人表优先）；随云联想开关一起开。
- 问字键（缺省 `u`）开头是问字模式（`PredictionKind::Question`，答案带读音、不校验拼音），`?` 开头要 `ModeKeys::question_mark` 开着才算（配置 `[shortcut] question_mark`，缺省关，壳用 `Engine::takes_question_mark` 决定空缓冲区的 `?` 是入口还是标点）；`PredictionKind::Translate` 是壳里快捷键触发的「翻译选中文字」
  （双向：汉字为主译成学习语言，外文译回中文，`prediction::translation_target`），译文走结果的 `sentence`。

## crates/glimmer-format

`.qj` 数据容器（`Container` mmap 读、`Writer` 写、`Table<T>` / `Text` 零拷贝视图、`hash` 可落盘哈希索引、`Metadata` 名称 / 许可证 / 署名）。
词库与语言模型都能 `write_qj` / 从 `.qj` 打开，启动 50 ms；`cargo run --release -p glimmer-dict-convert -- pack dict|lm --name … --license …`
生成 `data/generated/{dict,lm}.qj`，`bundle.sh` 在 TSV 更新时自动重打并只把 `.qj` 打进包。设计见 `docs/design/architecture.md`「数据文件：`.qj` 容器」。

## crates/glimmer-neural

`CharScorer`，Core `sentence::SentenceScorer` trait 的实现：candle 加载字级 Transformer（GPT-2 风格 decoder，训练仓库（本地 `../train`，私有，不在本仓库）导出的
`model.safetensors` + `config.json` + `vocab.json`），给「前文 + 整句」按字累加 log 概率；前文的每层 K / V 缓存（`PrefixCache`），
同一段前文只算一次，每个候选只算自己那几个字（64 字前文 × 8 条 28 ms，Metal）。features `accelerate` / `metal` 换后端，壳用 `metal`。

Engine 侧在 `engine/rescoring/`：接了打分器就取 Viterbi 前 `RESCORE_PATHS` = 6 条路径按 `路径分 + λ·(神经分 − 静态二元分)` 重排（λ `NEURAL_WEIGHT` 0.5，
个人 n-gram / 用户加分 / 代价不动），分走「前文 + 文本 → 神经分」缓存 `NeuralCache`；同步打分器（`with_sentence_scorer`，CLI 评测）当场补分，
异步的（`with_async_sentence_scorer`，后台线程 `RescoreWorker`）查询不等模型：缺分的记下来，壳停键后 `request_rescoring`、`poll_rescoring` 到了再 `query` 一次。
前文优先用壳给的应用光标前文（`set_rescoring_context`），没有用本会话最近 64 个上屏字符。CLI `--neural <导出目录>`（`--neural-weight` / `--neural-context` / `--neural-async`）。

## crates/glimmer-lm

`BigramModel`，Core `sentence::LanguageModel` trait 的实现，从 `data/generated/lm.qj`（或 `lm-unigram.tsv` / `lm-bigram.tsv`）加载
（没有这两个文件就退化为一元词频整句）。数据由 `tools/corpus/parquet_to_text.py`（uv 脚本，HF parquet → 简体纯文本）加
`cargo run --release -p glimmer-dict-convert -- bigram --phrases assets/lexicon/phrases.tsv --phrases assets/lexicon/domain_words.tsv --brand assets/lexicon/brand.tsv --brand assets/lexicon/mixed_words.tsv data/corpus/*.txt` 生成；语料在 `data/corpus/`（gitignore）。
短语层不当 token 统计（分词时摘掉、统计完按成分合成一元 / 二元，短语得分等于原来两个词的路径，见 `bigram.rs` 模块注释），品牌词按给定次数写进一元与句首二元。

## crates/glimmer-platform

`Config`（TOML 配置文件，`[general]` / `[shortcut]` / `[fuzzy]` / `[dictionaries]` / `[apps]` / `[predict]` 分节，首次运行写模板，
`set_value` 用 toml_edit 原地改键保留注释；`[model] enabled` 本地整句模型开关，`LocalModelConfig`）；`extra_dictionaries` 列出 / 加载随包领域词库与用户 `dicts/`
（mac 壳与 Windows Server 共用，同名 `.qj` 优先于 `.tsv`）；`protocol` 模块是 Windows Server ↔ TSF DLL 的 IPC 协议类型
（`ClientMessage` / `ServerMessage` / `Frame` / `PreeditSegment`，全 serde，两端共用，见 `docs/design/architecture.md`「Windows：TSF」）。

## crates/glimmer-server

输入法 Server 的平台无关部分，从 `apps/windows/server` 抽出，Windows Server 进程与 Linux IBus 引擎进程共用：`assembly`（按 `AssemblySpec` 装配 Engine：词库 / 释义 / 学习 / 语言模型 / 五笔码表——`WubiSpec` 给版本 + `[wubi]` 选项，`load_scheme` 按 `Variant::data_file()`
在主词库同目录找 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`，学习器按 `Variant::key()` 分子目录；配置热加载在 `dispatch/reload/wubi.rs`，开关或换版本才重开码表）、
`dispatch::Router`（`ClientMessage` → Engine → `ServerMessage` / `Frame`：多会话、按键、上屏、翻译选中文字、配置热加载、本地整句模型重排的节拍 `next_tick` / `tick`）、`error::ServerError`。
不含传输与绘制：候选窗 / 状态条经 `CandidateSink` / `StatusSink` 由平台壳注入，默认不画。键码沿用 Windows 虚拟键码（VK）语义，Linux 端把 keysym 翻成 VK 再发。
输入日志的会话条目（版本号 / 平台名）由壳用 `Router::set_log_identity` 设置。集成测试 `tests/engine_loop.rs` / `wubi_loop.rs` 用 `assets/sample/` 样例数据，全平台可跑。

## crates/glimmer-render

自绘渲染器：候选窗一帧 + 主题 → 预乘 RGBA 位图，tiny-skia 栅格 + cosmic-text 文字（fontdb 按平台清单只加载几个字体文件、不扫系统），
自己解析 `trak` 字距表、按主题 gamma 加深笔画；cosmic-text 打了 `opsz` 光学字号补丁（qingjian-team/cosmic-text 分支 `qingjian-opsz`，workspace `[patch.crates-io]` 钉 rev）。
`examples/preview.rs` 出 PNG 与真机截图并排比、`--measure` 与 AppKit 对宽度。mac 壳 `candidates/bitmap/` 贴位图，`[general] renderer = "system"` 切回 AppKit 绘制
（过渡期退路，偏好设置「候选窗口」页可选）；`[general] font` 是候选窗字族名（空为系统字体，`bitmap/font_files.rs` 用 CoreText 按字族名找文件只加载那几个，没装就回系统字体；
设置页 `preferences/font_picker/` 是搜索框 + 列表）；`[general] font_size` 是候选词字号（12–28，缺省 16 = `DEFAULT_TEXT_SIZE`），`Theme::with_text_size` 让译文与序号字号、三者行高同比缩放（行高向上取整）。设计与验收见 `docs/design/rendering.md`。

## apps/cli

测试工具，`cargo run -p glimmer-cli -- kaifa`。

- `--predict` 强制开云联想并等结果打印，交互模式下上屏后也联想。
- `--typing` 逐键计时（性能测试用 release 构建跑，目标每键 10 ms 以内）。
- `--chinese-first` 打开中文优先（`[general] chinese_first = true` 的排法），配合 `--replay` 比两种英文词位置。
- `--replay <input-log.jsonl>` 回放评测：把日志里每次上屏的键重新喂给引擎，按来源算首选 / 前五命中率、平均名次、不在候选的条数，打印没命中的例子（`--misses N`）；
  只在内存里学习不写文件，加 `--user-dict` 可带上现有学习数据。日志每条带 `scheme`，回放按它切双拼 / 注音 / 五笔：五笔条目要同时给 `--wubi`，没给就跳过并计数（`wubi_missing`）。
  一次回放只装得下一种五笔版本（`wubi_slot` 是单槽位：拼音条目时把码表卸到槽里、五笔条目再装回来），日志里其他五笔版本的条目一律计入 `wubi_missing`；
  混了多种版本的日志要按版本各跑一遍。
- `--wubi 86|98|xsj|off` 覆盖 `[general] wubi`（`xsj` 即新世纪，也可写 `06`）：码表只认 `data/generated/` 下该版本的 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`
  （没有就报错，用 `dict-convert wubi` + `pack dict --output` 生成），`[wubi]` 选项照配置；
  `--user-dict` 给了时按输入串记的表落该版本的方案子目录（`wubi86/` / `wubi98/` / `wubixsj/`）。`--typing` 逐键计时不模拟四码自动上屏（`set_input` 不走 `push`）。
- `--tune 名=值`（逗号分隔）覆盖个人 n-gram 插值与敲错代价的常数扫网格（名字见 `apps/cli/src/tuning.rs`，Core 侧是 `Engine::set_interpolation` / `set_typo_costs`，壳只用缺省值）。
- `--eval-text <文本>...` 整句评测：把用户自己写的中文文本按标点切句、按词库读音转成全拼，冷启动喂给引擎看整句能不能还原原句
  （首选命中率 / 字准确率 / 查询耗时；不依赖日志里当时选了什么，给整句排序与语言模型的改动当尺子），`--eval-save` 冻结成 `句子\t拼音\t上文` 三列文件，
  之后直接 `--eval-text` 它保证比的是同一份句子（本机的在 `data/eval/sentences.tsv`）。排序、整句、纠错的改动先跑它们再合。

## apps/macos

IMK 输入法，源码按 `app / host / imk / candidates / menubar / preferences` 分目录。

- 输入法菜单（状态项 + 系统输入源菜单）与偏好设置窗口都是配置文件的前端：只写 `config.toml`，`Host::apply_config` 一条通路热加载，激活期间每秒看一次文件 mtime。
- `apps/macos/scripts/bundle.sh --install` 打包安装到 `~/Library/Input Methods/`（开发用），`--pkg` 做分发用的 pkg（装 `/Library/Input Methods/`，postinstall 跑 `glimmer-macos --register`
  注册、启用并切成当前输入源；签名 / 公证靠 `GLIMMER_SIGN_IDENTITY` / `GLIMMER_INSTALLER_IDENTITY` / `GLIMMER_NOTARY_PROFILE`，没设就 ad-hoc；`GLIMMER_TARGET` 指定架构，
  成品 `target/pkg/Glimmer-<版本>-<arm64|x86_64>.pkg`）；`scripts/uninstall.sh` 卸载。
- 日志在 `~/Library/Logs/Glimmer/`（按天分文件留 7 天，删了会重建），用户数据与配置在 `~/Library/Application Support/Glimmer/`。
- 配置项：云联想 `[predict]`（偏好设置「云服务」页有「测试连接」按钮：`glimmer_predict::ConnectionTest` 起线程发一条最小请求，`Host` 用独立定时器 `CloudTestMonitor` 轮询结果显示到窗口底部；
  `reasoning_effort` 缺省 `none`，DeepSeek V4 默认思考，不关正文为空）；模糊音 `[fuzzy]` 默认都关；`[general]` 学习语言（`off` 不显示译文）/ 每页候选数 / 翻页键 / 外观 / 竖排横排 / 拼音显示位置 /
  英文模式候选开关 / 中文优先 `chinese_first` / 中文模式英文词候选 `mixed_english_candidates` / emoji 候选 `emoji_candidates` / 双拼方案 `shuangpin`（小鹤 / 自然码 / 微软 / 搜狗 / 智能ABC，空为全拼）/ 五笔 `wubi`（空为拼音，`86` / `98` / `xsj`，开着时双拼与注音被忽略；行为在 `[wubi]`：`auto_select` / `hint` / `fixed_order_length`，
  码表 `Resources/wubi86.qj` / `wubi98.qj` / `wubixsj.qj` 三份都随包，按版本装一份，偏好设置「通用」页的「五笔」弹出菜单四项（关 / 86 / 98 / 新世纪）由 `Variant::ALL` 出，
  壳每次 `push` 后先 `take_auto_commit`）/ 日志级别 `log_level`（缺省 info 不含敲的内容，debug 逐键记，热切换）/ 输入日志 `input_log`；
  `[shortcut]` 模式键 v / u、`question_mark`（缺省关，开了空缓冲区敲 `?` 进问字）、上屏第一 / 第二个译词的修饰键 `translation` / `translation_second`、删候选 `delete_candidate`（缺省 shift，用户词整删、词库词清学习）、翻译选中文字 `translate_selection`、macOS 中英文切换 `mode_switch`（缺省 `shift` 单击，也支持旧版修饰键加字母，不能与翻译键冲突）；
  `[apps] english_candidates_off` 按 bundle identifier 列出英文模式不给候选的应用（缺省终端 / 编辑器 / IDE，`*` 前缀匹配）；
  `[dictionaries] domains` 打开随包的领域词库（`Resources/dicts/` 12 本（含 AI 与软件开发），缺省只开 `idioms`），`disabled` 关掉用户目录 `dicts/` 里的某本导入词库；
  偏好设置「词库」页随包的可开关、导入的可开关 / 移除，可导入 TSV / Rime yaml / .qj。
- 系统文本替换（系统设置「键盘 → 文本替换」）：`host/config/text_replacements.rs` 从 `NSUserDefaults` 全局域读 `NSUserDictionaryReplacementItems`
  （每条 `{ on, replace, with }`），激活输入法时重读，变了就经 Core `merge_replacements` 并进配置里的自定义短语再 `set_custom_phrases`；
  `[general] system_text_replacements` 开关（缺省开，「自定义短语」页勾选框），内容可能含证件号、地址，日志只记条数。
- 输入法进程由 launchd 拉起，看不到 shell 的环境变量：密钥写进配置同目录的 `.env`（`GLIMMER_API_KEY=...`，输入法启动时 dotenvy 读入）或 `config.toml` 的 `api_key`。
- 本地整句模型：`bundle.sh` 把 `data/model/`（或 `GLIMMER_MODEL_DIR`）三件套打进 `Resources/model/`，用户目录 `model/` 优先；`host/model/mod.rs` 在后台线程加载并预热（首次 Metal 编译）后
  `set_async_sentence_scorer` 接上，`refresh` 每键先读应用光标前 64 字给 Engine 当前文、查询后 `schedule_rescoring`，`RescoreMonitor` 停键 80 ms 请求、20 ms 轮询，
  结果到了重查一次只重画当前页（翻过页 / 动过高亮不动）；「云服务」页有开关（`[model] enabled`）。
- 端到端验证可用 `osascript` 的 System Events 往 TextEdit 发按键再读回文本（终端需要辅助功能权限；输入法得在中文模式）。
- 中英文切换的状态、事件优先级与验证边界见 [macOS 中英文切换](../design/macos-mode-switch.md)。
- macOS 安装后 `repair-input-cache.sh` 定向备份微信 / 企业微信中不含微明的旧键盘缓存；已包含微明则不动。工具随包放在 `Resources/`，不退出应用；已运行的应用需重启。见 [输入源缓存](input-source-cache.md)。

## apps/windows

一个产品两个 package：`server`（Server 进程：命名管道传输 + 自绘候选窗与悬浮状态条；Engine 装配与协议分派在 `crates/glimmer-server`）与 `tsf`（TSF 文本服务 DLL，lib 名固定 `glimmer_tsf`），
外加 `settings`（WinUI 3 设置程序）与 `installer`（Inno Setup）。不合成一个 crate，因为 DLL 不能带 Engine 的依赖树，见 `apps/windows/README.md`；
协议类型在 `glimmer-platform::protocol`，设计见 `docs/design/architecture.md`「Windows：TSF」。

TSF 原有数字 / OEM 标点 / 空格键码按当前布局用 `ToUnicodeEx` 解析（bit 2 避免改变键盘状态），
仅接受单个非代理项 UTF-16 单元。字母、小键盘和 AltGr 处理不变，不保证组合音符输入。

词库导入（设置「词库」页）走 `glimmer-dictionary::import` 转成 `.qj`（空词库拒绝），多选批量、成功的从 `[dictionaries] disabled` 摘掉、页面显示每个文件的结果；
Server 每次轮询比对用户 `dicts\` 的路径 / mtime / 长度快照，配置没变也重载新增、同名更新与移除；配置解析失败时词库沿用上次有效的开关（#36）。

## apps/linux

IBus 引擎进程 `glimmer-ibus`（package `glimmer-linux`），设计见 `docs/design/architecture.md`「Linux：IBus / Fcitx」。

- 分层：`ibus/`（zbus 连私有总线、导出 Factory / Engine、IBus 对象的变体编码）→ `frontend/`（与 D-Bus 无关的会话状态机，`Frame` → preedit / 候选表）→ `backend/`（`Backend` trait：`RouterBackend` 是真后端，`EchoBackend` 只验链路）；`key/` 是 keysym → VK 与单击 Shift 判定；`startup.rs` 是 XDG 路径、配置模板与日志。
- 启动：`glimmer-ibus --ibus`（`--echo` 换回显后端）；日志写 stderr 与 `~/.local/share/glimmer/logs/glimmer-ibus.<日期>.log`，zbus 压到 warn。
- 常数：组句中 `Poll` 间隔 60 ms；空闲时学习数据 60 s 落盘一次。
- 打包：`scripts/package.sh`（Linux 上）/ `scripts/package-docker.sh`（macOS 上，`GLIMMER_DATA_DIR` 指向有产品数据的 `data/`）出 `target/deb/Glimmer-<版本>-<arch>.deb`；装机布局与版本号规则见 `docs/notes/release.md`。
- 测试：`cargo test -p glimmer-linux`（keysym、变体签名、会话状态机）；`tests/docker/run.sh`（回显后端 + 真 ibus-daemon）；`tests/docker/install.sh <deb>`（干净 Ubuntu 装包、真 Router 打字）。
- `fcitx5/`：Fcitx5 插件。`src/`（package `glimmer-fcitx5`，staticlib + rlib）是 C 接口：`global.rs`（进程级后端 / 日志 / `glimmer_tick`）、`session/`（会话与事件函数，转给 `frontend::Session`）、`outputs/`（指令串与访问器，preedit 光标换字节偏移、候选补数字标签）；
  接口声明 `include/glimmer_fcitx5.h`，改一边要改另一边。`addon/`（CMake，`src/{engine,state,candidate,action}.cpp`）出 `glimmer.so` 与两份 conf。常数：轮询 60 ms。
  构建：`cargo build --release --locked -p glimmer-fcitx5` → `cmake -S apps/linux/fcitx5/addon -B <build> -DCMAKE_INSTALL_PREFIX=/usr -DGLIMMER_RUST_LIB=<target>/release/libglimmer_fcitx5.a -DGLIMMER_DATA_ROOT=/usr/lib/glimmer` → `cmake --build` → `DESTDIR=<暂存> cmake --install`；
  Ubuntu 24.04 装到 `/usr/lib/<multiarch>/fcitx5/glimmer.so`、`/usr/share/fcitx5/addon/glimmer.conf`、`/usr/share/fcitx5/inputmethod/glimmer.conf`。运行时 `GLIMMER_DATA_ROOT` 覆盖资源根、`GLIMMER_ECHO=1` 用回显后端；日志 `~/.local/share/glimmer/logs/glimmer-fcitx5.<日期>.log`。
  测试：`cargo test -p glimmer-fcitx5`（回显后端驱动 C 接口、空指针）；`fcitx5/tests/docker/run.sh`（真 fcitx5 + 样例词库）；`e2e.py` 不设 `GLIMMER_FCITX5_STAGE` 时是装机模式（系统插件 + `/usr/lib/glimmer`），装 deb 后 `dbus-run-session -- python3 e2e.py`。

## assets

- `assets/sample/`：手写样例词库与释义表，不是产品数据。
- `assets/emoji/emoji-zh.tsv` / `emoji-en.tsv`：Unicode CLDR 中文 / 英文 annotations 转出的 emoji 表（Unicode License v3，可发布；中文词与英文词各配 emoji，两张表加载时合成一张），
  `cargo run --release -p glimmer-dict-convert -- --out-dir assets/emoji emoji --language zh data/cldr/annotations-zh.json data/cldr/annotationsDerived-zh.json`（en 同理）。
- 英文词表词频：`uv run tools/corpus/english_frequency.py data/generated/english.tsv -o data/generated/english-frequency.tsv`，再 `... english <词表> --frequency <那个文件>`。
- `assets/wubi/`：三份五笔码表的上游原文与许可，一个版本一个子目录，各有 README 记来源 URL、提交 / 取回日期、SHA256 与规模：
  - `wubi86/`：rime-wubi 的 `wubi86.dict.yaml`（提交 `152a0d3`，2023-10-25，YAML 头 `version: "0.7"`）+ `LICENSE.LGPL-3.0` + `AUTHORS`，LGPL-3.0。
  - `wubi98/`：yanhuacuo/98wubi 的含词表 `wubi98_ci.dict.yaml` + `LICENSE.LGPL-3.0`，LGPL-3.0，≈ 11.3 万条（单字 3.6 万 + 词 7.6 万）。
    不用同作者 `98wubi-tables` 的 `wubi98_U.dict.yaml`（Unlicense）：那份是纯单字表，没有词组，多字词只能逐字上屏。
  - `wubixsj/`：GuoBinyong/wubixinshiji 的 `wubixinshiji.dict.yaml` + `AUTHORS`（AUTHORS 声明 LGPL），上游是王永民《五笔字型大一统》的新世纪整合词库，≈ 11.2 万行。
  产品数据 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj` 都由 `dict-convert wubi` 转出（见下），逐版本的命令写在各子目录 README 里。

## tools/gloss-gen

用 LLM 批量生成释义表：`cargo run --release -p glimmer-gloss-gen -- generate`（密钥读 `GLIMMER_API_KEY`，结果 JSONL 在 `data/generated/`，不进 git、可续跑，`--limit 80` 试跑）
再 `... export`（写 `glossary-{en,ja}.tsv`，产品数据在 `assets/glossary/`，见那里的 README；格式 `词\t词性. 译词[|假名]`）。CLI 与 bundle.sh 用的就是这两个文件。

## tools/dict-convert

产品数据的生成工具，输出到 `data/generated/`（gitignore）。

- `ai`：AI 与软件开发领域词库，从 `assets/lexicon/ai/{terms,names}.tsv` 和 `domains/*.tsv` 校验生成 `dicts/ai.tsv` / `ai.qj`、`ai-audit.tsv`、`ai-coverage.tsv`；8,279 源条目，主词库去重后 8,179 条。`--exclude` 只排除必选主词库，`--corpus` 同时给 `corpus.txt` 和 `development.txt`，人工权重与真实次数分列，英文按标识符边界计数。`alias` 支持 `C++/@cpp`，`mixed` 支持 `Git分支/@gitfenzhi`，构建拒绝同码不同词。Core 统一开关，英文补全个人选择优先、领域次之；配置 `domains` 仍用 `ai`。见 [设计](../design/ai-dictionary.md)。

- `lexicon`：从 `assets/lexicon/`（自建词库源：规范字 + 常用词 + THUOCL 领域词）加 Unihan 读音（`data/unihan/Unihan_Readings.txt`）、LLM 多音字标注（`gloss-gen pinyin`，
  结果 `data/generated/pinyin-llm.jsonl`，不进 git）、语料词频（`lm-unigram.tsv`）建基础词库 `dict.tsv`（8.7 万条），并把 THUOCL 领域词按语料次数 < 50 拆成
  `dicts/<领域>.tsv` + `.qj`（11 本、13 万条，`--domain-keep-min`），流程见 `assets/lexicon/GLIMMER.md`；`--extra-words` 并入人工挑的领域词 `assets/lexicon/domain_words.tsv`。
- `english`：转 `assets/lexicon/05_english/00_all_words.tsv`；`cedict`：释义表备用来源。
- `bigram`：统计语料；`--phrases` 给短语层、`--brand` 给品牌词（`assets/lexicon/brand.tsv`，微明 210）与中英混杂词（`mixed_words.tsv`，C盘 / B站：合成计数要成分词在语料里，C 不是 token，只能直接给一元，次数对着同音竞争词定），领域词也走合成计数（语料里只有几十次的词当 token 统计会吸走成分词的二元证据）。
- `mine`：从语料挖词库没收的高频词并过滤（`oov_filter.rs`：虚词规则 + 相邻字对 PMI≥3，`--candidates` 只重过滤）。
- `phrases`：挖短语层（两遍扫语料：相邻两词、两段二元都够频的相邻三词，总次数与对话语料次数都 ≥ 2000 + 边界规则，读音由成分词拼出；我的 / 不知道 / 有没有 这类常用词表不收的组合，
  `assets/lexicon/phrases.tsv`；词库已并入过短语时重跑加 `--refresh`）。
- `wubi`：Rime 四列五笔码表 → 微明 TSV `词\t编码\t词频`（编码整个当一个音节）。86 / 98 / 新世纪同一种格式、同一个子命令。
  解析 YAML 头的列序（`text` / `code` / `weight[` / `stem]`）与表体；跳过编码含 `z`（符号表，`z` 留给反查）、超四码或含非小写字母的行；
  缺省按常用字集过滤（GB2312 一二级汉字 ∪ `--charset` 主词库里出现过的字，词的每个字都在集合里才留），`--extended` 全留；同（词，码）合并取最大词频，按（编码，词频降序，词）排序；
  顺带建单字全码表（每字取最长码），有 `stem` 列时与它核对，不一致只记 warn。86 原表 136,239 条 → 73,647 条，单字 8,236、有四码全码的 7,335。
  **一半以上的行没有 `weight` 列时补词频**（`apply_in_code_order_weights`，日志记 `in_code_order_weights=true`）：同一编码下按出现顺序编名次 r（从 0 起），
  词频 = `1_000_000 / (r + 1)`、最小 1，组与组之间不可比。新世纪那份上游 99% 的行只有 `词\t编码` 两列，词频全 0 时候选顺序跟上游排好的常用度完全无关。
  为什么是名次倒数而不是整表线性递减，两条缺一不可：Core 的分数是 `ln(词频 / 总频)` 再取整到千分位，相邻只差 1 的词频取整后完全打平、等于没补；
  上游文件除开头 25 行一级简码外整体按编码排序，「文件顺序 = 常用度」只在同一编码组内成立，跨组比大小没有意义。86 与 98 都有词频，一行不动。
  **`--out` 必填**（三个版本的 TSV 同在 `data/generated/`，给缺省值会互相覆盖）。生成（`<版本>` = `wubi86` / `wubi98` / `wubixsj`）：
  ```sh
  cargo run --release -p glimmer-dict-convert -- wubi --from rime assets/wubi/<版本>/<上游>.dict.yaml \
    --out data/generated/<版本>.tsv --charset data/generated/dict.qj
  cargo run --release -p glimmer-dict-convert -- pack dict --input data/generated/<版本>.tsv --output <版本>.qj \
    --name … --license … --attribution … --source … --data-version …
  ```
  每个版本的完整命令（含上游文件名与 META 的许可 / 署名 / 来源 / 版本）写在 `assets/wubi/<版本>/README.md` 里，改数据时改那里。
- `pack dict|lm|glossary`：打 `.qj`（释义表也进容器）；`--output` 改输出文件名（五笔码表打成 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`，缺省按种类 `dict.qj` / `lm.qj` / `glossary-<语言>.qj` / `model.qjm`）。
