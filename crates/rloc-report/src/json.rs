use serde::Serialize;

use rloc_core::{AnalysisWarning, MetricsSummary, ScanReport};

use crate::{GroupSummary, ScanJsonSection, ScanRenderOptions, summary, top};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<JsonMeta<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<&'a MetricsSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<Vec<GroupSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_files: Option<Vec<crate::TopFileEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_dirs: Option<Vec<GroupSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<&'a [AnalysisWarning]>,
}

pub fn render(
    report: &ScanReport,
    options: &ScanRenderOptions,
    sections: &[ScanJsonSection],
) -> Result<String, serde_json::Error> {
    let includes = |section| sections.is_empty() || sections.contains(&section);

    let payload = JsonScanReport {
        meta: includes(ScanJsonSection::Meta).then(|| JsonMeta {
            version: env!("CARGO_PKG_VERSION"),
            path: options.path.as_str(),
            format: options.format.as_str(),
            respect_gitignore: options.respect_gitignore,
            generated_included: options.include_generated,
            vendor_included: options.include_vendor,
            tests_included: options.include_tests,
        }),
        summary: includes(ScanJsonSection::Summary).then_some(&report.summary),
        groups: includes(ScanJsonSection::Groups)
            .then(|| summary::groups(report, &options.group_by)),
        top_files: includes(ScanJsonSection::TopFiles).then(|| {
            options
                .top_files
                .map(|limit| top::top_files(report, limit))
                .unwrap_or_default()
        }),
        top_dirs: includes(ScanJsonSection::TopDirs).then(|| {
            options
                .top_dirs
                .map(|limit| top::top_dirs(report, limit))
                .unwrap_or_default()
        }),
        warnings: includes(ScanJsonSection::Warnings).then_some(report.warnings.as_slice()),
    };

    serde_json::to_string_pretty(&payload)
}
