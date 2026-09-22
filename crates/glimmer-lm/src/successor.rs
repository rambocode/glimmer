use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// CSR 里的一条后继：后词编号与计数。二元与三元两层 CSR 共用这个条目（三元的「前词」是一条二元）。8 字节，原样落盘。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct Successor {
    /// 后词编号。
    pub word: u32,

    /// 这条接续的计数。
    pub count: u32,
}
