//! 整句评测按句长分的桶。

/// 各桶的字数上限（含），最后一桶收更长的全部。
const UPPER_BOUNDS: [usize; 4] = [5, 10, 15, 20];

/// 一个句长桶里的计数：句子越长整句全对越难，合在一起看会被短句的数字盖住。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LengthBucket {
    /// 评了几句。
    pub total: usize,

    /// 首选就是原句的句数。
    pub top1: usize,

    /// 整句候选逐字对上的字数。
    pub chars_correct: usize,

    pub chars_total: usize,
}

impl LengthBucket {
    /// 一共几个桶。
    pub const COUNT: usize = UPPER_BOUNDS.len() + 1;

    /// `chars` 个字的句子进第几个桶。
    pub fn index_of(chars: usize) -> usize {
        UPPER_BOUNDS
            .iter()
            .position(|&upper| chars <= upper)
            .unwrap_or(UPPER_BOUNDS.len())
    }

    /// 第 `index` 个桶的字数范围，报告里当行首。
    pub fn label(index: usize) -> String {
        let lower = index
            .checked_sub(1)
            .map_or(1, |previous| UPPER_BOUNDS[previous] + 1);
        match UPPER_BOUNDS.get(index) {
            Some(upper) => format!("{lower}-{upper} 字"),
            None => format!("{lower} 字以上"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentences_fall_into_buckets_by_length() {
        assert_eq!(LengthBucket::index_of(3), 0);
        assert_eq!(LengthBucket::index_of(5), 0);
        assert_eq!(LengthBucket::index_of(6), 1);
        assert_eq!(LengthBucket::index_of(20), 3);
        assert_eq!(LengthBucket::index_of(21), 4);
        assert_eq!(LengthBucket::label(0), "1-5 字");
        assert_eq!(LengthBucket::label(3), "16-20 字");
        assert_eq!(LengthBucket::label(4), "21 字以上");
    }
}
