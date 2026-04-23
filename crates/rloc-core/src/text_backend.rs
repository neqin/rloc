use std::fs;

use crate::{
    BackendFileAnalysis, FileCategory, FileMetrics, Language, LanguageBackend, LanguageDescriptor,
    LineBreakdown, LineExplanation, Utf8Path,
};

#[derive(Debug, Clone, Copy)]
enum TextMode {
    Markdown,
    Config,
}

#[derive(Debug, Clone, Copy)]
pub struct PlainTextBackend {
    descriptor: LanguageDescriptor,
    mode: TextMode,
}

pub fn markdown_backend() -> PlainTextBackend {
    PlainTextBackend {
        descriptor: LanguageDescriptor::new(Language::Markdown, "Markdown", &["md"]),
        mode: TextMode::Markdown,
    }
}

pub fn config_backend() -> PlainTextBackend {
    PlainTextBackend {
        descriptor: LanguageDescriptor::new(
            Language::Config,
            "Config",
            &[
                "toml", "yaml", "yml", "json", "jsonc", "lock", "ini", "cfg", "conf",
            ],
        ),
        mode: TextMode::Config,
    }
}

impl LanguageBackend for PlainTextBackend {
    fn descriptor(&self) -> LanguageDescriptor {
        self.descriptor
    }

    fn classify_file(
        &self,
        path: &Utf8Path,
        category: FileCategory,
        _options: &crate::ClassificationOptions,
    ) -> Result<BackendFileAnalysis, String> {
        let bytes = fs::read(path.as_std_path()).map_err(|error| error.to_string())?;
        let contents = String::from_utf8_lossy(&bytes);
        let mut blank_lines = 0u32;
        let mut code_lines = 0u32;
        let mut comment_lines = 0u32;
        let mut doc_lines = 0u32;
        let mut explanations = Vec::new();

        for (index, line) in contents.lines().enumerate() {
            let trimmed = line.trim();
            let (kind, reason) = if trimmed.is_empty() {
                blank_lines += 1;
                ("blank", "line contained only whitespace")
            } else {
                match self.mode {
                    TextMode::Markdown => {
                        doc_lines += 1;
                        ("doc", "markdown content counted as documentation")
                    }
                    TextMode::Config if is_config_comment(trimmed) => {
                        comment_lines += 1;
                        ("comment", "config comment line counted as comment")
                    }
                    TextMode::Config => {
                        code_lines += 1;
                        ("code", "config entry counted as code")
                    }
                }
            };

            explanations.push(LineExplanation {
                line_number: index as u32 + 1,
                kind: kind.to_owned(),
                snippet: trimmed.to_owned(),
                reason: reason.to_owned(),
            });
        }

        let total_lines = explanations.len() as u32;

        Ok(BackendFileAnalysis {
            metrics: FileMetrics::from_line_breakdown(
                path.to_path_buf(),
                self.descriptor.language,
                category,
                bytes.len() as u64,
                LineBreakdown {
                    total: total_lines,
                    blank: blank_lines,
                    code: code_lines,
                    comment: comment_lines,
                    doc: doc_lines,
                    ..LineBreakdown::default()
                },
            ),
            line_explanations: explanations,
            warnings: Vec::new(),
        })
    }
}

fn is_config_comment(trimmed: &str) -> bool {
    trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.starts_with("//")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{FileCategory, LanguageBackend, config_backend, markdown_backend};

    #[test]
    fn markdown_backend_counts_non_blank_lines_as_doc() {
        let root = temp_workspace("markdown_backend_counts_non_blank_lines_as_doc");
        let path = root.join("README.md");
        fs::write(path.as_std_path(), "# Title\n\nSome prose\nAnother line\n").unwrap();

        let analysis = markdown_backend()
            .classify_file(
                &path,
                FileCategory::Docs,
                &crate::ClassificationOptions::default(),
            )
            .unwrap();

        assert_eq!(analysis.metrics.doc_lines, 3);
        assert_eq!(analysis.metrics.blank_lines, 1);
        assert_eq!(analysis.metrics.code_lines, 0);

        cleanup_workspace(&root);
    }

    #[test]
    fn config_backend_counts_comments_and_entries() {
        let root = temp_workspace("config_backend_counts_comments_and_entries");
        let path = root.join("Cargo.toml");
        fs::write(
            path.as_std_path(),
            "# comment\n[package]\nname = \"demo\"\n",
        )
        .unwrap();

        let analysis = config_backend()
            .classify_file(
                &path,
                FileCategory::Config,
                &crate::ClassificationOptions::default(),
            )
            .unwrap();

        assert_eq!(analysis.metrics.comment_lines, 1);
        assert_eq!(analysis.metrics.code_lines, 2);
        assert_eq!(analysis.metrics.doc_lines, 0);

        cleanup_workspace(&root);
    }

    fn temp_workspace(test_name: &str) -> camino::Utf8PathBuf {
        let unique = format!(
            "rloc-text-backend-{test_name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        camino::Utf8PathBuf::from_path_buf(path).unwrap()
    }

    fn cleanup_workspace(root: &camino::Utf8Path) {
        if root.exists() {
            fs::remove_dir_all(root.as_std_path()).unwrap();
        }
    }
}
