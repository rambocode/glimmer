//! 数据文件位置：只读数据在 `.app/Contents/Resources/`，用户数据在 `~/Library/Application Support/Glimmer/`。

use std::path::{Path, PathBuf};

use objc2_foundation::NSBundle;

use crate::error::HostError;

/// 主 bundle 的 Resources 目录。
pub fn resources_dir() -> Result<PathBuf, HostError> {
    NSBundle::mainBundle()
        .resourcePath()
        .map(|p| PathBuf::from(p.to_string()))
        .ok_or(HostError::NoResources)
}

/// 某个资源文件的完整路径，不存在时报错而不是等到读取时才炸。
pub fn resource(name: &str) -> Result<PathBuf, HostError> {
    let path = resources_dir()?.join(name);
    if path.is_file() {
        Ok(path)
    } else {
        Err(HostError::MissingResource(path))
    }
}

/// 随包的领域词库目录：`.app/Contents/Resources/dicts/`；包里没有就是 `None`。
pub fn bundled_dicts_dir() -> Option<PathBuf> {
    let dir = resources_dir().ok()?.join("dicts");
    dir.is_dir().then_some(dir)
}

/// 配置文件：`~/Library/Application Support/Glimmer/config.toml`。
pub fn config_file() -> Option<PathBuf> {
    user_data_dir().map(|dir| dir.join("config.toml"))
}

/// 用户数据目录，不存在则创建；首次运行时把派生前「青简」的数据目录整个搬过来。
pub fn user_data_dir() -> Option<PathBuf> {
    let support = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    let dir = support.join("Glimmer");
    migrate_legacy_data_dir(&support.join("Qingjian"), &dir);
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 一次性迁移：新目录还不存在而旧目录在，就整个 rename（同卷瞬间完成，学习数据、配置、密钥全保留）。
/// 只在新目录完全不存在时做，避免两边都有数据时覆盖用户在新目录里的改动。
fn migrate_legacy_data_dir(legacy: &Path, dir: &Path) {
    if dir.exists() || !legacy.is_dir() {
        return;
    }
    match std::fs::rename(legacy, dir) {
        Ok(()) => {
            tracing::info!(from = %legacy.display(), to = %dir.display(), "已迁移青简的用户数据目录")
        }
        Err(err) => tracing::warn!(%err, "迁移青简用户数据目录失败，改用空目录"),
    }
}

/// 附加词库目录：`~/Library/Application Support/Glimmer/dicts/`，不存在则创建。
pub fn dicts_dir() -> Option<PathBuf> {
    let dir = user_data_dir()?.join("dicts");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 本地整句模型（`.qjm` 单文件，或开发时的三件套目录）：
/// 用户目录 `model/` 里有就用它（自己训的），否则用包里的 `Resources/model/`；都没有是 `None`。
pub fn model_path() -> Option<PathBuf> {
    let user = user_data_dir()?.join("model");
    if let Some(found) = glimmer_neural::find_model(&user) {
        return Some(found);
    }
    glimmer_neural::find_model(&resources_dir().ok()?.join("model"))
}

#[cfg(test)]
mod tests {
    use super::migrate_legacy_data_dir;

    /// 新目录不存在、旧目录在：整个搬过去，内容保留。
    #[test]
    fn moves_legacy_dir_when_new_missing() {
        let root = std::env::temp_dir().join(format!("glimmer-migrate-{}", std::process::id()));
        let legacy = root.join("Qingjian");
        let dir = root.join("Glimmer");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("user.tsv"), "a\t1\n").unwrap();
        migrate_legacy_data_dir(&legacy, &dir);
        assert!(!legacy.exists());
        assert_eq!(
            std::fs::read_to_string(dir.join("user.tsv")).unwrap(),
            "a\t1\n"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// 新目录已有：不动旧目录，不覆盖新目录。
    #[test]
    fn keeps_both_when_new_exists() {
        let root =
            std::env::temp_dir().join(format!("glimmer-migrate-keep-{}", std::process::id()));
        let legacy = root.join("Qingjian");
        let dir = root.join("Glimmer");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("user.tsv"), "new\n").unwrap();
        migrate_legacy_data_dir(&legacy, &dir);
        assert!(legacy.is_dir());
        assert_eq!(
            std::fs::read_to_string(dir.join("user.tsv")).unwrap(),
            "new\n"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
