use std::{collections::HashMap, fmt, sync::Arc};

use camino::Utf8Path;

use crate::types::{Language, LanguageBackend};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageDescriptor {
    pub language: Language,
    pub display_name: &'static str,
    pub extensions: &'static [&'static str],
    pub file_names: &'static [&'static str],
}

impl LanguageDescriptor {
    pub const fn new(
        language: Language,
        display_name: &'static str,
        extensions: &'static [&'static str],
    ) -> Self {
        Self::with_file_names(language, display_name, extensions, &[])
    }

    pub const fn with_file_names(
        language: Language,
        display_name: &'static str,
        extensions: &'static [&'static str],
        file_names: &'static [&'static str],
    ) -> Self {
        Self {
            language,
            display_name,
            extensions,
            file_names,
        }
    }
}

#[derive(Clone, Default)]
pub struct LanguageBackendRegistry {
    descriptors: Vec<LanguageDescriptor>,
    by_extension: HashMap<&'static str, Language>,
    by_file_name: HashMap<&'static str, Language>,
    backends: HashMap<Language, Arc<dyn LanguageBackend>>,
}

impl fmt::Debug for LanguageBackendRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LanguageBackendRegistry")
            .field("descriptors", &self.descriptors)
            .field("supported_languages", &self.supported_languages())
            .finish()
    }
}

