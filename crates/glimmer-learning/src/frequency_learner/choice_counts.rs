//! 一对 (输入串, 词) 的选择次数：按句首 / 句中分桶，外加一个不分位置的桶。

use glimmer_core::ChoicePosition;

/// 落盘时句首那一桶的位置列写什么。
pub(super) const START_TOKEN: &str = "start";

/// 落盘时句中那一桶的位置列写什么。
pub(super) const AFTER_TOKEN: &str = "after";

/// 落盘时不分位置那一桶的位置列写什么。
pub(super) const ANY_TOKEN: &str = "any";

/// 同一对 (输入串, 词) 在三个桶里的计数。
///
/// `any` 是「不分位置」的一桶，两种位置下都计入：原样上屏标记记在这里（与句首句中无关），
/// 老文件里三列的历史计数也先落在这里，加载时再按个人 n-gram 拆到前两桶（见 `FrequencyLearner::migrate_choices`）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ChoiceCounts {
    /// 句首选的次数。
    start: u32,

    /// 句中选的次数。
    after: u32,

    /// 不分位置的次数，两种位置都计入。
    any: u32,
}

impl ChoiceCounts {
    /// 某个位置那一桶（可写）。
    fn slot_mut(&mut self, position: ChoicePosition) -> &mut u32 {
        match position {
            ChoicePosition::SentenceStart => &mut self.start,
            ChoicePosition::Continuation => &mut self.after,
        }
    }

    /// 记一次这个位置的选择。
    pub(super) fn add(&mut self, position: ChoicePosition) {
        let slot = self.slot_mut(position);
        *slot = slot.saturating_add(1);
    }

    /// 记一次不分位置的选择（原样上屏标记）。
    pub(super) fn add_any(&mut self) {
        self.any = self.any.saturating_add(1);
    }

    /// 退回一次这个位置的选择。
    ///
    /// 这个位置那一桶是空的就从 `any` 里退：迁移拆不动的老计数全在 `any` 里，
    /// 不这样退的话用户撤销一次选错就等于没撤。
    pub(super) fn sub(&mut self, position: ChoicePosition) {
        let slot = self.slot_mut(position);
        if *slot > 0 {
            *slot -= 1;
        } else {
            self.any = self.any.saturating_sub(1);
        }
    }

    /// 排序用的权重：这个位置那一桶加上不分位置那一桶。
    pub(super) fn weight(&self, position: ChoicePosition) -> u32 {
        let slot = match position {
            ChoicePosition::SentenceStart => self.start,
            ChoicePosition::Continuation => self.after,
        };
        slot.saturating_add(self.any)
    }

    /// 三桶之和，不分位置地问「这对被选过几次」时用。
    pub(super) fn total(&self) -> u32 {
        self.start
            .saturating_add(self.after)
            .saturating_add(self.any)
    }

    /// 不分位置那一桶。
    pub(super) fn any(&self) -> u32 {
        self.any
    }

    /// 三桶全空（该从表里删掉了）。
    pub(super) fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// 三桶各减半（条数超上限时整体衰减）。
    pub(super) fn halve(&mut self) {
        self.start /= 2;
        self.after /= 2;
        self.any /= 2;
    }

    /// 把 `any` 里的历史计数拆成句首 `start_share` 份、其余归句中，`any` 清零。
    pub(super) fn split_any(&mut self, start_share: u32) {
        let share = start_share.min(self.any);
        self.start = self.start.saturating_add(share);
        self.after = self.after.saturating_add(self.any - share);
        self.any = 0;
    }

    /// 读到一行的位置列与次数：加进对应的桶。位置列认不出来返回 `false`（当坏行跳过）。
    ///
    /// 用加不用覆盖：同一对可能有两三行（一个位置一行），而且重复行相加比后来者覆盖更接近用户的真实次数。
    pub(super) fn add_token(&mut self, token: &str, count: u32) -> bool {
        match token {
            START_TOKEN => self.start = self.start.saturating_add(count),
            AFTER_TOKEN => self.after = self.after.saturating_add(count),
            ANY_TOKEN => self.any = self.any.saturating_add(count),
            _ => return false,
        }
        true
    }

    /// 落盘用：三桶里非零的 (次数, 位置列)，按 `start` / `after` / `any` 的顺序。
    pub(super) fn rows(&self) -> impl Iterator<Item = (u32, &'static str)> {
        [
            (self.start, START_TOKEN),
            (self.after, AFTER_TOKEN),
            (self.any, ANY_TOKEN),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_counts_in_both_positions() {
        let mut counts = ChoiceCounts::default();
        counts.add(ChoicePosition::SentenceStart);
        counts.add_any();
        assert_eq!(counts.weight(ChoicePosition::SentenceStart), 2);
        assert_eq!(counts.weight(ChoicePosition::Continuation), 1);
        assert_eq!(counts.total(), 2);
    }

    #[test]
    fn subtracting_falls_back_to_the_unpositioned_bucket() {
        let mut counts = ChoiceCounts::default();
        counts.add_any();
        counts.sub(ChoicePosition::Continuation);
        assert!(counts.is_empty());
    }

    #[test]
    fn splitting_moves_everything_out_of_any() {
        let mut counts = ChoiceCounts::default();
        for _ in 0..21 {
            counts.add_any();
        }
        counts.split_any(7);
        assert_eq!(counts.weight(ChoicePosition::SentenceStart), 7);
        assert_eq!(counts.weight(ChoicePosition::Continuation), 14);
        assert_eq!(counts.any(), 0);
    }
}
