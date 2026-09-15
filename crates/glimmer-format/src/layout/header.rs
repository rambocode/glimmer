use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 文件头魔数。沿用派生前的 "QINGJIAN"：这是 `.qj` 磁盘格式的标识，改了就读不了已生成的数据文件，与品牌名无关。
pub const MAGIC: [u8; 8] = *b"QINGJIAN";

/// 当前格式版本。布局不兼容时加一，读旧版本的代码按需保留。
pub const FORMAT_VERSION: u16 = 1;

/// 文件头，32 字节，紧跟着 `section_count` 条 [`crate::SectionEntry`]。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct Header {
    /// [`MAGIC`]。
    pub magic: [u8; 8],

    /// [`FORMAT_VERSION`]。
    pub version: u16,

    /// 数据种类，见 [`crate::Kind`]。
    pub kind: u16,

    /// 分节数。
    pub section_count: u32,

    /// 留给以后（校验和、标志位），现在全零。
    pub reserved: [u8; 16],
}

impl Header {
    pub const SIZE: usize = size_of::<Self>();
}
