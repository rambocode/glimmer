//! 同音节数的几种切分里挑整句按哪种读。
//!
//! 切分器只看字母：音节少、残缺少的在前，再同就前面的音节长的在前（贪心）。后一个音节是零声母时这条规则会偏向
//! 前字吞掉 n / g 的切法：`dangao` 排出 `dang ao` 在 `dan gao` 前，`henganrao` 排出 `heng an rao` 在 `hen gan rao` 前。
//! 哪种切法说得通是语言模型的判断，不是字母的，所以同形的切分（音节数、残缺数都相同）交给整句得分来挑。

use crate::engine::{Engine, segment_longest_prefix};
use crate::parser::{ParseError, Segmentation};

/// 记下的首选切分最多几条：查询的作用域与光标后的剩余拼音各一条，再留点余量。
const MAX_PREFERRED: usize = 4;

impl Engine {
    /// 在与最优切分同形（音节数、残缺音节数都相同）的切分里，把整句得分最高的挪到最前，其余保持切分器的顺序。
    ///
    /// 整句、上屏重算、纠错的原样得分、云端参考都读第一种切分，调用方在读之前调它，几处才读得一致。
    /// 转不出或带占位音节的切分不参与；得分相同时留切分器排前的（稳定）。只有一种同形切分时不做转换。
    /// `typos` 与随后那次整句转换的一致（整段纠错已生效时为假）。
    pub(crate) fn prefer_convertible(&self, segmentations: &mut [Segmentation], typos: bool) {
        let Some(first) = segmentations.first() else {
            return;
        };
        // 单音节没有整句；切分器按 (音节数, 残缺数) 排在最前，所以同形的切分连成开头的一段
        if first.syllables.len() < 2 {
            return;
        }
        let shape = (first.syllables.len(), first.incomplete_count());
        let tied = segmentations
            .iter()
            .take_while(|s| (s.syllables.len(), s.incomplete_count()) == shape)
            .count();
        if tied < 2 {
            return;
        }
        let mut best: Option<(usize, f64)> = None;
        for (index, segmentation) in segmentations[..tied].iter().enumerate() {
            let Some(conversion) = self.convert_sentence(&segmentation.patterns(), typos) else {
                continue;
            };
            if conversion.has_placeholder() {
                continue;
            }
            if best.is_none_or(|(_, score)| conversion.score > score) {
                best = Some((index, conversion.score));
            }
        }
        if let Some((index, _)) = best {
            segmentations[..=index].rotate_right(1);
        }
    }

    /// `text` 的首选切分与切不动的尾巴：切分同 `segment_longest_prefix`，同形的里按整句得分挑，与拼音行显示同一种。
    ///
    /// 光标按音节移动、按音节删用它。本轮查询（或拼音行显示光标后的剩余拼音）算过同一段就直接用，
    /// 每按一次方向键不再重转整句；没算过才算一次并记下。
    pub(crate) fn preferred_segmentation<'t>(
        &self,
        text: &'t str,
    ) -> Result<(Segmentation, &'t str), ParseError> {
        if let Some((_, segmentation, tail_len)) = self
            .preferred_segmentations
            .borrow()
            .iter()
            .find(|(typed, _, _)| typed == text)
        {
            return Ok((segmentation.clone(), &text[text.len() - tail_len..]));
        }
        let (mut segmentations, tail) = segment_longest_prefix(text)?;
        self.prefer_convertible(&mut segmentations, true);
        let first = segmentations
            .into_iter()
            .next()
            .ok_or(ParseError::NoSegmentation)?;
        self.remember_preferred(text, &first, tail.len());
        Ok((first, tail))
    }

    /// 记下 `text` 的首选切分（`tail_len` 是切不动的尾巴字节数）。同一段覆盖旧的，超过 [`MAX_PREFERRED`] 丢最早的。
    pub(crate) fn remember_preferred(
        &self,
        text: &str,
        segmentation: &Segmentation,
        tail_len: usize,
    ) {
        let mut remembered = self.preferred_segmentations.borrow_mut();
        remembered.retain(|(typed, _, _)| typed != text);
        if remembered.len() >= MAX_PREFERRED {
            remembered.remove(0);
        }
        remembered.push((text.to_owned(), segmentation.clone(), tail_len));
    }
}
