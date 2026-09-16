//! 启动装配：XDG 目录、配置模板、日志，然后交给 `glimmer-server` 装 Router。

use std::path::PathBuf;

use glimmer_platform::{Config, LogLevel, resources};
use glimmer_server::{ServerError, StartupPaths, build_router};

use crate::backend::RouterBackend;

/// 配置目录 `$XDG_CONFIG_HOME/glimmer`（缺省 `~/.config/glimmer`）：`config.toml` 与 `.env`。
fn config_dir() -> Option<PathBuf> {
    xdg_dir("XDG_CONFIG_HOME", ".config").map(|dir| dir.join("glimmer"))
}

/// 数据目录 `$XDG_DATA_HOME/glimmer`（缺省 `~/.local/share/glimmer`）：学习数据、用户词库、日志。
fn data_dir() -> Option<PathBuf> {
    xdg_dir("XDG_DATA_HOME", ".local/share").map(|dir| dir.join("glimmer"))
}

/// XDG 基础目录：环境变量是绝对路径就用它（规范要求忽略相对路径），否则 `$HOME/<fallback>`。
fn xdg_dir(variable: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(fallback)))
}

/// 读密钥：工作目录 `.env`，再叠加配置目录的 `.env`；不覆盖已有环境变量。
fn load_env() {
    let _ = dotenvy::dotenv();
    if let Some(env_file) = config_dir().map(|dir| dir.join(".env")) {
        let _ = dotenvy::from_path(&env_file);
    }
}

/// 首次启动写出带说明的配置模板，再读配置；解析失败记错误退回默认。
fn load_config() -> Config {
    let Some(path) = config_dir().map(|dir| dir.join("config.toml")) else {
        return Config::default();
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match Config::write_template_if_missing(&path) {
        Ok(true) => tracing::info!(path = %path.display(), "已写出配置模板"),
        Ok(false) => {}
        Err(error) => tracing::warn!(%error, "写配置模板失败"),
    }
    Config::load(&path).unwrap_or_else(|error| {
        tracing::error!(%error, path = %path.display(), "配置解析失败，用默认值");
        Config::default()
    })
}

/// 日志写 stderr（ibus-daemon / fcitx5 的输出）与 `<数据目录>/logs/<file_prefix>.<日期>.log`（按天滚动，留 7 天）。
/// 级别按 `[general] log_level`，`RUST_LOG` 可覆盖。返回的 guard 要活到进程结束，否则缓冲的日志不落盘。
/// 全局 subscriber 只能装一次，重复调用返回 `None`、不 panic。
pub fn init_logging(file_prefix: &str) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    // 级别取自配置，而日志还没装：这里静默读一次，读配置的告警由 build_backend 再读时记下
    let level = match config_dir()
        .map(|dir| dir.join("config.toml"))
        .and_then(|path| Config::load(&path).ok())
    {
        Some(config) if config.general.log_level == LogLevel::Debug => "debug",
        _ => "info",
    };
    // zbus 的 info 日志每条都带整条 D-Bus 消息的 span，压到 warn
    let filter = tracing_subscriber::EnvFilter::try_from_env("RUST_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(format!("{level},zbus=warn")));
    let appender = data_dir()
        .map(|dir| dir.join("logs"))
        .filter(|dir| std::fs::create_dir_all(dir).is_ok())
        .and_then(|dir| {
            tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix(file_prefix)
                .filename_suffix("log")
                .max_log_files(7)
                .build(dir)
                .ok()
        });
    match appender {
        Some(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_ansi(false)
                .with_writer(writer.and(std::io::stderr))
                .try_init()
                .ok()
                .map(|()| guard)
        }
        None => {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(std::io::stderr)
                .try_init();
            None
        }
    }
}

/// 读配置、按随包资源装好 Router 后端。`root` 是随包资源根；`None` 时按可执行文件位置找（`resources::bundled_root`），
/// 跑在别人进程里的前端（Fcitx5 插件在 `/usr/bin/fcitx5` 里）要显式传。
pub fn build_backend(root: Option<PathBuf>) -> Result<RouterBackend, ServerError> {
    load_env();
    let config = load_config();
    let paths = StartupPaths {
        user_dir: data_dir().filter(|dir| std::fs::create_dir_all(dir).is_ok()),
        config_path: config_dir().map(|dir| dir.join("config.toml")),
        // 装机布局是 exe 同级（/usr/lib/glimmer），开发布局是仓库根；都找不到回落工作目录
        root: root
            .or_else(resources::bundled_root)
            .unwrap_or_else(|| PathBuf::from(".")),
        version: env!("CARGO_PKG_VERSION"),
        platform: "linux",
    };
    build_router(&paths, &config).map(RouterBackend::new)
}