impl LanguageBackendRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_descriptor(mut self, descriptor: LanguageDescriptor) -> Self {
        self.register_descriptor(descriptor);
        self
    }

    pub fn extend<I>(mut self, descriptors: I) -> Self
    where
        I: IntoIterator<Item = LanguageDescriptor>,
    {
        for descriptor in descriptors {
            self.register_descriptor(descriptor);
        }
        self
    }

    pub fn with_backend<B>(mut self, backend: B) -> Self
    where
        B: LanguageBackend + 'static,
    {
        self.register_backend(backend);
        self
    }

    pub fn register_backend<B>(&mut self, backend: B)
    where
        B: LanguageBackend + 'static,
    {
        let descriptor = backend.descriptor();
        self.backends.insert(descriptor.language, Arc::new(backend));
        self.register_descriptor(descriptor);
    }

    fn register_descriptor(&mut self, descriptor: LanguageDescriptor) {
        if let Some(index) = self
            .descriptors
            .iter()
            .position(|candidate| candidate.language == descriptor.language)
        {
            let previous = self.descriptors.remove(index);
            for extension in previous.extensions {
                self.by_extension.remove(extension);
            }
            for file_name in previous.file_names {
                self.by_file_name.remove(file_name);
            }
        }

        for extension in descriptor.extensions {
            self.by_extension.insert(*extension, descriptor.language);
        }
        for file_name in descriptor.file_names {
            self.by_file_name.insert(*file_name, descriptor.language);
        }

        self.descriptors.push(descriptor);
        self.descriptors
            .sort_by_key(|candidate| candidate.language.as_str());
    }

    pub fn backend(&self, language: Language) -> Option<&dyn LanguageBackend> {
        self.backends.get(&language).map(Arc::as_ref)
    }

    pub fn descriptors(&self) -> &[LanguageDescriptor] {
        &self.descriptors
    }

    pub fn supported_languages(&self) -> Vec<Language> {
        self.descriptors
            .iter()
            .map(|descriptor| descriptor.language)
            .collect()
    }

    pub fn detect_language(&self, path: &Utf8Path) -> Language {
        if let Some(language) = path
            .file_name()
            .and_then(|file_name| self.by_file_name.get(file_name).copied())
        {
            return language;
        }

        path.extension()
            .and_then(|extension| self.by_extension.get(extension).copied())
            .unwrap_or(Language::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use camino::Utf8Path;

    use super::{LanguageBackendRegistry, LanguageDescriptor};
    use crate::{
        BackendFileAnalysis, ClassificationOptions, FileCategory, FileMetrics, Language,
        LanguageBackend, LineBreakdown, LineExplanation,
    };

    #[derive(Debug, Clone, Copy)]
    struct FakeRustBackend;

    impl LanguageBackend for FakeRustBackend {
        fn descriptor(&self) -> LanguageDescriptor {
            LanguageDescriptor::new(Language::Rust, "Rust", &["rs"])
        }

        fn classify_file(
            &self,
            path: &crate::Utf8Path,
            category: FileCategory,
            _options: &ClassificationOptions,
        ) -> Result<BackendFileAnalysis, String> {
            Ok(BackendFileAnalysis {
                metrics: FileMetrics::from_line_breakdown(
                    path.to_path_buf(),
                    Language::Rust,
                    category,
                    1,
                    LineBreakdown {
                        total: 1,
                        code: 1,
                        ..LineBreakdown::default()
                    },
                ),
                line_explanations: vec![LineExplanation {
                    line_number: 1,
                    kind: "code".to_owned(),
                    snippet: "fn main() {}".to_owned(),
                    reason: "fake backend classified a code line".to_owned(),
                }],
                warnings: Vec::new(),
            })
        }
    }

    #[test]
    fn registry_detects_registered_extensions() {
        let registry = LanguageBackendRegistry::new()
            .with_descriptor(LanguageDescriptor::new(Language::Rust, "Rust", &["rs"]))
            .with_descriptor(LanguageDescriptor::new(Language::Python, "Python", &["py"]))
            .with_descriptor(LanguageDescriptor::new(
                Language::Markdown,
                "Markdown",
                &["md"],
            ))
            .with_descriptor(LanguageDescriptor::new(
                Language::Config,
                "Config",
                &[
                    "toml", "yaml", "yml", "json", "jsonc", "lock", "ini", "cfg", "conf",
                ],
            ));

        assert_eq!(
            registry.detect_language(Utf8Path::new("src/lib.rs")),
            Language::Rust
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("tests/test_app.py")),
            Language::Python
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("README.md")),
            Language::Markdown
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("Cargo.toml")),
            Language::Config
        );
    }

    #[test]
    fn registry_detects_shell_dotfiles_and_wave1_extensions() {
        let registry = LanguageBackendRegistry::new()
            .with_descriptor(LanguageDescriptor::with_file_names(
                Language::Shell,
                "Shell",
                &["sh", "bash", "zsh"],
                &[".bashrc", ".zshrc", ".envrc"],
            ))
            .with_descriptor(LanguageDescriptor::new(
                Language::Sql,
                "SQL",
                &["sql", "psql"],
            ))
            .with_descriptor(LanguageDescriptor::new(Language::Go, "Go", &["go"]))
            .with_descriptor(LanguageDescriptor::new(
                Language::Html,
                "HTML",
                &["html", "htm", "xhtml", "gohtml"],
            ))
            .with_descriptor(LanguageDescriptor::new(Language::Css, "CSS", &["css"]));

        assert_eq!(
            registry.detect_language(Utf8Path::new("scripts/build.sh")),
            Language::Shell
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("scripts/build.bash")),
            Language::Shell
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("scripts/build.zsh")),
            Language::Shell
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new(".bashrc")),
            Language::Shell
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new(".zshrc")),
            Language::Shell
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new(".envrc")),
            Language::Shell
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("db/query.sql")),
            Language::Sql
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("db/query.psql")),
            Language::Sql
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("cmd/main.go")),
            Language::Go
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("web/index.html")),
            Language::Html
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("web/index.htm")),
            Language::Html
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("web/layout.xhtml")),
            Language::Html
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("web/page.gohtml")),
            Language::Html
        );
        assert_eq!(
            registry.detect_language(Utf8Path::new("web/app.css")),
            Language::Css
        );
    }

    #[test]
    fn supported_languages_follow_registered_descriptors() {
        let registry = LanguageBackendRegistry::new()
            .with_descriptor(LanguageDescriptor::new(Language::Rust, "Rust", &["rs"]))
            .with_descriptor(LanguageDescriptor::new(
                Language::TypeScript,
                "TypeScript",
                &["ts"],
            ))
            .with_descriptor(LanguageDescriptor::with_file_names(
                Language::Shell,
                "Shell",
                &["sh"],
                &[".bashrc"],
            ))
            .with_descriptor(LanguageDescriptor::new(Language::Sql, "SQL", &["sql"]))
            .with_descriptor(LanguageDescriptor::new(Language::Go, "Go", &["go"]))
            .with_descriptor(LanguageDescriptor::new(Language::Html, "HTML", &["html"]))
            .with_descriptor(LanguageDescriptor::new(Language::Css, "CSS", &["css"]));

        assert_eq!(
            registry.supported_languages(),
            vec![
                Language::Css,
                Language::Go,
                Language::Html,
                Language::Rust,
                Language::Shell,
                Language::Sql,
                Language::TypeScript,
            ]
        );
    }

    #[test]
    fn registry_returns_registered_backend() {
        let registry = LanguageBackendRegistry::new().with_backend(FakeRustBackend);

        assert!(registry.backend(Language::Rust).is_some());
        assert!(registry.backend(Language::Python).is_none());
        assert_eq!(
            registry.detect_language(Utf8Path::new("src/lib.rs")),
            Language::Rust
        );
    }
}
