pub mod json;
pub mod summary;
pub mod table;
pub mod top;

use std::fmt::Write as _;

use serde::Serialize;

use rloc_core::{
    AnalysisWarning, DetectReport, ExplainReport, FileCategory, FileMetrics, Language,
    MetricsSummary, ScanReport, Utf8PathBuf,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanGroupBy {
    Language,
    Category,
    Dir,
    File,
}

impl ScanGroupBy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Language => "language",
            Self::Category => "category",
            Self::Dir => "dir",
            Self::File => "file",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRenderOptions {
    pub path: Utf8PathBuf,
    pub format: String,
    pub group_by: Vec<ScanGroupBy>,
    pub top_files: Option<usize>,
    pub top_dirs: Option<usize>,
    pub respect_gitignore: bool,
    pub include_generated: bool,
    pub include_vendor: bool,
    pub include_tests: bool,
}

impl Default for ScanRenderOptions {
    fn default() -> Self {
        Self {
            path: Utf8PathBuf::from("."),
            format: "table".to_owned(),
            group_by: vec![ScanGroupBy::Language],
            top_files: Some(10),
            top_dirs: Some(10),
            respect_gitignore: true,
            include_generated: false,
            include_vendor: false,
            include_tests: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GroupSummary {
    pub group_by: String,
    pub key: String,
    pub files: usize,
    pub lines: u64,
    pub code: u64,
    pub mixed: u64,
    pub comment: u64,
    pub doc: u64,
    pub blank: u64,
    pub sloc: u64,
    pub bytes: u64,
    pub parse_errors: u64,
}

impl GroupSummary {
    pub fn from_metrics(group_by: ScanGroupBy, key: String, summary: &MetricsSummary) -> Self {
        Self {
            group_by: group_by.as_str().to_owned(),
            key,
            files: summary.files,
            lines: summary.lines,
            code: summary.code,
            mixed: summary.mixed,
            comment: summary.comment,
            doc: summary.doc,
            blank: summary.blank,
            sloc: summary.sloc,
            bytes: summary.bytes,
            parse_errors: summary.parse_errors,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TopFileEntry {
    pub path: Utf8PathBuf,
    pub language: Language,
    pub category: FileCategory,
    pub files: usize,
    pub lines: u64,
    pub code: u64,
    pub mixed: u64,
    pub comment: u64,
    pub doc: u64,
    pub blank: u64,
    pub sloc: u64,
    pub bytes: u64,
    pub parse_errors: u64,
}

impl From<&FileMetrics> for TopFileEntry {
    fn from(file: &FileMetrics) -> Self {
        Self {
            path: file.path.clone(),
            language: file.language,
            category: file.category,
            files: 1,
            lines: u64::from(file.total_lines),
            code: u64::from(file.code_lines),
            mixed: u64::from(file.mixed_lines),
            comment: u64::from(file.comment_lines),
            doc: u64::from(file.doc_lines),
            blank: u64::from(file.blank_lines),
            sloc: u64::from(file.sloc()),
            bytes: file.bytes,
            parse_errors: u64::from(file.parse_errors),
        }
    }
}

pub fn render_table(report: &ScanReport) -> String {
    render_table_with_options(report, &ScanRenderOptions::default())
}

pub fn render_human_summary(report: &ScanReport) -> String {
    render_table(report)
}

pub fn render_json(report: &ScanReport) -> Result<String, serde_json::Error> {
    render_json_with_options(
        report,
        &ScanRenderOptions {
            format: "json".to_owned(),
            ..ScanRenderOptions::default()
        },
    )
}

pub fn render_table_with_options(report: &ScanReport, options: &ScanRenderOptions) -> String {
    table::render(report, options)
}

pub fn render_json_with_options(
    report: &ScanReport,
    options: &ScanRenderOptions,
) -> Result<String, serde_json::Error> {
    json::render(report, options)
}

pub fn render_detect_table(report: &DetectReport) -> String {
    let mut output = String::new();
    writeln!(&mut output, "path: {}", report.path).expect("write to string");
    write_list_section(
        &mut output,
        "languages",
        report.languages.iter().map(ToString::to_string).collect(),
    );
    write_list_section(&mut output, "presets", report.presets.clone());
    write_list_section(
        &mut output,
        "categories",
        report.categories.iter().map(ToString::to_string).collect(),
    );
    write_list_section(
        &mut output,
        "active_ignore_rules",
        report.active_ignore_rules.clone(),
    );
    write_list_section(
        &mut output,
        "warnings",
        report.warnings.iter().map(format_warning).collect(),
    );
    output.trim_end().to_owned()
}

pub fn render_explain_table(report: &ExplainReport) -> String {
    let mut output = String::new();
    writeln!(&mut output, "path: {}", report.path).expect("write to string");
    writeln!(&mut output, "language: {}", report.language).expect("write to string");
    writeln!(&mut output, "category: {}", report.category).expect("write to string");
    write_metrics_section(&mut output, &report.metrics);
    write_list_section(&mut output, "reasons", report.reasons.clone());
    write_list_section(
        &mut output,
        "line_explanations",
        report
            .line_explanations
            .iter()
            .map(format_line_explanation)
            .collect(),
    );
    write_list_section(
        &mut output,
        "warnings",
        report.warnings.iter().map(format_warning).collect(),
    );
    output.trim_end().to_owned()
}

pub fn render_explain_json(report: &ExplainReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

fn write_metrics_section(output: &mut String, metrics: &FileMetrics) {
    writeln!(output, "metrics:").expect("write to string");
    writeln!(output, "  bytes: {}", metrics.bytes).expect("write to string");
    writeln!(output, "  total_lines: {}", metrics.total_lines).expect("write to string");
    writeln!(output, "  code_lines: {}", metrics.code_lines).expect("write to string");
    writeln!(output, "  mixed_lines: {}", metrics.mixed_lines).expect("write to string");
    writeln!(output, "  comment_lines: {}", metrics.comment_lines).expect("write to string");
    writeln!(output, "  doc_lines: {}", metrics.doc_lines).expect("write to string");
    writeln!(output, "  blank_lines: {}", metrics.blank_lines).expect("write to string");
    writeln!(output, "  sloc: {}", metrics.sloc()).expect("write to string");
    writeln!(output, "  parse_errors: {}", metrics.parse_errors).expect("write to string");
    writeln!(output, "  is_generated: {}", metrics.is_generated).expect("write to string");
    writeln!(output, "  is_vendor: {}", metrics.is_vendor).expect("write to string");
}

fn write_list_section(output: &mut String, title: &str, items: Vec<String>) {
    writeln!(output, "{title}:").expect("write to string");
    if items.is_empty() {
        writeln!(output, "- none").expect("write to string");
        return;
    }

    for item in items {
        writeln!(output, "- {item}").expect("write to string");
    }
}

fn format_line_explanation(explanation: &rloc_core::LineExplanation) -> String {
    format!(
        "line {} [{}] {}: {}",
        explanation.line_number, explanation.kind, explanation.snippet, explanation.reason
    )
}

pub(crate) fn format_warning(warning: &AnalysisWarning) -> String {
    match (&warning.path, warning.language) {
        (Some(path), Some(language)) => format!("{path} ({language}): {}", warning.message),
        (Some(path), None) => format!("{path}: {}", warning.message),
        (None, Some(language)) => format!("{language}: {}", warning.message),
        (None, None) => warning.message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{render_detect_table, render_explain_json, render_explain_table};
    use rloc_core::{
        AnalysisWarning, DetectReport, ExplainReport, FileCategory, FileMetrics, Language,
    };

    #[test]
    fn detect_table_rendering_lists_all_sections() {
        let report = DetectReport {
            path: "repo".into(),
            languages: vec![Language::Rust, Language::Python],
            presets: vec!["rust".to_owned(), "monorepo".to_owned()],
            categories: vec![FileCategory::Source, FileCategory::Test],
            active_ignore_rules: vec![".gitignore".to_owned(), "target/".to_owned()],
            warnings: vec![AnalysisWarning::for_language(
                Language::Rust,
                "fallback mode active",
            )],
        };

        let rendered = render_detect_table(&report);
        assert!(rendered.contains("path: repo"));
        assert!(rendered.contains("languages:\n- rust\n- python"));
        assert!(rendered.contains("presets:\n- rust\n- monorepo"));
        assert!(rendered.contains("categories:\n- source\n- test"));
        assert!(rendered.contains("active_ignore_rules:\n- .gitignore\n- target/"));
        assert!(rendered.contains("warnings:\n- rust: fallback mode active"));
    }

    #[test]
    fn explain_rendering_includes_metrics_and_warnings() {
        let report = ExplainReport {
            path: "src/lib.rs".into(),
            language: Language::Rust,
            category: FileCategory::Source,
            metrics: FileMetrics::from_physical_snapshot(
                "src/lib.rs".into(),
                Language::Rust,
                FileCategory::Source,
                42,
                10,
                2,
            ),
            reasons: vec![
                "language detected as rust from .rs extension".to_owned(),
                "category fell back to source because no more specific rule matched".to_owned(),
            ],
            line_explanations: vec![rloc_core::LineExplanation {
                line_number: 1,
                kind: "code".to_owned(),
                snippet: "fn main() {}".to_owned(),
                reason: "backend classified the line as code".to_owned(),
            }],
            warnings: vec![AnalysisWarning::for_file(
                "src/lib.rs".into(),
                Language::Rust,
                "fallback mode active",
            )],
        };

        let rendered = render_explain_table(&report);
        assert!(rendered.contains("path: src/lib.rs"));
        assert!(rendered.contains("language: rust"));
        assert!(rendered.contains("category: source"));
        assert!(rendered.contains("metrics:\n  bytes: 42\n  total_lines: 10"));
        assert!(rendered.contains("reasons:\n- language detected as rust from .rs extension"));
        assert!(rendered.contains(
            "line_explanations:\n- line 1 [code] fn main() {}: backend classified the line as code"
        ));
        assert!(rendered.contains("warnings:\n- src/lib.rs (rust): fallback mode active"));

        let json = render_explain_json(&report).unwrap();
        assert!(json.contains("\"language\": \"rust\""));
        assert!(json.contains("\"path\": \"src/lib.rs\""));
    }
}
