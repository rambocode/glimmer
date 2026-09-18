//! 检查结果：比当前版本新的那一版。

use crate::feed::Asset;

/// 有新版本时给壳的信息：显示用的版本号、日期、更新日志，与本机该装的那个包。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Available {
    pub version: String,

    /// `YYYY-MM-DD`，可能为空。
    pub date: String,

    /// 更新日志，一条一行。
    pub notes: Vec<String>,

    /// 本机平台 / 架构对应的安装包。
    pub asset: Asset,
}

impl Available {
    /// 给状态行的一句话：「0.1.8（2026-09-20）」。
    pub fn label(&self) -> String {
        if self.date.is_empty() {
            self.version.clone()
        } else {
            format!("{}（{}）", self.version, self.date)
        }
    }
}
