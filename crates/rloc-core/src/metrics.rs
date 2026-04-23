use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use serde::Serialize;

use crate::types::{AnalysisWarning, FileCategory, Language};

#[derive(Debug, Clone, Serialize)]
pub struct FileMetrics {
    pub path: Utf8PathBuf,
    pub language: Language,
    pub category: FileCategory,
    pub bytes: u64,
    pub total_lines: u32,
    pub blank_lines: u32,
    pub code_lines: u32,
    pub comment_lines: u32,
    pub doc_lines: u32,
    pub mixed_lines: u32,
    pub parse_errors: u32,
    pub is_generated: bool,
    pub is_vendor: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineBreakdown {
    pub total: u32,
    pub blank: u32,
    pub code: u32,
    pub comment: u32,
    pub doc: u32,
    pub mixed: u32,
    pub parse_errors: u32,
}

impl FileMetrics {
    pub fn from_line_breakdown(
        path: Utf8PathBuf,
        language: Language,
        category: FileCategory,
        bytes: u64,
        lines: LineBreakdown,
    ) -> Self {
        Self {
            path,
            language,
            category,
            bytes,
            total_lines: lines.total,
            blank_lines: lines.blank,
            code_lines: lines.code,
            comment_lines: lines.comment,
            doc_lines: lines.doc,
            mixed_lines: lines.mixed,
            parse_errors: lines.parse_errors,
            is_generated: matches!(category, FileCategory::Generated),
            is_vendor: matches!(category, FileCategory::Vendor),
        }
    }

    pub fn from_physical_snapshot(
        path: Utf8PathBuf,
        language: Language,
        category: FileCategory,
        bytes: u64,
        total_lines: u32,
        blank_lines: u32,
    ) -> Self {
        Self::from_line_breakdown(
            path,
            language,
            category,
            bytes,
            LineBreakdown {
                total: total_lines,
                blank: blank_lines,
                ..LineBreakdown::default()
            },
        )
    }

    pub fn sloc(&self) -> u32 {
        self.code_lines + self.mixed_lines
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MetricsSummary {
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

impl MetricsSummary {
    pub fn add_file(&mut self, file: &FileMetrics) {
        self.files += 1;
        self.lines += u64::from(file.total_lines);
        self.code += u64::from(file.code_lines);
        self.mixed += u64::from(file.mixed_lines);
        self.comment += u64::from(file.comment_lines);
        self.doc += u64::from(file.doc_lines);
        self.blank += u64::from(file.blank_lines);
        self.sloc += u64::from(file.sloc());
        self.bytes += file.bytes;
        self.parse_errors += u64::from(file.parse_errors);
    }

    pub fn from_files(files: &[FileMetrics]) -> Self {
        let mut summary = Self::default();

        for file in files {
            summary.add_file(file);
        }

        summary
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanReport {
    pub summary: MetricsSummary,
    pub files: Vec<FileMetrics>,
    pub by_language: BTreeMap<Language, MetricsSummary>,
    pub warnings: Vec<AnalysisWarning>,
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;

    use super::{FileMetrics, LineBreakdown};
    use crate::{FileCategory, Language};

    #[test]
    fn constructs_metrics_from_named_line_breakdown() {
        let metrics = FileMetrics::from_line_breakdown(
            Utf8PathBuf::from("src/lib.rs"),
            Language::Rust,
            FileCategory::Source,
            42,
            LineBreakdown {
                total: 5,
                blank: 1,
                code: 2,
                comment: 1,
                doc: 0,
                mixed: 1,
                parse_errors: 0,
            },
        );

        assert_eq!(metrics.total_lines, 5);
        assert_eq!(metrics.blank_lines, 1);
        assert_eq!(metrics.code_lines, 2);
        assert_eq!(metrics.comment_lines, 1);
        assert_eq!(metrics.mixed_lines, 1);
        assert_eq!(metrics.sloc(), 3);
    }
}
