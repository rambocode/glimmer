//! 版本号解析与比较。

use std::cmp::Ordering;

/// 预发布段里的一节：数字节按数值比，字母节按 ASCII 比，数字节小于字母节（SemVer 规则）。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Identifier {
    Numeric(u64),
    Alpha(String),
}

impl Ord for Identifier {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Numeric(a), Self::Numeric(b)) => a.cmp(b),
            (Self::Alpha(a), Self::Alpha(b)) => a.cmp(b),
            (Self::Numeric(_), Self::Alpha(_)) => Ordering::Less,
            (Self::Alpha(_), Self::Numeric(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for Identifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 版本号 `主.次.修订[-预发布][+构建]`，按 SemVer 比较，`+` 后面的构建元数据不参与比较。
///
/// 本仓库的开发版写法是在发布版后面接 `-dev-<短哈希>`（`0.1.8-dev-1a2b3c4`、`0.1.0-alpha.7-dev-1a2b3c4`），
/// 意思是「正在做 0.1.8，还没发」，所以 `dev` 段单独拆出来：带 dev 的比同号不带 dev 的**小**，比前一个版本大。
/// 纯 SemVer 会把 `alpha.7-dev-x` 排在 `alpha.7` 后面，那样开发机永远看不到自己那一版的发布。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    major: u64,

    minor: u64,

    patch: u64,

    /// `-dev` 之前的预发布段（`alpha.6` → `[alpha, 6]`），发布版为空。
    pre: Vec<Identifier>,

    /// 是不是开发版（预发布段里有 `dev` 一节）。
    dev: bool,
}

impl Version {
    /// 解析失败（不是三段数字开头、预发布段有空节）返回 `None`。允许开头的 `v` 与结尾表示脏工作区的 `+`。
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().trim_start_matches('v');
        // `+` 后面是构建元数据（脏工作区的标记也是它），不参与比较
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut numbers = core.split('.').map(|part| part.parse::<u64>().ok());
        let major = numbers.next()??;
        let minor = numbers.next()??;
        let patch = numbers.next()??;
        if numbers.next().is_some() {
            return None;
        }
        let mut identifiers = Vec::new();
        let mut dev = false;
        if let Some(pre) = pre {
            // `-` 与 `.` 都当分隔：`alpha.7-dev-1a2b3c4` → alpha, 7, dev, 1a2b3c4；dev 之后的短哈希不参与比较
            for part in pre.split(['.', '-']) {
                if part.is_empty() {
                    return None;
                }
                if part == "dev" {
                    dev = true;
                    break;
                }
                identifiers.push(match part.parse::<u64>() {
                    Ok(number) if part.bytes().all(|b| b.is_ascii_digit()) => {
                        Identifier::Numeric(number)
                    }
                    _ => Identifier::Alpha(part.to_owned()),
                });
            }
        }
        Some(Self {
            major,
            minor,
            patch,
            pre: identifiers,
            dev,
        })
    }

    /// 是不是开发版（版本号里带 `-dev`）。
    pub fn is_dev(&self) -> bool {
        self.dev
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then_with(|| compare_pre(&self.pre, &other.pre))
            // 同号：开发版排在发布版前面
            .then(other.dev.cmp(&self.dev))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// SemVer 的预发布比较：没有预发布段的是正式版，比任何预发布大；都有则逐节比，前缀相同短的小。
fn compare_pre(a: &[Identifier], b: &[Identifier]) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => a.cmp(b),
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap_or_else(|| panic!("解析不了 {text}"))
    }

    #[test]
    fn releases_order_by_semver() {
        assert!(v("0.1.7") < v("0.1.8"));
        assert!(v("0.1.7") < v("0.2.0"));
        assert!(v("0.1.0-alpha.6") < v("0.1.0-alpha.7"));
        assert!(v("0.1.0-alpha.6") < v("0.1.0"));
        assert!(v("0.1.0-linux.1") < v("0.1.0-linux.2"));
        assert!(v("0.1.0-alpha.6") < v("0.1.0-linux.2"));
        assert_eq!(v("0.1.7"), v("v0.1.7"));
    }

    #[test]
    fn dev_builds_sit_between_previous_and_own_release() {
        assert!(v("0.1.7") < v("0.1.8-dev-1a2b3c4"));
        assert!(v("0.1.8-dev-1a2b3c4") < v("0.1.8"));
        assert!(v("0.1.8-dev-1a2b3c4+") < v("0.1.8"));
        assert!(v("0.1.0-alpha.6") < v("0.1.0-alpha.7-dev-1a2b3c4"));
        assert!(v("0.1.0-alpha.7-dev-1a2b3c4") < v("0.1.0-alpha.7"));
        assert!(v("0.1.8-dev-1a2b3c4").is_dev());
        assert!(!v("0.1.8").is_dev());
    }

    #[test]
    fn rejects_garbage() {
        assert!(Version::parse("").is_none());
        assert!(Version::parse("0.1").is_none());
        assert!(Version::parse("0.1.7.1").is_none());
        assert!(Version::parse("0.1.7-").is_none());
        assert!(Version::parse("预览").is_none());
    }
}
