//! 拼音标注的结果：一个词一条，可带多个读音。

mod entry;
mod reading;

pub use entry::PinyinEntry;
pub use reading::PinyinReading;
