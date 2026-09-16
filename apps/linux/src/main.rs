//! `glimmer-ibus`：IBus 引擎进程入口。ibus-daemon 按组件 XML 的 `<exec>` 以 `--ibus` 拉起本进程。
//! 这一阶段装配的是回显后端（[`EchoBackend`]），接 Router 时只换这里。

use std::process::ExitCode;

use glimmer_linux::EchoBackend;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("RUST_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    if !std::env::args().skip(1).any(|arg| arg == "--ibus") {
        eprintln!("用法：glimmer-ibus --ibus（由 ibus-daemon 拉起）");
        return ExitCode::from(2);
    }
    match glimmer_linux::run(EchoBackend::new()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "IBus 引擎退出");
            ExitCode::FAILURE
        }
    }
}
