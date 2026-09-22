//! TSV → `.qj`：解析成内存结构后原样落盘，加上元数据；`model` 是三件套目录 → `.qjm`。

use std::path::{Path, PathBuf};
use std::time::Instant;

use glimmer_core::Language;
use glimmer_dictionary::Dictionary;
use glimmer_format::Metadata;
use glimmer_lm::NgramModel;
use glimmer_translate::Glossary;

use crate::args::PackKind;
use crate::error::ConvertError;

/// 打包一种数据。`inputs` 为空时从 `out_dir` 里找缺省的 TSV；`output` 给了就用它当输出文件名，否则按种类取缺省名。
pub fn pack(
    kind: PackKind,
    inputs: &[PathBuf],
    language: &str,
    output: Option<&str>,
    metadata: Metadata,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    // 输出文件名：显式给的优先，否则用各种类的缺省名
    let out_path = |default: String| out_dir.join(output.map_or(default, str::to_owned));
    let metadata = Metadata {
        generator: format!("glimmer-dict-convert {}", env!("CARGO_PKG_VERSION")),
        ..metadata
    };
    let started = Instant::now();
    match kind {
        PackKind::Dict => {
            let input = inputs
                .first()
                .cloned()
                .unwrap_or_else(|| out_dir.join("dict.tsv"));
            let dictionary = Dictionary::from_path(&input)?;
            let out = out_path("dict.qj".to_owned());
            dictionary.write_qj(&out, &metadata)?;
            report(&out, dictionary.len(), started);
        }
        PackKind::Lm => {
            // 显式给输入时第三个是三元表（可省）；不给就在 out_dir 里找，三元表在就一起打
            let (unigram, bigram, trigram) = match inputs {
                [unigram, bigram, trigram, ..] => {
                    (unigram.clone(), bigram.clone(), Some(trigram.clone()))
                }
                [unigram, bigram] => (unigram.clone(), bigram.clone(), None),
                _ => {
                    let trigram = out_dir.join("lm-trigram.tsv");
                    (
                        out_dir.join("lm-unigram.tsv"),
                        out_dir.join("lm-bigram.tsv"),
                        trigram.is_file().then_some(trigram),
                    )
                }
            };
            let model = NgramModel::from_paths(&unigram, &bigram, trigram.as_deref())?;
            let out = out_path("lm.qj".to_owned());
            model.write_qj(&out, &metadata)?;
            report(&out, model.bigram_count() + model.trigram_count(), started);
        }
        PackKind::Glossary => {
            let language: Language = language.parse().map_err(|_| ConvertError::Format {
                path: PathBuf::from(language),
                line: 0,
                reason: "language must be en / ja / zh / es".to_owned(),
            })?;
            let input = inputs.first().cloned().unwrap_or_else(|| {
                PathBuf::from("assets/glossary").join(format!("glossary-{}.tsv", language.code()))
            });
            let glossary = Glossary::from_path(language, &input)?;
            let out = out_path(format!("glossary-{}.qj", language.code()));
            glossary.write_qj(&out, &metadata)?;
            report(&out, glossary.len(), started);
        }
        PackKind::Model => {
            let input = inputs
                .first()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("data/model"));
            let out = out_path("model.qjm".to_owned());
            let parameters = glimmer_neural::qjm::pack(&input, &out, &metadata)?;
            report(
                &out,
                usize::try_from(parameters).unwrap_or(usize::MAX),
                started,
            );
        }
    }
    Ok(())
}

fn report(out: &Path, entries: usize, started: Instant) {
    let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
    tracing::info!(
        out = %out.display(),
        entries,
        size_mb = size / 1_000_000,
        elapsed_ms = started.elapsed().as_millis(),
        "已打包"
    );
}
