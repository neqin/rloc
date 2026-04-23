use rloc_config::{
    ClassificationConfig, ConfigFile, FiltersConfig, ReportConfig, ReportFormat, ReportGroupBy,
    ScanConfig,
};
use rloc_core::ClassificationOptions;

#[test]
fn scan_options_include_classification_toggles_and_custom_patterns() {
    let config = ConfigFile {
        scan: ScanConfig {
            respect_gitignore: false,
            hidden: true,
        },
        filters: FiltersConfig {
            exclude: vec!["**/target/**".to_owned()],
            include_tests: false,
            include_generated: false,
            include_vendor: false,
            generated_patterns: vec!["**/custom-generated/**".to_owned()],
            vendor_patterns: vec!["**/external/vendorish/**".to_owned()],
        },
        classification: ClassificationConfig {
            count_doc_comments: false,
            count_docstrings_as_comments: false,
            mixed_lines_as_code: false,
        },
        report: ReportConfig::default(),
    };

    let options = rloc_config::scan_options_from_config(&config);

    assert!(options.hidden);
    assert!(!options.respect_gitignore);
    assert!(!options.include_tests);
    assert!(!options.include_generated);
    assert!(!options.include_vendor);
    assert_eq!(options.exclude_patterns, vec!["**/target/**"]);
    assert_eq!(
        options.generated_patterns,
        vec!["**/custom-generated/**".to_owned()]
    );
    assert_eq!(
        options.vendor_patterns,
        vec!["**/external/vendorish/**".to_owned()]
    );
    assert_eq!(
        options.classification,
        ClassificationOptions {
            count_doc_comments: false,
            count_docstrings_as_comments: false,
            mixed_lines_as_code: false,
        }
    );
}

#[test]
fn report_defaults_still_merge_from_config() {
    let config = ConfigFile {
        report: ReportConfig {
            format: ReportFormat::Json,
            top_files: Some(7),
            top_dirs: Some(3),
            group_by: vec![ReportGroupBy::Category, ReportGroupBy::Dir],
        },
        ..ConfigFile::default()
    };

    let report =
        rloc_config::merge_report_config(&config, &rloc_config::ReportOverrides::default());

    assert_eq!(report.format, ReportFormat::Json);
    assert_eq!(report.top_files, Some(7));
    assert_eq!(report.top_dirs, Some(3));
    assert_eq!(
        report.group_by,
        vec![ReportGroupBy::Category, ReportGroupBy::Dir]
    );
}

#[test]
fn cli_overrides_do_not_reset_classification_or_custom_patterns() {
    let config = ConfigFile {
        filters: FiltersConfig {
            exclude: vec!["**/target/**".to_owned()],
            generated_patterns: vec!["**/custom-generated/**".to_owned()],
            vendor_patterns: vec!["**/external/vendorish/**".to_owned()],
            ..FiltersConfig::default()
        },
        classification: ClassificationConfig {
            count_doc_comments: false,
            count_docstrings_as_comments: false,
            mixed_lines_as_code: false,
        },
        ..ConfigFile::default()
    };

    let merged = rloc_config::merge_scan_options(
        &config,
        &rloc_config::ScanOverrides {
            exclude_patterns: Some(vec!["**/dist/**".to_owned()]),
            include_generated: Some(false),
            include_vendor: Some(false),
            ..rloc_config::ScanOverrides::default()
        },
    );

    assert_eq!(
        merged.exclude_patterns,
        vec!["**/target/**".to_owned(), "**/dist/**".to_owned()]
    );
    assert_eq!(
        merged.generated_patterns,
        vec!["**/custom-generated/**".to_owned()]
    );
    assert_eq!(
        merged.vendor_patterns,
        vec!["**/external/vendorish/**".to_owned()]
    );
    assert!(!merged.classification.count_doc_comments);
    assert!(!merged.classification.count_docstrings_as_comments);
    assert!(!merged.classification.mixed_lines_as_code);
}
