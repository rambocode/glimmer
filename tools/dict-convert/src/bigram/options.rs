//! `bigram` 子命令的参数。

use std::path::PathBuf;

/// 从语料统计 n-gram 的参数。
pub struct ConvertOptions {
    /// 语料文件。
    pub corpus: Vec<PathBuf>,

    /// 分词用的词库。
    pub dict: PathBuf,

    /// 短语层文件：里面的词不参与分词，统计完按成分合成计数。
    pub phrases: Vec<PathBuf>,

    /// 品牌词文件：语料里没有的词按文件给的次数写进一元表。
    pub brand: Vec<PathBuf>,

    /// 二元计数下限。
    pub min_count: u32,

    /// 最多输出多少条二元。
    pub max_bigrams: usize,

    /// 三元计数下限。二元用 3 就够，三元同样的阈值会出一个十几 G 的表，所以单独一个、定得高些。
    pub min_trigram_count: u32,

    /// 最多输出多少条三元：直接决定 `lm.qj` 多大（一条 8 字节，外加每条二元 4 字节的段偏移）。
    pub max_trigrams: usize,
}
