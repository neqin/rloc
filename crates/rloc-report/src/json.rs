use serde::Serialize;

use rloc_core::{AnalysisWarning, MetricsSummary, ScanReport};

use crate::{GroupSummary, ScanRenderOptions, summary, top};

#[derive(Debug, Serialize)]
struct JsonMeta<'a> {
    version: &'a str,
    path: &'a str,
    format: &'a str,
    respect_gitignore: bool,
    generated_included: bool,
    vendor_included: bool,
    tests_included: bool,
}

#[derive(Debug, Serialize)]
struct JsonScanReport<'a> {
    meta: JsonMeta<'a>,
    summary: &'a MetricsSummary,
    groups: Vec<GroupSummary>,
    top_files: Vec<crate::TopFileEntry>,
    top_dirs: Vec<GroupSummary>,
    warnings: &'a [AnalysisWarning],
}

pub fn render(
    report: &ScanReport,
    options: &ScanRenderOptions,
) -> Result<String, serde_json::Error> {
    let groups = summary::groups(report, &options.group_by);
    let top_files = options
        .top_files
        .map(|limit| top::top_files(report, limit))
        .unwrap_or_default();
    let top_dirs = options
        .top_dirs
        .map(|limit| top::top_dirs(report, limit))
        .unwrap_or_default();

    let payload = JsonScanReport {
        meta: JsonMeta {
            version: env!("CARGO_PKG_VERSION"),
            path: options.path.as_str(),
            format: options.format.as_str(),
            respect_gitignore: options.respect_gitignore,
            generated_included: options.include_generated,
            vendor_included: options.include_vendor,
            tests_included: options.include_tests,
        },
        summary: &report.summary,
        groups,
        top_files,
        top_dirs,
        warnings: &report.warnings,
    };

    serde_json::to_string_pretty(&payload)
}
