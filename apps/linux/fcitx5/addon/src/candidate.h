// 候选页：Router 管翻页，fcitx5 只看到当前页；面板的翻页 / 移动高亮 / 点选都转回会话。
#ifndef GLIMMER_FCITX5_CANDIDATE_H
#define GLIMMER_FCITX5_CANDIDATE_H

#include <cstddef>
#include <memory>
#include <vector>

#include <fcitx/candidatelist.h>
#include <fcitx/text.h>

#include "glimmer_fcitx5.h"

namespace glimmer {

class GlimmerState;

// 一个候选：点选时让会话按对应数字键。
class GlimmerCandidateWord final : public fcitx::CandidateWord {
public:
    // 文字（旧版 fcitx5 里拼上译词）与页内下标。
    GlimmerCandidateWord(GlimmerState *state, int index, const char *text, const char *comment);

    // 点选：会话重画候选页会销毁本对象，调用之后不再碰成员。
    void select(fcitx::InputContext *inputContext) const override;

private:
    // 所属会话。
    GlimmerState *state_;

    // 页内下标。
    int index_;
};

// 当前页候选表。
class GlimmerCandidateList final : public fcitx::CandidateList,
                                   public fcitx::PageableCandidateList,
                                   public fcitx::CursorMovableCandidateList {
public:
    // 从第 index 条候选指令读出本页。
    GlimmerCandidateList(GlimmerState *state, const GlimmerOutputs *outputs, size_t index);

    // 数字标签。
    const fcitx::Text &label(int idx) const override;

    // 第 idx 个候选。
    const fcitx::CandidateWord &candidate(int idx) const override;

    // 本页候选数。
    int size() const override { return static_cast<int>(words_.size()); }

    // 高亮下标。
    int cursorIndex() const override { return cursor_; }

    // 竖排 / 横排。
    fcitx::CandidateLayoutHint layoutHint() const override { return layout_; }

    // 有上一页。
    bool hasPrev() const override { return hasPrev_; }

    // 有下一页。
    bool hasNext() const override { return hasNext_; }

    // 上一页（会话重画会销毁本对象，之后不碰成员）。
    void prev() override;

    // 下一页（同上）。
    void next() override;

    // 翻过页就一直显示翻页按钮。
    bool usedNextBefore() const override { return true; }

    // 高亮上移。
    void prevCandidate() override;

    // 高亮下移。
    void nextCandidate() override;

private:
    // 下标越界时收到最近的有效值，避免 fcitx5 传坏下标时越界访问。
    size_t clamp(int idx) const;

    // 所属会话。
    GlimmerState *state_;

    // 标签。
    std::vector<fcitx::Text> labels_;

    // 候选。
    std::vector<std::unique_ptr<GlimmerCandidateWord>> words_;

    // 高亮下标。
    int cursor_ = 0;

    // 排布。
    fcitx::CandidateLayoutHint layout_ = fcitx::CandidateLayoutHint::NotSet;

    // 有上一页。
    bool hasPrev_ = false;

    // 有下一页。
    bool hasNext_ = false;
};

} // namespace glimmer

#endif
