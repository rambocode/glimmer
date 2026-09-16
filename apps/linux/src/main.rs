//! `glimmer-ibus`：IBus 引擎进程入口。ibus-daemon 按组件 XML 的 `<exec>` 以 `--ibus` 拉起本进程。
//! 缺省装配真 Router；`--echo` 换成回显后端，只验 IBus 链路（Docker 端到端测试用）。

use std::process::ExitCode;

use glimmer_linux::{EchoBackend, build_backend, init_logging};

fn main() -> ExitCode {
    let _log_guard = init_logging();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.iter().any(|arg| arg == "--ibus") {
        eprintln!("用法：glimmer-ibus --ibus [--echo]（由 ibus-daemon 拉起）");
        return ExitCode::from(2);
    }
    let result = if args.iter().any(|arg| arg == "--echo") {
        glimmer_linux::run(EchoBackend::new())
    } else {
        match build_backend() {
            Ok(backend) => glimmer_linux::run(backend),
            Err(error) => {
                tracing::error!(%error, "样例词库也装配失败");
                return ExitCode::FAILURE;
            }
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "IBus 引擎退出");
            ExitCode::FAILURE
        }
    }
}
