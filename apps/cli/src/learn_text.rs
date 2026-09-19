//! `--learn-text`：从用户自己写的中文文本里学个人 n-gram（[`Engine::learn_text`]），只在本机、只进学习器。

use std::path::PathBuf;

use glimmer_core::Engine;

/// 读 `paths` 里的文本逐个学；返回一共记了多少条转移。
pub fn run(engine: &mut Engine, paths: &[PathBuf]) -> Result<usize, LearnTextError> {
    let mut recorded = 0;
    for path in paths {
        let text = std::fs::read_to_string(path).map_err(|source| LearnTextError::Read {
            path: path.clone(),
            source,
        })?;
        recorded += engine.learn_text(&text);
    }
    tracing::info!(files = paths.len(), transitions = recorded, "已从文本学习");
    Ok(recorded)
}

/// 读不了文本。
#[derive(Debug, thiserror::Error)]
pub enum LearnTextError {
    #[error("cannot read text to learn from {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
