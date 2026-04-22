use std::{
    fs,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use camino::{Utf8Path, Utf8PathBuf};
use rloc_core::{
    AnalysisWarning, Analyzer, BackendFileAnalysis, ClassificationOptions, FileCategory,
    FileMetrics, Language, LanguageBackend, LanguageBackendRegistry, LanguageDescriptor,
    LineExplanation, ScanOptions,
};

#[derive(Debug, Default)]
struct FakeRustBackend {
    calls: Arc<Mutex<Vec<Utf8PathBuf>>>,
}

impl FakeRustBackend {
    fn with_calls(calls: Arc<Mutex<Vec<Utf8PathBuf>>>) -> Self {
        Self { calls }
    }
}

impl LanguageBackend for FakeRustBackend {
    fn descriptor(&self) -> LanguageDescriptor {
        LanguageDescriptor::new(Language::Rust, "Rust", &["rs"])
    }

    fn classify_file(
        &self,
        path: &Utf8Path,
        category: FileCategory,
        options: &ClassificationOptions,
    ) -> Result<BackendFileAnalysis, String> {
        self.calls.lock().unwrap().push(path.to_path_buf());

        Ok(BackendFileAnalysis {
            metrics: FileMetrics::from_line_breakdown(
                path.to_path_buf(),
                Language::Rust,
                category,
                42,
                5,
                1,
                3,
                0,
                u32::from(options.count_doc_comments),
                1,
                0,
            ),
            line_explanations: vec![LineExplanation {
                line_number: 2,
                kind: "mixed".to_owned(),
                snippet: "let x = 1; // note".to_owned(),
                reason: "fake backend reported an inline comment".to_owned(),
            }],
            warnings: vec![AnalysisWarning::for_language(
                Language::Rust,
                "fake backend warning",
            )],
        })
    }
}

#[test]
fn analyzer_uses_registered_backend_metrics() {
    let root = temp_workspace("analyzer_uses_registered_backend_metrics");
    let file = root.join("src/lib.rs");
    write_file(&root, "src/lib.rs", "fn main() {}\n");

    let calls = Arc::new(Mutex::new(Vec::new()));
    let registry =
        LanguageBackendRegistry::new().with_backend(FakeRustBackend::with_calls(calls.clone()));

    let analyzer = Analyzer::new(registry);
    let report = analyzer.scan(&root, &ScanOptions::default()).unwrap();

    assert_eq!(report.summary.files, 1);
    assert_eq!(report.summary.code, 3);
    assert_eq!(report.summary.doc, 1);
    assert_eq!(report.summary.mixed, 1);
    assert_eq!(report.files[0].bytes, 42);
    assert_eq!(report.files[0].category, FileCategory::Source);
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(report.warnings[0].message, "fake backend warning");
    assert_eq!(calls.lock().unwrap().as_slice(), &[file]);

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
