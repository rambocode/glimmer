//! 检查更新与下载安装包：起线程、轮询、把结果套到「关于」页与菜单版本行；安装交给系统（`open` 打开 pkg 走 Installer.app）。
//!
//! 自动检查按 `[update] check` 与标记文件走（每 12 小时最多一次，见 [`Host::auto_update_check`]）：启动 30 秒后一次，之后进程开着每 12 小时一次；
//! 「关于」页与菜单的「检查更新…」是手动查。网络都在 `glimmer-update` 的线程里，主线程只起线程、按定时器轮询通道，
//! 一次也不等：GitHub 连不上、很慢的用户（连接 10 秒、整体 20 秒超时）最多是后台线程多活一会儿，打字不受影响。

mod monitor;
mod state;

pub(super) use monitor::UpdateMonitor;
pub(super) use state::UpdateState;

use std::path::{Path, PathBuf};

use glimmer_update::{
    Available, CHECK_INTERVAL, Download, STAMP_FILE, Target, UpdateCheck, UpdateError, check_due,
    touch,
};

use super::Host;
use super::diagnostics::open_with_system;
use crate::app::paths;

/// 下载的安装包放在用户数据目录下的这个子目录。
const DOWNLOAD_DIR: &str = "updates";

/// 启动后过这么多秒才起自动检查：输入法多半是用户切过来正要打字时被拉起的，头几秒留给加载与首次按键。
const AUTO_CHECK_DELAY: f64 = 30.0;

impl Host {
    /// 启动时排自动检查：[`AUTO_CHECK_DELAY`] 秒后第一次，之后每 [`CHECK_INTERVAL`] 一次；到没到点各次自己判断。
    pub(super) fn schedule_auto_update_check(&mut self) {
        self.update.monitor.schedule_auto(AUTO_CHECK_DELAY);
        self.update
            .monitor
            .schedule_periodic(CHECK_INTERVAL.as_secs_f64());
    }

    /// 定时器来的自动检查：开关关着、或距上次检查不到 [`CHECK_INTERVAL`] 就不查。
    pub(super) fn auto_update_check(&mut self) {
        if !self.settings.config().update.check {
            return;
        }
        let due = stamp_path().is_none_or(|stamp| check_due(&stamp, CHECK_INTERVAL));
        if !due {
            tracing::debug!("距上次检查更新不到 12 小时，跳过");
            return;
        }
        self.start_update_check(false);
    }

    /// 起一次检查。`manual` 是用户点的：过程与「已是最新」都要在「关于」页说一声；自动查只在有新版本时才出声。
    pub(super) fn start_update_check(&mut self, manual: bool) {
        if self.update.check.is_some() {
            return;
        }
        self.update.manual = manual;
        let Some(target) = Target::current() else {
            self.set_update_text("这个平台 / 架构没有发布安装包，请到官网下载", false);
            return;
        };
        match UpdateCheck::start(&self.settings.config().update, &self.version, target) {
            Ok(check) => {
                self.update.check = Some(check);
                self.update.monitor.start();
                if manual {
                    self.set_update_text("正在检查更新…", self.update.available.is_some());
                }
            }
            Err(error) => {
                self.set_update_text(&format!("检查更新没能开始：{}", describe(&error)), false);
            }
        }
    }

    /// 轮询定时器每 0.2 秒来一次：检查结果到了就套用，下载中就刷进度，都没有在跑了就停表。
    pub fn poll_update(&mut self) {
        if let Some(check) = &self.update.check
            && let Some(outcome) = check.poll()
        {
            self.update.check = None;
            self.apply_check(outcome);
        }
        if let Some(download) = &self.update.download {
            match download.poll() {
                Some(outcome) => {
                    self.update.download = None;
                    self.apply_download(outcome);
                }
                None => {
                    let (received, total) = download.progress();
                    self.set_update_text(&progress_text(received, total), false);
                }
            }
        }
        if self.update.check.is_none() && self.update.download.is_none() {
            self.update.monitor.stop();
        }
    }

    /// 检查结果：记标记、更新菜单版本行与「关于」页。
    fn apply_check(&mut self, outcome: Result<Option<Available>, UpdateError>) {
        match outcome {
            Ok(available) => {
                if let Some(stamp) = stamp_path() {
                    touch(&stamp);
                }
                self.update.downloaded = None;
                self.menu
                    .set_update(available.as_ref().map(|a| a.version.as_str()));
                match available {
                    Some(update) => {
                        let text = available_text(&update);
                        self.update.available = Some(update);
                        self.set_update_text(&text, true);
                    }
                    None => {
                        self.update.available = None;
                        if self.update.manual {
                            self.set_update_text(
                                &format!("已是最新版本（{}）", self.version),
                                false,
                            );
                        }
                    }
                }
            }
            Err(error) => {
                if self.update.manual {
                    self.set_update_text(&format!("检查更新失败：{}", describe(&error)), false);
                }
            }
        }
    }

