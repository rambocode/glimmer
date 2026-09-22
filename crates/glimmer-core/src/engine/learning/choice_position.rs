//! 「同一输入串下选过几次」按上文位置分桶的位置标记。

use crate::sentence::Context;

/// 一次选择发生在句首还是句中。选择次数按这两档分开记、分开查（见 [`super::Learner::record_choice`]）。
///
/// 同一个 `ba`：句首用户要的是 把，接在「做了」后面要的是 吧。两处的次数混在一个桶里数，
/// 排序第 5 级只看总数，谁先攒够次数谁就在两处都排第一，第 6 级的上下文再也没机会说话。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ChoicePosition {
    /// 句首：上文里没有前一个词（上屏链是空的，应用前文也切不出词）。
    /// 也是缺省值：还没上屏过任何词时就在句首。
    #[default]
    SentenceStart,

    /// 句中：前面有词。
    Continuation,
}

impl ChoicePosition {
    /// 按上文判位置：没有前一个词就是句首。
    pub fn of(context: Context<'_>) -> Self {
        if context.previous.is_none() {
            Self::SentenceStart
        } else {
            Self::Continuation
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_the_context_without_a_previous_word() {
        assert_eq!(
            ChoicePosition::of(Context::START),
            ChoicePosition::SentenceStart
        );
        assert_eq!(
            ChoicePosition::of(Context::after("做了")),
            ChoicePosition::Continuation
        );
        assert_eq!(
            ChoicePosition::of(Context::after_two("我", "做了")),
            ChoicePosition::Continuation
        );
    }
}
