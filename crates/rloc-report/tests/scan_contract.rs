use std::collections::BTreeMap;

use rloc_core::{AnalysisWarning, FileCategory, FileMetrics, Language, MetricsSummary, ScanReport};
use rloc_report::{
    ScanGroupBy, ScanRenderOptions, render_json_with_options, render_table_with_options,
    top::{top_dirs, top_files},
};
use serde_json::Value;

#[test]
fn json_output_matches_mvp_schema() {
    let report = sample_report();
    let options = ScanRenderOptions {
        path: "repo".into(),
        format: "json".to_owned(),
        group_by: vec![ScanGroupBy::Language, ScanGroupBy::Category],
        top_files: Some(2),
        top_dirs: Some(2),
        respect_gitignore: true,
        include_generated: true,
        include_vendor: false,
        include_tests: true,
    };

    let rendered = render_json_with_options(&report, &options).unwrap();
    let json: Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["meta"]["path"], "repo");
    assert_eq!(json["meta"]["format"], "json");
    assert_eq!(json["meta"]["respect_gitignore"], true);
    assert_eq!(json["meta"]["generated_included"], true);
    assert_eq!(json["meta"]["vendor_included"], false);
    assert_eq!(json["meta"]["tests_included"], true);

    assert_eq!(json["summary"]["files"], 4);
    assert_eq!(json["summary"]["sloc"], 30);

    let groups = json["groups"].as_array().unwrap();
    assert!(groups.iter().any(|group| {
        group["group_by"] == "language" && group["key"] == "rust" && group["files"] == 2
    }));
    assert!(groups.iter().any(|group| {
        group["group_by"] == "category" && group["key"] == "generated" && group["files"] == 1
    }));

    let top_files = json["top_files"].as_array().unwrap();
    assert_eq!(top_files.len(), 2);
    assert_eq!(top_files[0]["path"], "frontend/App.tsx");
    assert_eq!(top_files[1]["path"], "src/lib.rs");

    let top_dirs = json["top_dirs"].as_array().unwrap();
    assert_eq!(top_dirs.len(), 2);
    assert_eq!(top_dirs[0]["group_by"], "dir");
    assert_eq!(top_dirs[0]["key"], "frontend");
    assert_eq!(top_dirs[1]["key"], "src");

    assert_eq!(json["warnings"].as_array().unwrap().len(), 1);
}

#[test]
fn render_json_defaults_to_generated_files_excluded() {
    let report = sample_report();

    let rendered = rloc_report::render_json(&report).unwrap();
    let json: Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["meta"]["generated_included"], false);
}

#[test]
fn table_output_renders_requested_groups() {
    let report = sample_report();
    let options = ScanRenderOptions {
        path: "repo".into(),
        format: "table".to_owned(),
        group_by: vec![ScanGroupBy::Category, ScanGroupBy::Dir],
        top_files: Some(2),
        top_dirs: Some(2),
        respect_gitignore: true,
        include_generated: true,
        include_vendor: false,
        include_tests: true,
    };

    let rendered = render_table_with_options(&report, &options);

    assert!(rendered.contains("Summary"));
    assert!(rendered.contains("Groups by category"));
    assert!(rendered.contains("generated"));
    assert!(rendered.contains("Groups by dir"));
    assert!(rendered.contains("frontend"));
    assert!(rendered.contains("Top files"));
    assert!(rendered.contains("Top dirs"));
    assert!(rendered.contains("Warnings"));
}

#[test]
fn table_output_includes_compact_category_totals() {
    let report = sample_report();
    let options = ScanRenderOptions {
        path: "repo".into(),
        format: "table".to_owned(),
        group_by: vec![ScanGroupBy::Language],
        top_files: None,
        top_dirs: None,
        respect_gitignore: true,
        include_generated: true,
        include_vendor: false,
        include_tests: true,
    };

    let rendered = render_table_with_options(&report, &options);

    let summary_index = rendered.find("Summary").unwrap();
    let category_index = rendered.find("Category totals").unwrap();
    let group_index = rendered.find("Groups by language").unwrap();

    assert!(summary_index < category_index);
    assert!(category_index < group_index);
    let category_header = header_after_heading(&rendered, "Category totals");
    assert_eq!(
        cell_value(category_header, category_header, "category"),
        "category"
    );
    assert_eq!(
        cell_value(category_header, category_header, "files"),
        "files"
    );
    assert_eq!(
        cell_value(category_header, category_header, "lines"),
        "lines"
    );
    assert_eq!(cell_value(category_header, category_header, "sloc"), "sloc");
    assert!(rendered.contains("source"));
    assert!(rendered.contains("generated"));
    assert!(rendered.contains("test"));
}