    /// 「下载并安装」：已经下好就直接打开安装器，否则起下载。
    pub(super) fn start_update_download(&mut self) {
        if let Some(path) = self.update.downloaded.clone() {
            self.open_installer(&path);
            return;
        }
        if self.update.download.is_some() {
            return;
        }
        let Some(update) = self.update.available.clone() else {
            self.set_update_text("先点「检查更新」", false);
            return;
        };
        let Some(dir) = download_dir() else {
            self.set_update_text("找不到用户数据目录，没法下载", true);
            return;
        };
        match Download::start(&update.asset, &dir) {
            Ok(download) => {
                self.update.download = Some(download);
                self.update.monitor.start();
                self.set_update_text(&format!("正在下载 {}…", update.asset.file), false);
            }
            Err(error) => {
                self.set_update_text(&format!("下载没能开始：{}", describe(&error)), true);
            }
        }
    }

    /// 下载结果：校验通过就打开安装器；失败把按钮放回去让用户重试。
    fn apply_download(&mut self, outcome: Result<PathBuf, UpdateError>) {
        match outcome {
            Ok(path) => {
                self.update.downloaded = Some(path.clone());
                self.open_installer(&path);
            }
            Err(error) => {
                self.set_update_text(&format!("下载失败：{}", describe(&error)), true);
            }
        }
    }

    /// 交给系统打开 pkg（Installer.app），安装过程由它接管；输入法进程随后会被新版本替换。
    fn open_installer(&mut self, path: &Path) {
        open_with_system(&[&path.to_string_lossy()]);
        self.set_update_text(
            &format!(
                "安装包已下载并通过校验，已打开安装器：{}。安装完成后请注销再登录，或重新选择输入法",
                path.display()
            ),
            true,
        );
    }

    /// 「关于」页那三行字。
    fn set_update_text(&self, text: &str, installable: bool) {
        self.preferences.set_update(text, installable);
    }
}

/// 「有新版本 0.1.8（2026-09-20）：第一条更新日志…」。
fn available_text(update: &Available) -> String {
    let mut text = format!("有新版本 {}", update.label());
    if let Some(first) = update.notes.first() {
        text.push('：');
        text.push_str(first);
        if update.notes.len() > 1 {
            text.push_str(&format!("（还有 {} 条，见官网）", update.notes.len() - 1));
        }
    }
    text
}

/// 「正在下载 12.3 / 35.0 MB」；清单没写体积就只显示已收到的。
fn progress_text(received: u64, total: u64) -> String {
    let mb = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
    if total == 0 {
        format!("正在下载 {:.1} MB…", mb(received))
    } else {
        format!("正在下载 {:.1} / {:.1} MB…", mb(received), mb(total))
    }
}

/// 检查更新的错误换成给用户看的中文；网络库的原话保留，方便排查。
fn describe(error: &UpdateError) -> String {
    match error {
        UpdateError::BadCurrentVersion(version) => format!("本机版本号「{version}」认不出来"),
        UpdateError::Http(error) if error.is_timeout() => {
            "连接超时。检查网络或代理：输入法进程不读终端里的代理变量".to_owned()
        }
        UpdateError::Http(error) => format!("网络请求失败：{error}"),
        UpdateError::Feed(error) => format!("版本清单读不懂：{error}"),
        UpdateError::Io(error) => format!("读写文件失败：{error}"),
        UpdateError::MissingChecksum(file) => format!("清单里没有 {file} 的校验值，不敢自动安装"),
        UpdateError::Checksum { file, .. } => {
            format!("{file} 校验不符（下载不完整或被篡改），已删掉，请重试")
        }
    }
}

/// 标记文件：`~/Library/Application Support/Glimmer/update-check`。
fn stamp_path() -> Option<PathBuf> {
    paths::user_data_dir().map(|dir| dir.join(STAMP_FILE))
}

/// 安装包目录：`~/Library/Application Support/Glimmer/updates/`。
fn download_dir() -> Option<PathBuf> {
    paths::user_data_dir().map(|dir| dir.join(DOWNLOAD_DIR))
}

#[cfg(test)]
mod tests {
    use glimmer_update::{Asset, Available};

    #[test]
    fn texts_read_naturally() {
        let update = Available {
            version: "0.1.8".into(),
            date: "2026-09-20".into(),
            notes: vec!["整句更准".into(), "修了一个崩溃".into()],
            asset: Asset {
                platform: "macos".into(),
                arch: "Apple Silicon".into(),
                file: "Glimmer-0.1.8-arm64.pkg".into(),
                url: String::new(),
                size: 0,
                sha256: String::new(),
            },
        };
        assert_eq!(
            super::available_text(&update),
            "有新版本 0.1.8（2026-09-20）：整句更准（还有 1 条，见官网）"
        );
        assert_eq!(super::progress_text(0, 0), "正在下载 0.0 MB…");
        assert_eq!(
            super::progress_text(12 * 1024 * 1024, 35 * 1024 * 1024),
            "正在下载 12.0 / 35.0 MB…"
        );
    }
}
