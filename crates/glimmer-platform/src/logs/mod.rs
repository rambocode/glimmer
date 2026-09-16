//! 三个进程（Server / TSF DLL / 设置程序）日志的公共部分。目前只有按天分文件的命名与清理（[`daily`]），
//! 日志目录本身在 [`crate::dirs::log_dir`]。

pub mod daily;
