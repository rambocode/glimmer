use std::path::{Path, PathBuf};

use glimmer_lm::{LmError, NgramModel};

/// 语言模型的数据文件：`lm.qj` 优先，没有就用 TSV（三元表可选，没有就是纯二元模型）。
pub enum LanguageModelFiles {
    Packed(PathBuf),

    Tsv {
        unigram: PathBuf,

        bigram: PathBuf,

        trigram: Option<PathBuf>,
    },
}

impl LanguageModelFiles {
    pub fn find(dir: &Path) -> Option<Self> {
        let packed = dir.join("lm.qj");
        if packed.is_file() {
            return Some(Self::Packed(packed));
        }
        let unigram = dir.join("lm-unigram.tsv");
        let bigram = dir.join("lm-bigram.tsv");
        let trigram = dir.join("lm-trigram.tsv");
        (unigram.is_file() && bigram.is_file()).then_some(Self::Tsv {
            unigram,
            bigram,
            trigram: trigram.is_file().then_some(trigram),
        })
    }

    pub(super) fn load(&self) -> Result<NgramModel, LmError> {
        match self {
            Self::Packed(path) => NgramModel::from_path(path),
            Self::Tsv {
                unigram,
                bigram,
                trigram,
            } => NgramModel::from_paths(unigram, bigram, trigram.as_deref()),
        }
    }
}