#[test]
fn top_files_and_top_dirs_are_sorted_and_limited() {
    let report = sample_report();

    let files = top_files(&report, 2);
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path.as_str(), "frontend/App.tsx");
    assert_eq!(files[1].path.as_str(), "src/lib.rs");

    let dirs = top_dirs(&report, 2);
    assert_eq!(dirs.len(), 2);
    assert_eq!(dirs[0].key, "frontend");
    assert_eq!(dirs[0].sloc, 11);
    assert_eq!(dirs[1].key, "src");
    assert_eq!(dirs[1].sloc, 9);
}

#[test]
fn top_dirs_skip_overlapping_parent_and_child_directories() {
    let report = sample_report_with_overlapping_dirs();

    let dirs = top_dirs(&report, 2);

    assert_eq!(dirs.len(), 2);
    assert_eq!(dirs[0].key, "experiments/plate_ocr");
    assert_eq!(dirs[1].key, "tools/streamer/src");
}

#[test]
fn table_output_keeps_top_file_columns_aligned_for_long_paths() {
    let report = sample_report_with_long_paths();
    let options = ScanRenderOptions {
        path: "repo".into(),
        format: "table".to_owned(),
        group_by: vec![],
        top_files: Some(1),
        top_dirs: None,
        respect_gitignore: true,
        include_generated: true,
        include_vendor: false,
        include_tests: true,
    };

    let rendered = render_table_with_options(&report, &options);
    let row = row_after_heading(&rendered, "Top files");
    let header = header_after_heading(&rendered, "Top files");

    assert_eq!(cell_value(header, row, "language"), "rust");
    assert_eq!(cell_value(header, row, "category"), "source");
    assert_eq!(
        cell_value(header, row, "path"),
        "packages/company/ultra/really/deep/module/src/classify.rs"
    );
}

#[test]
fn table_output_keeps_top_dir_columns_aligned_for_long_paths() {
    let report = sample_report_with_long_paths();
    let options = ScanRenderOptions {
        path: "repo".into(),
        format: "table".to_owned(),
        group_by: vec![],
        top_files: None,
        top_dirs: Some(1),
        respect_gitignore: true,
        include_generated: true,
        include_vendor: false,
        include_tests: true,
    };

    let rendered = render_table_with_options(&report, &options);
    let row = row_after_heading(&rendered, "Top dirs");
    let header = header_after_heading(&rendered, "Top dirs");

    assert_eq!(cell_value(header, row, "files"), "1");
    assert_eq!(cell_value(header, row, "lines"), "120");
    assert_eq!(
        cell_value(header, row, "group"),
        "packages/company/ultra/really/deep/module/src"
    );
}

#[test]
fn table_output_places_path_and_group_columns_last_in_top_sections() {
    let report = sample_report();
    let options = ScanRenderOptions {
        path: "repo".into(),
        format: "table".to_owned(),
        group_by: vec![],
        top_files: Some(1),
        top_dirs: Some(1),
        respect_gitignore: true,
        include_generated: true,
        include_vendor: false,
        include_tests: true,
    };

    let rendered = render_table_with_options(&report, &options);
    let top_files_header = header_after_heading(&rendered, "Top files");
    let top_dirs_header = header_after_heading(&rendered, "Top dirs");

    assert_eq!(top_files_header.split_whitespace().last().unwrap(), "path");
    assert_eq!(top_dirs_header.split_whitespace().last().unwrap(), "group");
}

