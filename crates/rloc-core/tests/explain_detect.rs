use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use camino::{Utf8Path, Utf8PathBuf};
use rloc_core::{
    AnalysisWarning, Analyzer, BackendFileAnalysis, ClassificationOptions, FileCategory,
    FileMetrics, Language, LanguageBackend, LanguageBackendRegistry, LanguageDescriptor,
    LineExplanation, ScanOptions,
};

#[derive(Debug, Clone, Copy)]
struct FakeRustBackend;

#[derive(Debug, Clone, Copy)]
struct FakePythonBackend;

impl LanguageBackend for FakeRustBackend {
    fn descriptor(&self) -> LanguageDescriptor {
        LanguageDescriptor::new(Language::Rust, "Rust", &["rs"])
    }

    fn classify_file(
        &self,
        path: &Utf8Path,
        category: FileCategory,
        _options: &ClassificationOptions,
    ) -> Result<BackendFileAnalysis, String> {
        Ok(BackendFileAnalysis {
            metrics: FileMetrics::from_line_breakdown(
                path.to_path_buf(),
                Language::Rust,
                category,
                40,
                2,
                0,
                1,
                0,
                0,
                1,
                0,
            ),
            line_explanations: vec![
                LineExplanation {
                    line_number: 1,
                    kind: "code".to_owned(),
                    snippet: "fn main() {}".to_owned(),
                    reason: "backend marked the function line as code".to_owned(),
                },
                LineExplanation {
                    line_number: 2,
                    kind: "mixed".to_owned(),
                    snippet: "let x = 1; // note".to_owned(),
                    reason: "backend marked the inline comment line as mixed".to_owned(),
                },
            ],
            warnings: vec![AnalysisWarning::for_file(
                path.to_path_buf(),
                Language::Rust,
                "backend used fallback comment scanner",
            )],
        })
    }
}

impl LanguageBackend for FakePythonBackend {
    fn descriptor(&self) -> LanguageDescriptor {
        LanguageDescriptor::new(Language::Python, "Python", &["py"])
    }

    fn classify_file(
        &self,
        path: &Utf8Path,
        category: FileCategory,
        _options: &ClassificationOptions,
    ) -> Result<BackendFileAnalysis, String> {
        Ok(BackendFileAnalysis {
            metrics: FileMetrics::from_line_breakdown(
                path.to_path_buf(),
                Language::Python,
                category,
                24,
                2,
                0,
                2,
                0,
                0,
                0,
                0,
            ),
            line_explanations: vec![LineExplanation {
                line_number: 1,
                kind: "code".to_owned(),
                snippet: "def test_ok():".to_owned(),
                reason: "backend marked the function definition as code".to_owned(),
            }],
            warnings: Vec::new(),
        })
    }
}

#[test]
fn detect_reports_languages_categories_and_ignore_rules() {
    let root = temp_workspace("detect_reports_languages_categories_and_ignore_rules");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "tests/test_app.py", "def test_ok():\n    pass\n");

    let analyzer = Analyzer::new(
        LanguageBackendRegistry::new()
            .with_backend(FakeRustBackend)
            .with_backend(FakePythonBackend),
    );
    let report = analyzer.detect(&root, &ScanOptions::default()).unwrap();

    assert_eq!(report.languages, vec![Language::Python, Language::Rust]);
    assert_eq!(
        report.categories,
        vec![FileCategory::Source, FileCategory::Test]
    );
    assert!(
        report
            .active_ignore_rules
            .iter()
            .any(|rule| rule == ".gitignore")
    );
    assert!(report.warnings.is_empty());

    cleanup_workspace(&root);
}

#[test]
fn explain_includes_line_level_breakdown_and_warnings() {
    let root = temp_workspace("explain_includes_line_level_breakdown_and_warnings");
    let file = root.join("src/lib.rs");
    write_file(&root, "src/lib.rs", "fn main() {}\nlet x = 1; // note\n");

    let analyzer = Analyzer::new(LanguageBackendRegistry::new().with_backend(FakeRustBackend));
    let report = analyzer.explain(&file).unwrap();

    assert_eq!(report.language, Language::Rust);
    assert_eq!(report.category, FileCategory::Source);
    assert_eq!(report.metrics.mixed_lines, 1);
    assert_eq!(report.line_explanations.len(), 2);
    assert_eq!(report.line_explanations[1].kind, "mixed");
    assert!(
        report.line_explanations[1]
            .reason
            .contains("inline comment")
    );
    assert_eq!(report.warnings.len(), 1);
    assert!(
        report.warnings[0]
            .message
            .contains("fallback comment scanner")
    );

    cleanup_workspace(&root);
}

fn temp_workspace(test_name: &str) -> Utf8PathBuf {
    let unique = format!(
        "rloc-core-{test_name}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let path = std::env::temp_dir().join(unique);
    fs::create_dir_all(&path).unwrap();
    Utf8PathBuf::from_path_buf(path).unwrap()
}

fn write_file(root: &Utf8Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent.as_std_path()).unwrap();
    }
    fs::write(path.as_std_path(), contents).unwrap();
}

fn cleanup_workspace(root: &Utf8Path) {
    if root.exists() {
        fs::remove_dir_all(root.as_std_path()).unwrap();
    }
}
