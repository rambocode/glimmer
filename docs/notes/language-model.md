# 静态语言模型：三元与回退平滑（2026-09-22）

整句转换的静态模型原来是「固定 λ 的词级 bigram」：P(w|v) = 0.8·c(v,w)/c(v) + 0.2·c(w)/N，没有折扣、回退比例与上文无关。
这一版换成**带绝对折扣回退的词级 trigram**。这篇记尺子、做法、数字、试过不行的和怎么复现。
诊断的来龙去脉在 [long-sentence.md](long-sentence.md)，模型实现要点在 [crate-notes.md](crate-notes.md) 的 `crates/glimmer-lm` 段。

## 尺子

与 long-sentence.md 同一套，句子集重新冻结过（`/tmp/glimmer-eval/sentences.tsv`，9808 句）：

- 干净集：`--eval-text`，冷启动，按句长分桶。
- 分段输入：同一份句子 `--eval-chunk 2`。
- 注错集：同一份句子 `--eval-typos`。
- 真实日志：`--replay`（2026-09-22 冻结，词 9575 条），只看词级命中与「现在仍纠」。
- 开本地模型：干净集加 `--neural data/model/model.qjm`。
- 验收（todo ③ 的原话）：每格候选 `SPAN_CANDIDATES` 6 → 12 **不该变差**。打分模型够可信时放宽搜索不会找到更好的错路径。

## 做法

（数字表与取舍见下面各节，填完整后见「结果」。）
