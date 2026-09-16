//! 按天一个文件的日志：文件名约定 `<前缀>.<YYYY-MM-DD>.log` 与清理旧文件。
//!
//! Server 用 tracing_appender 滚动，TSF DLL 与设置程序自己开文件追加；三者共用这里的命名与「只留最近几天」规则，
//! 日期怎么取由调用方定（DLL 用 `GetLocalTime`，不想在每个应用进程里多拉一份时区库）。

use std::path::Path;

/// 缺省保留天数，三个进程一致。
pub const KEEP_DAYS: u32 = 7;

/// `<前缀>.<YYYY-MM-DD>.log`。
pub fn file_name(prefix: &str, year: i64, month: u32, day: u32) -> String {
    format!("{prefix}.{year:04}-{month:02}-{day:02}.log")
}

/// 公历日期 → 自 1970-01-01 起的天数（Howard Hinnant 的 `days_from_civil`），够比大小用。
pub fn days_from_civil(year: i64, month: u32, day: u32) -> u32 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u32;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + i64::from(doe) - 719_468) as u32
}

/// 删掉目录里早于 `today - keep_days` 的 `<前缀>.<日期>.log`；别的文件（包括其他进程的日志）不碰。
pub fn prune(dir: &Path, prefix: &str, today: u32, keep_days: u32) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(day) = name.to_str().and_then(|name| log_day(name, prefix)) else {
            continue;
        };
        if today.saturating_sub(day) >= keep_days {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// `tsf.2026-09-11.log` → 那天的天数；不是该前缀的日志文件为 `None`。
pub fn log_day(name: &str, prefix: &str) -> Option<u32> {
    let date = name
        .strip_prefix(prefix)?
        .strip_prefix('.')?
        .strip_suffix(".log")?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_numbers_follow_the_calendar() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(
            days_from_civil(2026, 9, 11) - days_from_civil(2026, 9, 4),
            7
        );
        assert_eq!(
            days_from_civil(2026, 3, 1) - days_from_civil(2026, 2, 28),
            1
        );
    }

    #[test]
    fn only_dated_logs_of_the_prefix_are_recognised() {
        assert_eq!(
            log_day("tsf.2026-09-11.log", "tsf"),
            Some(days_from_civil(2026, 9, 11))
        );
        assert_eq!(log_day("tsf.log", "tsf"), None);
        assert_eq!(log_day("server.2026-09-11.log", "tsf"), None);
        assert_eq!(log_day("tsf.2026-13-01.log", "tsf"), None);
        assert_eq!(
            file_name("settings", 2026, 9, 12),
            "settings.2026-09-12.log"
        );
    }

    #[test]
    fn prune_keeps_recent_and_foreign_files() {
        let dir = std::env::temp_dir().join(format!("glimmer-daily-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let today = days_from_civil(2026, 9, 12);
        for name in [
            "tsf.2026-09-12.log",
            "tsf.2026-09-06.log",
            "tsf.2026-09-05.log",
            "server.2026-09-01.log",
            "notes.txt",
        ] {
            std::fs::write(dir.join(name), "").unwrap();
        }
        prune(&dir, "tsf", today, KEEP_DAYS);
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "notes.txt",
                "server.2026-09-01.log",
                "tsf.2026-09-06.log",
                "tsf.2026-09-12.log"
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
