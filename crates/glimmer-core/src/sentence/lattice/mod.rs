//! 词图：一段输入的各个位置之间有哪些词。Viterbi（`super::viterbi`）只认 [`Lattice`]，不管位置是什么：
//! 拼音的位置是音节（[`SyllableLattice`]）；别的输入方案换一种位置，实现同一个接口就能复用找路与打分。

mod source;
mod syllable;

pub use source::Lattice;
pub use syllable::SyllableLattice;
