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

impl FileMetrics {
    pub fn from_line_breakdown(
        path: Utf8PathBuf,
        language: Language,
        category: FileCategory,
        bytes: u64,
        total_lines: u32,
        blank_lines: u32,
        code_lines: u32,
        comment_lines: u32,
        doc_lines: u32,
        mixed_lines: u32,
        parse_errors: u32,
    ) -> Self {
        Self {
            path,
            language,
            category,
            bytes,
            total_lines,
            blank_lines,
            code_lines,
            comment_lines,
            doc_lines,
            mixed_lines,
            parse_errors,
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
            total_lines,
            blank_lines,
            0,
            0,
            0,
            0,
            0,
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
