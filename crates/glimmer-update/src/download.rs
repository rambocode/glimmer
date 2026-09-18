//! 下载安装包到本机并校验 sha256。线程 + 通道版给主线程轮询（带进度），阻塞版给自带后台任务的 UI 框架。

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use sha2::{Digest, Sha256};

use crate::error::UpdateError;
use crate::feed::Asset;
use crate::http::download_client;

/// 每次从连接上读多少再写盘、更新进度。
const CHUNK: usize = 64 * 1024;

/// 一次进行中的下载。
pub struct Download {
    /// 下载线程送回的结果：校验过的安装包路径；线程只发一次。
    result: mpsc::Receiver<Result<PathBuf, UpdateError>>,

    /// 已收到的字节数，线程边下边加。
    received: Arc<AtomicU64>,

    /// 清单里的体积（0 表示不知道）。
    total: u64,
}

impl Download {
    /// 起线程把 `asset` 下到 `dir` 里（文件名取清单里的）。
    pub fn start(asset: &Asset, dir: &Path) -> Result<Self, UpdateError> {
        let total = asset.size;
        let asset = asset.clone();
        let dir = dir.to_owned();
        let received = Arc::new(AtomicU64::new(0));
        let counter = Arc::clone(&received);
        let (sender, result) = mpsc::channel();
        std::thread::Builder::new()
            .name("glimmer-update-download".to_owned())
            .spawn(move || {
                let _ = sender.send(download_blocking(&asset, &dir, &counter));
            })?;
        Ok(Self {
            result,
            received,
            total,
        })
    }

    /// 结果到了就取走；没到返回 `None`。
    pub fn poll(&self) -> Option<Result<PathBuf, UpdateError>> {
        self.result.try_recv().ok()
    }

    /// （已收到字节数，总字节数）；总数为 0 是清单没写体积。
    pub fn progress(&self) -> (u64, u64) {
        (self.received.load(Ordering::Relaxed), self.total)
    }
}

/// 阻塞地下载：先写 `<文件名>.part`，边下边算 sha256，对上清单才改成正式文件名。
/// 目录里上一次留下的 `Glimmer-*` 先删掉，免得装了几版之后堆一堆安装包。
pub fn download_blocking(
    asset: &Asset,
    dir: &Path,
    received: &AtomicU64,
) -> Result<PathBuf, UpdateError> {
    if asset.sha256.trim().is_empty() {
        return Err(UpdateError::MissingChecksum(asset.file.clone()));
    }
    std::fs::create_dir_all(dir)?;
    remove_old_packages(dir);
    let target = dir.join(&asset.file);
    let part = dir.join(format!("{}.part", asset.file));
    let outcome = fetch_to(&asset.url, &part, received);
    let actual = match outcome {
        Ok(digest) => digest,
        Err(error) => {
            let _ = std::fs::remove_file(&part);
            return Err(error);
        }
    };
    if !actual.eq_ignore_ascii_case(asset.sha256.trim()) {
        let _ = std::fs::remove_file(&part);
        tracing::warn!(file = %asset.file, expected = %asset.sha256, actual = %actual, "安装包校验失败");
        return Err(UpdateError::Checksum {
            file: asset.file.clone(),
            expected: asset.sha256.clone(),
            actual,
        });
    }
    std::fs::rename(&part, &target)?;
    tracing::info!(path = %target.display(), "安装包已下载并通过校验");
    Ok(target)
}

/// 把 `url` 流式写到 `path`，返回内容的 sha256（十六进制小写）。
fn fetch_to(url: &str, path: &Path, received: &AtomicU64) -> Result<String, UpdateError> {
    let mut response = download_client()?.get(url).send()?.error_for_status()?;
    let mut file = File::create(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    received.store(0, Ordering::Relaxed);
    loop {
        let n = response.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])?;
        hasher.update(&buffer[..n]);
        received.fetch_add(n as u64, Ordering::Relaxed);
    }
    file.flush()?;
    Ok(hex(&hasher.finalize()))
}

/// 清掉目录里以前下的安装包与半截文件；只动我们自己命名的文件。
fn remove_old_packages(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("Glimmer-") {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// 字节串转十六进制小写。
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    #[test]
    fn hex_matches_sha256sum_output() {
        // `printf 'abc' | shasum -a 256`
        let digest = super::hex(&Sha256::digest(b"abc"));
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn missing_checksum_refuses_before_touching_network() {
        let asset = super::Asset {
            platform: "macos".into(),
            arch: "Intel".into(),
            file: "Glimmer-9.9.9-x86_64.pkg".into(),
            url: "http://127.0.0.1:9/nope".into(),
            size: 0,
            sha256: String::new(),
        };
        let dir = std::env::temp_dir().join("glimmer-update-test-missing-checksum");
        let counter = std::sync::atomic::AtomicU64::new(0);
        let error = super::download_blocking(&asset, &dir, &counter).unwrap_err();
        assert!(matches!(error, super::UpdateError::MissingChecksum(_)));
    }
}