#[test]
fn table_output_keeps_summary_columns_aligned_for_large_numbers() {
    let report = sample_report_with_large_metrics();
    let options = ScanRenderOptions {
        path: "repo".into(),
        format: "table".to_owned(),
        group_by: vec![],
        top_files: None,
        top_dirs: None,
        respect_gitignore: true,
        include_generated: true,
        include_vendor: false,
        include_tests: true,
    };

    let rendered = render_table_with_options(&report, &options);
    let row = row_after_heading(&rendered, "Summary");
    let header = header_after_heading(&rendered, "Summary");

    assert_eq!(cell_value(header, row, "files"), "1");
    assert_eq!(cell_value(header, row, "lines"), "123456");
    assert_eq!(cell_value(header, row, "bytes"), "12345678");
    assert_eq!(cell_value(header, row, "sloc"), "111111");
}

#[test]
fn json_output_uses_canonical_js_family_language_names() {
    let report = sample_report_with_js_family_languages();
    let options = ScanRenderOptions {
        path: "repo".into(),
        format: "json".to_owned(),
        group_by: vec![ScanGroupBy::Language],
        top_files: Some(2),
        top_dirs: None,
        respect_gitignore: true,
        include_generated: true,
        include_vendor: false,
        include_tests: true,
    };

    let rendered = render_json_with_options(&report, &options).unwrap();
    let json: Value = serde_json::from_str(&rendered).unwrap();

    let groups = json["groups"].as_array().unwrap();
    assert!(
        groups
            .iter()
            .any(|group| { group["group_by"] == "language" && group["key"] == "javascript" })
    );
    assert!(
        groups
            .iter()
            .any(|group| { group["group_by"] == "language" && group["key"] == "typescript" })
    );

    let top_files = json["top_files"].as_array().unwrap();
    assert!(
        top_files
            .iter()
            .any(|entry| entry["path"] == "frontend/index.ts" && entry["language"] == "typescript")
    );
    assert!(top_files
        .iter()
        .any(|entry| entry["path"] == "frontend/legacy.js" && entry["language"] == "javascript"));
}

fn sample_report() -> ScanReport {
    let files = vec![
        FileMetrics::from_line_breakdown(
            "src/lib.rs".into(),
            Language::Rust,
            FileCategory::Source,
            120,
            12,
            2,
            8,
            1,
            0,
            1,
            0,
        ),
        FileMetrics::from_line_breakdown(
            "src/generated/client.generated.rs".into(),
            Language::Rust,
            FileCategory::Generated,
            70,
            6,
            1,
            4,
            0,
            0,
            0,
            0,
        ),
        FileMetrics::from_line_breakdown(
            "tests/test_app.py".into(),
            Language::Python,
            FileCategory::Test,
            80,
            10,
            2,
            5,
            2,
            0,
            1,
            0,
        ),
        FileMetrics::from_line_breakdown(
            "frontend/App.tsx".into(),
            Language::Tsx,
            FileCategory::Source,
            150,
            14,
            3,
            8,
            1,
            1,
            3,
            0,
        ),
    ];

    let mut by_language = BTreeMap::new();
    for language in [Language::Rust, Language::Python, Language::Tsx] {
        let matching = files
            .iter()
            .filter(|file| file.language == language)
            .cloned()
            .collect::<Vec<_>>();
        if !matching.is_empty() {
            by_language.insert(language, MetricsSummary::from_files(&matching));
        }
    }

    ScanReport {
        summary: MetricsSummary::from_files(&files),
        files,
        by_language,
        warnings: vec![AnalysisWarning::for_path(
            "src/generated/client.generated.rs".into(),
            "generated heuristic matched fixture file",
        )],
    }
}

fn sample_report_with_long_paths() -> ScanReport {
    let files = vec![FileMetrics::from_line_breakdown(
        "packages/company/ultra/really/deep/module/src/classify.rs".into(),
        Language::Rust,
        FileCategory::Source,
        120,
        120,
        0,
        110,
        5,
        0,
        5,
        0,
    )];

    let mut by_language = BTreeMap::new();
    by_language.insert(Language::Rust, MetricsSummary::from_files(&files));

    ScanReport {
        summary: MetricsSummary::from_files(&files),
        files,
        by_language,
        warnings: Vec::new(),
    }
}

