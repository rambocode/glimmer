//! 「从文件学习」：把用户选的文本读出来交给 Core 学个人 n-gram。这里只管找文件、读文件、报结果，切词与记转移在 `Engine::learn_text`。

use std::path::{Path, PathBuf};

use super::*;

/// 认这些扩展名的文本文件。
const TEXT_EXTENSIONS: [&str; 4] = ["md", "markdown", "txt", "text"];

/// 单个文件超过这么大就跳过：多半不是人写的笔记（导出的数据、日志）。
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// 一次最多学多少个文件：选了整个家目录这类误操作不至于把界面卡死（读与学都在主线程）。
const MAX_FILES: usize = 2000;

/// 文件夹最多往下走几层。
const MAX_DEPTH: usize = 6;

impl Host {
    /// 学 `sources`（文件或文件夹）里的文本，学完立刻落盘，结果显示在偏好设置底部的状态行。
    /// 不记文件路径、不留文件内容；日志只记个数。
    pub(super) fn learn_from_text(&mut self, sources: &[PathBuf]) {
        let mut files = Vec::new();
        for source in sources {
            collect_text_files(source, 0, &mut files);
        }
        let mut learned_files = 0;
        let mut transitions = 0;
        for file in &files {
            let Ok(text) = std::fs::read_to_string(file) else {
                continue;
            };
            let recorded = self.engine.learn_text(&text);
            if recorded > 0 {
                learned_files += 1;
                transitions += recorded;
            }
        }
        self.engine.flush_learning();
        tracing::info!(files = learned_files, transitions, "已从用户选的文本学习");
        let status = if files.is_empty() {
            "没找到 .md / .txt 文本文件".to_owned()
        } else if transitions == 0 {
            "没学到东西：文件里没有中文，或「学习输入习惯」关着".to_owned()
        } else {
            format!("已从 {learned_files} 个文件学了 {transitions} 处用词")
        };
        self.preferences.set_status(&status);
    }
}

/// 把 `path` 下认得的文本文件收进 `files`：文件直接看扩展名与大小，文件夹往下走（隐藏项与符号链接不进）。
fn collect_text_files(path: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    if files.len() >= MAX_FILES {
        return;
    }
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return;
    };
    if metadata.is_file() {
        let known = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| TEXT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()));
        if known && metadata.len() <= MAX_FILE_BYTES {
            files.push(path.to_owned());
        }
        return;
    }
    if !metadata.is_dir() || depth >= MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|child| {
            child
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| !name.starts_with('.'))
        })
        .collect();
    children.sort();
    for child in children {
        collect_text_files(&child, depth + 1, files);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_text_files_and_skips_the_rest() {
        let root = std::env::temp_dir().join(format!("glimmer-learn-text-{}", std::process::id()));
        let nested = root.join("notes");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        std::fs::write(root.join("a.md"), "笔记").unwrap();
        std::fs::write(nested.join("b.TXT"), "笔记").unwrap();
        std::fs::write(nested.join("c.png"), "x").unwrap();
        std::fs::write(root.join(".hidden").join("d.md"), "笔记").unwrap();
        let mut files = Vec::new();
        collect_text_files(&root, 0, &mut files);
        let names: Vec<String> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.md", "b.TXT"]);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
