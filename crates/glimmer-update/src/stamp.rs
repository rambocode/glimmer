//! 「上次什么时候查过」的标记文件：只看修改时间，内容无所谓。

use std::path::Path;
use std::time::{Duration, SystemTime};

/// 标记文件名，放在用户数据目录。
pub const STAMP_FILE: &str = "update-check";

/// 距上次检查是否已过 `interval`（自动检查用 [`CHECK_INTERVAL`](crate::CHECK_INTERVAL)）；文件不在、读不了都算到点了。
pub fn check_due(stamp: &Path, interval: Duration) -> bool {
    let Ok(modified) = std::fs::metadata(stamp).and_then(|m| m.modified()) else {
        return true;
    };
    match SystemTime::now().duration_since(modified) {
        Ok(elapsed) => elapsed >= interval,
        // 修改时间在未来（改过系统时间）：当到点，免得永远不查
        Err(_) => true,
    }
}

/// 记下「刚查过」。失败只记日志：下次多查一次而已。
pub fn touch(stamp: &Path) {
    if let Some(dir) = stamp.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(error) = std::fs::write(stamp, b"checked\n") {
        tracing::warn!(path = %stamp.display(), %error, "写检查更新标记失败");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    #[test]
    fn due_when_missing_then_not_after_touch() {
        let dir = std::env::temp_dir().join(format!("glimmer-update-stamp-{}", std::process::id()));
        let stamp = dir.join(super::STAMP_FILE);
        let _ = std::fs::remove_file(&stamp);
        assert!(super::check_due(&stamp, Duration::from_secs(60)));
        super::touch(&stamp);
        assert!(!super::check_due(&stamp, Duration::from_secs(60)));
        assert!(super::check_due(&stamp, Duration::ZERO));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