fn sample_report_with_large_metrics() -> ScanReport {
    let files = vec![FileMetrics::from_line_breakdown(
        "src/lib.rs".into(),
        Language::Rust,
        FileCategory::Source,
        12_345_678,
        123_456,
        23_456,
        100_000,
        11_111,
        0,
        11_111,
        0,
    )];

    let mut by_language = BTreeMap::new();
    by_language.insert(Language::Rust, MetricsSummary::from_files(&files));

    ScanReport {
        summary: MetricsSummary::from_files(&files),
        files,
        by_language,
        warnings: Vec::new(),
    }
}

fn sample_report_with_overlapping_dirs() -> ScanReport {
    let files = vec![
        FileMetrics::from_line_breakdown(
            "experiments/uv.lock".into(),
            Language::Config,
            FileCategory::Generated,
            1_200,
            320,
            20,
            300,
            0,
            0,
            0,
            0,
        ),
        FileMetrics::from_line_breakdown(
            "experiments/train.py".into(),
            Language::Python,
            FileCategory::Source,
            400,
            80,
            5,
            75,
            0,
            0,
            0,
            0,
        ),
        FileMetrics::from_line_breakdown(
            "experiments/plate_ocr/app.py".into(),
            Language::Python,
            FileCategory::Source,
            1_500,
            420,
            20,
            400,
            0,
            0,
            0,
            0,
        ),
        FileMetrics::from_line_breakdown(
            "tools/streamer/src/main.rs".into(),
            Language::Rust,
            FileCategory::Script,
            900,
            220,
            15,
            205,
            0,
            0,
            0,
            0,
        ),
    ];

    let mut by_language = BTreeMap::new();
    for language in [Language::Config, Language::Python, Language::Rust] {
        let matching = files
            .iter()
            .filter(|file| file.language == language)
            .cloned()
            .collect::<Vec<_>>();
        if !matching.is_empty() {
            by_language.insert(language, MetricsSummary::from_files(&matching));
        }
    }

    ScanReport {
        summary: MetricsSummary::from_files(&files),
        files,
        by_language,
        warnings: Vec::new(),
    }
}

fn sample_report_with_js_family_languages() -> ScanReport {
    let files = vec![
        FileMetrics::from_line_breakdown(
            "frontend/index.ts".into(),
            Language::TypeScript,
            FileCategory::Source,
            180,
            18,
            2,
            14,
            1,
            0,
            1,
            0,
        ),
        FileMetrics::from_line_breakdown(
            "frontend/legacy.js".into(),
            Language::JavaScript,
            FileCategory::Source,
            160,
            16,
            1,
            13,
            1,
            0,
            1,
            0,
        ),
    ];

    let mut by_language = BTreeMap::new();
    for language in [Language::JavaScript, Language::TypeScript] {
        let matching = files
            .iter()
            .filter(|file| file.language == language)
            .cloned()
            .collect::<Vec<_>>();
        if !matching.is_empty() {
            by_language.insert(language, MetricsSummary::from_files(&matching));
        }
    }

    ScanReport {
        summary: MetricsSummary::from_files(&files),
        files,
        by_language,
        warnings: Vec::new(),
    }
}

fn header_after_heading<'a>(rendered: &'a str, heading: &str) -> &'a str {
    let lines = rendered.lines().collect::<Vec<_>>();
    let index = lines.iter().position(|line| *line == heading).unwrap();
    lines[index + 1]
}

fn row_after_heading<'a>(rendered: &'a str, heading: &str) -> &'a str {
    let lines = rendered.lines().collect::<Vec<_>>();
    let index = lines.iter().position(|line| *line == heading).unwrap();
    lines[index + 2]
}

fn cell_value<'a>(header: &str, row: &'a str, label: &str) -> &'a str {
    let labels = header.split_whitespace().collect::<Vec<_>>();
    let index = labels.iter().position(|column| *column == label).unwrap();
    let start = header.find(label).unwrap();
    let end = labels
        .get(index + 1)
        .and_then(|next| header.find(next))
        .unwrap_or(row.len());

    row[start..end].trim()
}
