//! 找 IBus 私有总线的地址，照 ibus `src/ibusshare.c` 的 `ibus_get_address` / `ibus_get_socket_path`：
//! 先看 `IBUS_ADDRESS`；否则读地址文件里的 `IBUS_ADDRESS=` 行。地址文件路径先看 `IBUS_ADDRESS_FILE`，
//! 否则是 `<用户配置目录>/ibus/bus/<machine-id>-<host>-<display-number>`：
//! 有 `WAYLAND_DISPLAY` 时 host 是 `unix`、display-number 是整个 `WAYLAND_DISPLAY`；
//! 否则拆 `DISPLAY`（`host:number.screen`，host 为空取 `unix`，没有 `DISPLAY` 时 number 取 `0`）。
//! ibus 还会用 `IBUS_DAEMON_PID` 检查 daemon 活着没有，这里不查：地址失效时连接会直接失败。

use std::path::PathBuf;

use crate::error::LinuxError;

/// 按当前进程的环境变量找地址。
pub fn resolve() -> Result<String, LinuxError> {
    let var = |name: &str| std::env::var(name).ok();
    if let Some(address) = var("IBUS_ADDRESS").filter(|address| !address.is_empty()) {
        return Ok(address);
    }
    let path = socket_path(var, &machine_id());
    let content = std::fs::read_to_string(&path).map_err(|source| LinuxError::AddressFile {
        path: path.clone(),
        source,
    })?;
    parse_address(&content).ok_or(LinuxError::NoAddress(path))
}

/// 地址文件的路径；`var` 取环境变量（测试注入）。
pub fn socket_path(var: impl Fn(&str) -> Option<String>, machine_id: &str) -> PathBuf {
    if let Some(path) = var("IBUS_ADDRESS_FILE") {
        return PathBuf::from(path);
    }
    let (host, number) = display_parts(&var);
    // glib 的 g_get_user_config_dir：XDG_CONFIG_HOME 非空就用它，否则 ~/.config。
    let config = var("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(var("HOME").unwrap_or_default()).join(".config"));
    config
        .join("ibus")
        .join("bus")
        .join(format!("{machine_id}-{host}-{number}"))
}

/// 从地址文件内容里取 `IBUS_ADDRESS=` 的值（跳过注释，多行时取最后一行，与 ibus 相同）。
pub fn parse_address(content: &str) -> Option<String> {
    content
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.strip_prefix("IBUS_ADDRESS="))
        .next_back()
        .map(str::to_owned)
}

/// `(host, display-number)`，规则见文件头。
fn display_parts(var: &impl Fn(&str) -> Option<String>) -> (String, String) {
    if let Some(wayland) = var("WAYLAND_DISPLAY") {
        return ("unix".to_owned(), wayland);
    }
    let Some(display) = var("DISPLAY") else {
        return ("unix".to_owned(), "0".to_owned());
    };
    let (host, rest) = match display.split_once(':') {
        Some((host, rest)) => (host, rest),
        None => (display.as_str(), "0"),
    };
    let number = rest.split('.').next().unwrap_or(rest);
    let host = if host.is_empty() { "unix" } else { host };
    (host.to_owned(), number.to_owned())
}

/// 本机 machine-id：先 `/var/lib/dbus/machine-id`，再 `/etc/machine-id`，都读不到时 ibus 用字面量 `machine-id`。
fn machine_id() -> String {
    ["/var/lib/dbus/machine-id", "/etc/machine-id"]
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .map(|id| id.trim().to_owned())
        .unwrap_or_else(|| "machine-id".to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn x11_display_with_screen() {
        let path = socket_path(env(&[("HOME", "/home/u"), ("DISPLAY", ":1.0")]), "abc");
        assert_eq!(path, PathBuf::from("/home/u/.config/ibus/bus/abc-unix-1"));
    }

    #[test]
    fn remote_x11_host() {
        let path = socket_path(
            env(&[("XDG_CONFIG_HOME", "/cfg"), ("DISPLAY", "box:2")]),
            "abc",
        );
        assert_eq!(path, PathBuf::from("/cfg/ibus/bus/abc-box-2"));
    }

    #[test]
    fn wayland_uses_whole_display_name() {
        let path = socket_path(
            env(&[
                ("HOME", "/home/u"),
                ("WAYLAND_DISPLAY", "wayland-0"),
                ("DISPLAY", ":0"),
            ]),
            "abc",
        );
        assert_eq!(
            path,
            PathBuf::from("/home/u/.config/ibus/bus/abc-unix-wayland-0")
        );
    }

    #[test]
    fn no_display_defaults_to_zero() {
        let path = socket_path(env(&[("HOME", "/h")]), "id");
        assert_eq!(path, PathBuf::from("/h/.config/ibus/bus/id-unix-0"));
    }

    #[test]
    fn address_file_override() {
        let path = socket_path(env(&[("IBUS_ADDRESS_FILE", "/tmp/addr")]), "id");
        assert_eq!(path, PathBuf::from("/tmp/addr"));
    }

    #[test]
    fn parses_address_line() {
        let content = "# comment\nIBUS_ADDRESS=unix:path=/tmp/ibus-x,guid=1\nIBUS_DAEMON_PID=42\n";
        assert_eq!(
            parse_address(content).as_deref(),
            Some("unix:path=/tmp/ibus-x,guid=1")
        );
        assert_eq!(parse_address("# nothing\n"), None);
    }
}
