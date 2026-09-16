//! Server 进程入口：读配置、装配 Engine、在命名管道上服务 TSF DLL。逻辑在库部分，这里只装配与启动。
//! release 编成 GUI 子系统（登录自启静默跑，日志走文件）；debug 保留控制台看 stderr。
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::PathBuf;

use glimmer_platform::{Config, ConfigError, LogLevel, resources};
use glimmer_server::{Router, StartupPaths, build_router};

/// 用户数据目录 `%APPDATA%\Glimmer`。非 Windows 拿不到。
fn user_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("Glimmer"))
}

fn config_path() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("config.toml"))
}

/// 首次启动把带说明的配置模板写到 `%APPDATA%\Glimmer\config.toml`（与 macOS 一致）；
/// 这时日志还没装好，结果交给 `main` 记。已有文件返回 `Ok(false)`。
fn write_config_template() -> Option<Result<bool, ConfigError>> {
    let path = config_path()?;
    if let Some(dir) = path.parent()
        && let Err(source) = std::fs::create_dir_all(dir)
    {
        return Some(Err(ConfigError::Write { path, source }));
    }
    Some(Config::write_template_if_missing(&path))
}

/// 文件不存在按默认值；解析失败记错误退回默认。
fn load_config() -> Config {
    match config_path() {
        Some(path) => Config::load(&path).unwrap_or_else(|error| {
            tracing::error!(%error, path = %path.display(), "配置解析失败，用默认值");
            Config::default()
        }),
        None => Config::default(),
    }
}

/// 读密钥：工作目录 `.env`，再叠加 `%APPDATA%\Glimmer\.env`；不覆盖已有环境变量。
fn load_env() {
    let _ = dotenvy::dotenv();
    if let Some(env_file) = user_dir().map(|dir| dir.join(".env")) {
        let _ = dotenvy::from_path(&env_file);
    }
}

fn log_dir() -> Option<PathBuf> {
    let dir = user_dir()?.join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 级别按 `[general] log_level`（`RUST_LOG` 可覆盖），同时写 stderr 与按天滚动的文件（留 7 天）。
/// 返回的 guard 要活到进程结束，否则缓冲的日志不落盘。
fn init_logging(config: &Config) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let level = if config.general.log_level == LogLevel::Debug {
        "debug"
    } else {
        "info"
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));
    match log_dir() {
        Some(dir) => {
            let appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("glimmer-server")
                .filename_suffix("log")
                .max_log_files(7)
                .build(&dir)
                .expect("构建滚动日志文件");
            let (writer, guard) = tracing_appender::non_blocking(appender);
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_ansi(false)
                .with_writer(writer.and(std::io::stderr))
                .init();
            Some(guard)
        }
        None => {
            tracing_subscriber::fmt().with_env_filter(filter).init();
            None
        }
    }
}

fn main() {
    load_env();

    // 日志级别取自配置，所以先写模板、读配置，再装日志。
    let template = write_config_template();
    let config = load_config();
    let _log_guard = init_logging(&config);
    match template {
        Some(Ok(true)) => tracing::info!("已写出配置模板"),
        Some(Err(error)) => tracing::warn!(%error, "写配置模板失败"),
        _ => {}
    }
    // 装机布局与 exe 同级，开发布局是仓库根；都找不到回落工作目录。
    let paths = StartupPaths {
        user_dir: user_dir(),
        config_path: config_path(),
        root: resources::bundled_root().unwrap_or_else(|| PathBuf::from(".")),
        version: env!("CARGO_PKG_VERSION"),
        platform: "windows",
    };
    let router = match build_router(&paths, &config) {
        Ok(router) => router,
        Err(error) => {
            tracing::error!(%error, "样例词库也装配失败");
            std::process::exit(1);
        }
    };

    serve(router);
}

/// DLL 日志目录 `%LOCALAPPDATA%\Glimmer` 给 AppContainer 应用（任务栏搜索 / 设置）写权限：
/// 那些进程里的 DLL 默认写不了用户目录，出了问题连日志都没有。失败只记警告。
#[cfg(windows)]
fn grant_appcontainer_log_access() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let Some(dir) =
        std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Glimmer"))
    else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(&dir) {
        tracing::warn!(%error, dir = %dir.display(), "建 DLL 日志目录失败");
        return;
    }
    // S-1-15-2-1 = ALL APPLICATION PACKAGES，S-1-15-2-2 = ALL RESTRICTED APPLICATION PACKAGES。
    let status = std::process::Command::new("icacls")
        .arg(&dir)
        .args(["/grant", "*S-1-15-2-1:(OI)(CI)M"])
        .args(["/grant", "*S-1-15-2-2:(OI)(CI)M"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => tracing::warn!(%status, "给 AppContainer 授权 DLL 日志目录失败"),
        Err(error) => tracing::warn!(%error, "跑 icacls 失败"),
    }
}

/// 起 UI 线程作为候选窗口 / 状态条的输出端（失败退化为不画），再在命名管道上服务到进程结束。
#[cfg(windows)]
fn serve(mut router: Router) {
    use glimmer_windows_server::ipc::{Work, pipe};
    use glimmer_windows_server::ui::UiHandle;
    grant_appcontainer_log_access();
    // 工人循环的活：各连接的消息 + 状态条上的操作（UI 线程投进来）。
    let (work_tx, work_rx) = std::sync::mpsc::channel::<Work>();
    let status_events = work_tx.clone();
    let on_status = Box::new(move |event| {
        let _ = status_events.send(Work::Status(event));
    });
    match UiHandle::spawn(on_status) {
        Ok(ui) => {
            router.set_candidate_sink(Box::new(ui.clone()));
            router.set_status_sink(Box::new(ui));
        }
        Err(error) => tracing::error!(%error, "UI 线程启动失败，将不显示候选框 / 状态条"),
    }
    if let Err(error) = pipe::serve_pipe(pipe::DEFAULT_PIPE_NAME, &mut router, work_tx, work_rx) {
        tracing::error!(%error, "命名管道服务退出");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn serve(_router: Router) {
    tracing::warn!("命名管道传输仅 Windows 提供；本平台只装配 Engine 供测试");
}
