//! 词图：一段输入的各个位置之间有哪些词。Viterbi（`super::viterbi`）只认 [`Lattice`]，不管位置是什么：
//! 拼音的位置是音节（[`SyllableLattice`]），五笔的位置是编码的字母（[`CodeLattice`]）。

mod code;
mod source;
mod syllable;

pub use code::{CodeLattice, TAIL_PREFIX_PENALTY};
pub use source::Lattice;
pub use syllable::SyllableLattice;
