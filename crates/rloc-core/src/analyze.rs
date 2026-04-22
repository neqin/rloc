use std::{collections::BTreeMap, collections::BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::{
    categories, discover, filters,
    metrics::{FileMetrics, MetricsSummary, ScanReport},
    registry::LanguageBackendRegistry,
    types::{AnalysisWarning, FileCategory, Language, LineExplanation, ScanOptions},
};

#[derive(Debug, Clone)]
pub struct Analyzer {
    registry: LanguageBackendRegistry,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectReport {
    pub path: Utf8PathBuf,
    pub languages: Vec<Language>,
    pub presets: Vec<String>,
    pub categories: Vec<FileCategory>,
    pub active_ignore_rules: Vec<String>,
    pub warnings: Vec<AnalysisWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplainReport {
    pub path: Utf8PathBuf,
    pub language: Language,
    pub category: FileCategory,
    pub metrics: FileMetrics,
    pub reasons: Vec<String>,
    pub line_explanations: Vec<LineExplanation>,
    pub warnings: Vec<AnalysisWarning>,
}

impl Analyzer {
    pub fn new(registry: LanguageBackendRegistry) -> Self {
        Self { registry }
    }

    pub fn scan(&self, root: &Utf8Path, options: &ScanOptions) -> Result<ScanReport, String> {
        let discovery = discover::discover_candidate_files(root, options)?;
        let files = discovery.files;
        let mut entries = Vec::with_capacity(files.len());
        let mut warnings = discovery.warnings;
        let mut unsupported_extensions = BTreeMap::<String, usize>::new();
        let mut unsupported_samples = Vec::new();
        let mut summary = MetricsSummary::default();
        let mut by_language = BTreeMap::<Language, MetricsSummary>::new();
        let classification = options.classification.clone();

        for path in files {
            let language = self.registry.detect_language(&path);
            if matches!(language, Language::Unknown) {
                record_unsupported_extension(&mut unsupported_extensions, &path);
                record_unsupported_sample(&mut unsupported_samples, options, &path);
                continue;
            }
            if !options.allows_language(language) {
                continue;
            }

            let category = categories::analyze_category_with_options(&path, options).category;
            if options.exclude_category(category) {
                continue;
            }

            let Some(backend) = self.registry.backend(language) else {
                warnings.push(AnalysisWarning::for_file(
                    path,
                    language,
                    "no backend registered for detected language",
                ));
                continue;
            };

            let analysis = match backend.classify_file(&path, category, &classification) {
                Ok(analysis) => analysis,
                Err(error) => {
                    warnings.push(AnalysisWarning::for_file(
                        path,
                        language,
                        format!("failed to analyze file: {error}"),
                    ));
                    continue;
                }
            };
            let metrics = analysis.metrics;
            summary.add_file(&metrics);
            by_language.entry(language).or_default().add_file(&metrics);
            warnings.extend(analysis.warnings);
            entries.push(metrics);
        }

        if let Some(warning) = unsupported_extension_warning(&unsupported_extensions) {
            warnings.push(warning);
        }
        warnings.extend(unsupported_samples);

        Ok(ScanReport {
            summary,
            files: entries,
            by_language,
            warnings,
        })
    }

    pub fn detect(&self, root: &Utf8Path, options: &ScanOptions) -> Result<DetectReport, String> {
        let discovery = discover::discover_candidate_files(root, options)?;
        let files = discovery.files;
        let mut languages = BTreeSet::new();
        let mut categories_found = BTreeSet::new();
        let mut warnings = discovery.warnings;
        let mut unsupported_extensions = BTreeMap::<String, usize>::new();
        let mut unsupported_samples = Vec::new();

        for path in files {
            let language = self.registry.detect_language(&path);
            if matches!(language, Language::Unknown) {
                record_unsupported_extension(&mut unsupported_extensions, &path);
                record_unsupported_sample(&mut unsupported_samples, options, &path);
                continue;
            }
            if !options.allows_language(language) {
                continue;
            }

            let category = categories::analyze_category_with_options(&path, options).category;
            if options.exclude_category(category) {
                continue;
            }

            languages.insert(language);
            categories_found.insert(category);
        }

        if let Some(warning) = unsupported_extension_warning(&unsupported_extensions) {
            warnings.push(warning);
        }
        warnings.extend(unsupported_samples);

        Ok(DetectReport {
            path: root.to_path_buf(),
            languages: sort_languages(languages),
            presets: Vec::new(),
            categories: sort_categories(categories_found),
            active_ignore_rules: filters::active_ignore_rules(options),
            warnings,
        })
    }

    pub fn explain(&self, path: &Utf8Path) -> Result<ExplainReport, String> {
        self.explain_with_options(path, &ScanOptions::default())
    }

    pub fn explain_with_options(
        &self,
        path: &Utf8Path,
        options: &ScanOptions,
    ) -> Result<ExplainReport, String> {
        if !path.is_file() {
            return Err(format!("{} is not a readable file", path));
        }

        let language = self.registry.detect_language(path);
        if matches!(language, Language::Unknown) {
            return Err(format!("{} is not a supported source file", path));
        }

        let category = categories::analyze_category_with_options(path, options);
        let backend = self
            .registry
            .backend(language)
            .ok_or_else(|| format!("no backend registered for {language}"))?;
        let analysis = backend.classify_file(path, category.category, &options.classification)?;
        let metrics = analysis.metrics;
        let reasons = explain_reasons(path, language, &category.reasons);
        let line_explanations = analysis.line_explanations;
        let warnings = analysis.warnings;

        Ok(ExplainReport {
            path: path.to_path_buf(),
            language,
            category: category.category,
            metrics,
            reasons,
            line_explanations,
            warnings,
        })
    }

    pub fn registry(&self) -> &LanguageBackendRegistry {
        &self.registry
    }
}

fn sort_languages(items: BTreeSet<Language>) -> Vec<Language> {
    let mut values = items.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|language| language.as_str());
    values
}

fn sort_categories(items: BTreeSet<FileCategory>) -> Vec<FileCategory> {
    let mut values = items.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|category| category.as_str());
    values
}

fn explain_reasons(
    path: &Utf8Path,
    language: Language,
    category_reasons: &[String],
) -> Vec<String> {
    let language_reason = match path.extension() {
        Some(extension) => format!("language detected as {language} from .{extension} extension"),
        None => format!("language detected as {language} without a file extension"),
    };
    let mut reasons = Vec::with_capacity(category_reasons.len() + 1);
    reasons.push(language_reason);
    reasons.extend(category_reasons.iter().cloned());
    reasons
}

fn record_unsupported_extension(counts: &mut BTreeMap<String, usize>, path: &Utf8Path) {
    let bucket = match path.extension() {
        Some(extension) => format!(".{extension}"),
        None => "no extension".to_owned(),
    };
    *counts.entry(bucket).or_default() += 1;
}

fn record_unsupported_sample(
    samples: &mut Vec<AnalysisWarning>,
    options: &ScanOptions,
    path: &Utf8Path,
) {
    let Some(limit) = options.unsupported_sample_limit else {
        return;
    };
    if samples.len() >= limit {
        return;
    }

    samples.push(AnalysisWarning::for_path(
        path.to_path_buf(),
        "unsupported extension skipped",
    ));
}

fn unsupported_extension_warning(counts: &BTreeMap<String, usize>) -> Option<AnalysisWarning> {
    let count = counts.values().sum::<usize>();
    if count == 0 {
        return None;
    }

    let breakdown = unsupported_extension_breakdown(counts);
    let message = match count {
        1 => format!("1 file with an unsupported extension was skipped ({breakdown})"),
        count => format!("{count} files with unsupported extensions were skipped ({breakdown})"),
    };

    Some(AnalysisWarning::new(message))
}

fn unsupported_extension_breakdown(counts: &BTreeMap<String, usize>) -> String {
    let mut items = counts
        .iter()
        .map(|(bucket, count)| (bucket.as_str(), *count))
        .collect::<Vec<_>>();
    items.sort_by(|(left_bucket, left_count), (right_bucket, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_bucket.cmp(right_bucket))
    });

    items
        .into_iter()
        .map(|(bucket, count)| format!("{bucket}: {count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use camino::{Utf8Path, Utf8PathBuf};

    use super::{Analyzer, DetectReport, ExplainReport};
    use crate::{
        BackendFileAnalysis, ClassificationOptions, FileCategory, FileMetrics, Language,
        LanguageBackend, LanguageBackendRegistry, LanguageDescriptor, LineExplanation, ScanOptions,
        metrics::ScanReport,
    };

    #[derive(Debug, Clone, Copy)]
    struct TestRustBackend;

    impl LanguageBackend for TestRustBackend {
        fn descriptor(&self) -> LanguageDescriptor {
            LanguageDescriptor::new(Language::Rust, "Rust", &["rs"])
        }

        fn classify_file(
            &self,
            path: &Utf8Path,
            category: FileCategory,
            _options: &ClassificationOptions,
        ) -> Result<BackendFileAnalysis, String> {
            let bytes = fs::read(path.as_std_path()).map_err(|error| error.to_string())?;
            let contents = String::from_utf8_lossy(&bytes);
            let total_lines = contents.lines().count() as u32;
            let blank_lines = contents
                .lines()
                .filter(|line| line.trim().is_empty())
                .count() as u32;
            let code_lines = total_lines - blank_lines;

            Ok(BackendFileAnalysis {
                metrics: FileMetrics::from_line_breakdown(
                    path.to_path_buf(),
                    Language::Rust,
                    category,
                    bytes.len() as u64,
                    total_lines,
                    blank_lines,
                    code_lines,
                    0,
                    0,
                    0,
                    0,
                ),
                line_explanations: vec![LineExplanation {
                    line_number: 1,
                    kind: "code".to_owned(),
                    snippet: "fn main() {}".to_owned(),
                    reason: "test backend treats non-empty lines as code".to_owned(),
                }],
                warnings: Vec::new(),
            })
        }
    }

    #[test]
    fn scan_populates_physical_metrics_for_supported_files() {
        let root = temp_workspace("scan_populates_physical_metrics_for_supported_files");
        write_file(&root, "src/lib.rs", "fn main() {}\n\n");
        write_file(&root, "README.md", "ignored\n");

        let analyzer = Analyzer::new(test_registry());
        let report = analyzer.scan(&root, &ScanOptions::default()).unwrap();

        assert_scan_report(&report);
        assert_eq!(report.summary.files, 1);
        assert_eq!(report.summary.lines, 2);
        assert_eq!(report.summary.code, 1);
        assert_eq!(report.summary.blank, 1);
        assert_eq!(report.by_language.len(), 1);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(
            report.warnings[0].message,
            "1 file with an unsupported extension was skipped (.md: 1)"
        );
        assert!(report.warnings[0].path.is_none());
        assert!(report.warnings[0].language.is_none());

        cleanup_workspace(&root);
    }

    #[test]
    fn scan_aggregates_unsupported_extension_warnings() {
        let root = temp_workspace("scan_aggregates_unsupported_extension_warnings");
        write_file(&root, "src/lib.rs", "fn main() {}\n");
        write_file(&root, "README.md", "ignored\n");
        write_file(&root, "Cargo.toml", "[package]\nname = \"fixture\"\n");

        let analyzer = Analyzer::new(test_registry());
        let report = analyzer.scan(&root, &ScanOptions::default()).unwrap();

        assert_eq!(report.summary.files, 1);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(
            report.warnings[0].message,
            "2 files with unsupported extensions were skipped (.md: 1, .toml: 1)"
        );
        assert!(report.warnings[0].path.is_none());
        assert!(report.warnings[0].language.is_none());

        cleanup_workspace(&root);
    }

    #[test]
    fn detect_summarizes_languages_categories_and_ignore_rules() {
        let root = temp_workspace("detect_summarizes_languages_categories_and_ignore_rules");
        write_file(&root, "src/lib.rs", "fn main() {}\n");
        write_file(&root, "tests/test_app.py", "def test_ok():\n    pass\n");

        let analyzer = Analyzer::new(test_registry());
        let report = analyzer.detect(&root, &ScanOptions::default()).unwrap();

        assert_detect_report(&report);
        assert_eq!(report.languages, vec![Language::Python, Language::Rust]);
        assert_eq!(report.categories.len(), 2);
        assert!(
            report
                .active_ignore_rules
                .iter()
                .any(|rule| rule == ".gitignore")
        );

        cleanup_workspace(&root);
    }

    #[test]
    fn detect_aggregates_unsupported_extension_warnings() {
        let root = temp_workspace("detect_aggregates_unsupported_extension_warnings");
        write_file(&root, "src/lib.rs", "fn main() {}\n");
        write_file(&root, "README.md", "ignored\n");
        write_file(&root, "Cargo.toml", "[package]\nname = \"fixture\"\n");

        let analyzer = Analyzer::new(test_registry());
        let report = analyzer.detect(&root, &ScanOptions::default()).unwrap();

        assert_eq!(report.languages, vec![Language::Rust]);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(
            report.warnings[0].message,
            "2 files with unsupported extensions were skipped (.md: 1, .toml: 1)"
        );
        assert!(report.warnings[0].path.is_none());
        assert!(report.warnings[0].language.is_none());

        cleanup_workspace(&root);
    }

    #[test]
    fn scan_adds_sample_unsupported_path_warnings_when_requested() {
        let root = temp_workspace("scan_adds_sample_unsupported_path_warnings_when_requested");
        write_file(&root, "src/lib.rs", "fn main() {}\n");
        write_file(&root, "README.txt", "ignored\n");
        write_file(&root, "notes.log", "ignored\n");

        let report = Analyzer::new(test_registry())
            .scan(
                &root,
                &ScanOptions {
                    unsupported_sample_limit: Some(1),
                    ..ScanOptions::default()
                },
            )
            .unwrap();

        assert_eq!(report.warnings.len(), 2);
        assert_eq!(
            report.warnings[0].message,
            "2 files with unsupported extensions were skipped (.log: 1, .txt: 1)"
        );
        assert_eq!(report.warnings[1].message, "unsupported extension skipped");
        assert_eq!(report.warnings[1].path, Some(root.join("README.txt")));

        cleanup_workspace(&root);
    }

    #[test]
    fn detect_adds_sample_unsupported_path_warnings_when_requested() {
        let root = temp_workspace("detect_adds_sample_unsupported_path_warnings_when_requested");
        write_file(&root, "src/lib.rs", "fn main() {}\n");
        write_file(&root, "README.txt", "ignored\n");
        write_file(&root, "notes.log", "ignored\n");

        let report = Analyzer::new(test_registry())
            .detect(
                &root,
                &ScanOptions {
                    unsupported_sample_limit: Some(2),
                    ..ScanOptions::default()
                },
            )
            .unwrap();

        assert_eq!(report.warnings.len(), 3);
        assert_eq!(
            report.warnings[0].message,
            "2 files with unsupported extensions were skipped (.log: 1, .txt: 1)"
        );
        assert_eq!(report.warnings[1].path, Some(root.join("README.txt")));
        assert_eq!(report.warnings[2].path, Some(root.join("notes.log")));

        cleanup_workspace(&root);
    }

    #[test]
    fn scan_reports_missing_extensions_in_unsupported_warning() {
        let root = temp_workspace("scan_reports_missing_extensions_in_unsupported_warning");
        write_file(&root, "src/lib.rs", "fn main() {}\n");
        write_file(&root, "README", "ignored\n");

        let analyzer = Analyzer::new(test_registry());
        let report = analyzer.scan(&root, &ScanOptions::default()).unwrap();

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(
            report.warnings[0].message,
            "1 file with an unsupported extension was skipped (no extension: 1)"
        );
        assert!(report.warnings[0].path.is_none());
        assert!(report.warnings[0].language.is_none());

        cleanup_workspace(&root);
    }

    #[test]
    fn explain_reports_reasons_for_supported_files() {
        let root = temp_workspace("explain_reports_reasons_for_supported_files");
        write_file(&root, "tools/build.rs", "fn main() {}\n");

        let analyzer = Analyzer::new(test_registry());
        let report = analyzer.explain(&root.join("tools/build.rs")).unwrap();

        assert_explain_report(&report);
        assert_eq!(report.language, Language::Rust);
        assert_eq!(report.category.as_str(), "script");
        assert_eq!(report.reasons.len(), 2);

        cleanup_workspace(&root);
    }

    fn test_registry() -> LanguageBackendRegistry {
        LanguageBackendRegistry::new()
            .with_backend(TestRustBackend)
            .with_descriptor(LanguageDescriptor::new(Language::Python, "Python", &["py"]))
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

    fn assert_scan_report(report: &ScanReport) {
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].language, Language::Rust);
        assert_eq!(report.files[0].total_lines, 2);
        assert_eq!(report.files[0].code_lines, 1);
        assert_eq!(report.files[0].blank_lines, 1);
    }

    fn assert_detect_report(report: &DetectReport) {
        assert!(report.presets.is_empty());
        assert!(report.warnings.is_empty());
    }

    fn assert_explain_report(report: &ExplainReport) {
        assert!(report.reasons[0].contains("language detected as rust"));
        assert_eq!(report.metrics.total_lines, 1);
        assert_eq!(report.metrics.code_lines, 1);
        assert_eq!(report.line_explanations.len(), 1);
        assert!(report.warnings.is_empty());
    }
}
