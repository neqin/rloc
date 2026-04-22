use std::fmt;

use camino::Utf8PathBuf;
use serde::{Serialize, Serializer};

use crate::{Utf8Path, metrics::FileMetrics, registry::LanguageDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Language {
    Css,
    Go,
    Html,
    Rust,
    Shell,
    Sql,
    Python,
    JavaScript,
    TypeScript,
    Jsx,
    Tsx,
    Markdown,
    Config,
    Unknown,
}

impl Language {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Css => "css",
            Self::Go => "go",
            Self::Html => "html",
            Self::Rust => "rust",
            Self::Shell => "shell",
            Self::Sql => "sql",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Jsx => "jsx",
            Self::Tsx => "tsx",
            Self::Markdown => "markdown",
            Self::Config => "config",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    Source,
    Test,
    Example,
    Bench,
    Script,
    Docs,
    Config,
    Generated,
    Vendor,
    Unknown,
}

impl FileCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Test => "test",
            Self::Example => "example",
            Self::Bench => "bench",
            Self::Script => "script",
            Self::Docs => "docs",
            Self::Config => "config",
            Self::Generated => "generated",
            Self::Vendor => "vendor",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for FileCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisWarning {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
}

impl AnalysisWarning {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: None,
            language: None,
        }
    }

    pub fn for_path(path: Utf8PathBuf, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: Some(path),
            language: None,
        }
    }

    pub fn for_language(language: Language, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: None,
            language: Some(language),
        }
    }

    pub fn for_file(path: Utf8PathBuf, language: Language, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: Some(path),
            language: Some(language),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationOptions {
    pub count_doc_comments: bool,
    pub count_docstrings_as_comments: bool,
    pub mixed_lines_as_code: bool,
}

impl Default for ClassificationOptions {
    fn default() -> Self {
        Self {
            count_doc_comments: true,
            count_docstrings_as_comments: true,
            mixed_lines_as_code: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LineExplanation {
    pub line_number: u32,
    pub kind: String,
    pub snippet: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendFileAnalysis {
    pub metrics: FileMetrics,
    pub line_explanations: Vec<LineExplanation>,
    pub warnings: Vec<AnalysisWarning>,
}

pub trait LanguageBackend: Send + Sync {
    fn descriptor(&self) -> LanguageDescriptor;

    fn classify_file(
        &self,
        path: &Utf8Path,
        category: FileCategory,
        options: &ClassificationOptions,
    ) -> Result<BackendFileAnalysis, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOptions {
    pub languages: Option<Vec<Language>>,
    pub hidden: bool,
    pub respect_gitignore: bool,
    pub include_tests: bool,
    pub include_generated: bool,
    pub include_vendor: bool,
    pub exclude_patterns: Vec<String>,
    pub generated_patterns: Vec<String>,
    pub vendor_patterns: Vec<String>,
    pub unsupported_sample_limit: Option<usize>,
    pub classification: ClassificationOptions,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            languages: None,
            hidden: false,
            respect_gitignore: true,
            include_tests: true,
            include_generated: false,
            include_vendor: false,
            exclude_patterns: Vec::new(),
            generated_patterns: Vec::new(),
            vendor_patterns: Vec::new(),
            unsupported_sample_limit: None,
            classification: ClassificationOptions::default(),
        }
    }
}

impl ScanOptions {
    pub fn allows_language(&self, language: Language) -> bool {
        match &self.languages {
            Some(allowed) => allowed.contains(&language),
            None => !matches!(language, Language::Unknown),
        }
    }

    pub fn exclude_category(&self, category: FileCategory) -> bool {
        matches!(category, FileCategory::Test) && !self.include_tests
            || matches!(category, FileCategory::Generated) && !self.include_generated
            || matches!(category, FileCategory::Vendor) && !self.include_vendor
    }
}

#[cfg(test)]
mod tests {
    use super::ScanOptions;

    #[test]
    fn scan_options_default_excludes_generated_files() {
        let options = ScanOptions::default();

        assert!(!options.include_generated);
    }
}
