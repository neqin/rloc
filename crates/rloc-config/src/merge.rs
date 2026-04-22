use rloc_core::{ClassificationOptions, Language, ScanOptions};

use crate::{ConfigFile, ReportConfig, ReportFormat, ReportGroupBy};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanOverrides {
    pub languages: Option<Vec<Language>>,
    pub exclude_patterns: Option<Vec<String>>,
    pub hidden: Option<bool>,
    pub respect_gitignore: Option<bool>,
    pub include_tests: Option<bool>,
    pub include_generated: Option<bool>,
    pub include_vendor: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportOverrides {
    pub format: Option<ReportFormat>,
    pub top_files: Option<usize>,
    pub top_dirs: Option<usize>,
    pub disable_top_files: bool,
    pub disable_top_dirs: bool,
    pub group_by: Option<Vec<ReportGroupBy>>,
}

pub fn scan_options_from_config(config: &ConfigFile) -> ScanOptions {
    let mut options = ScanOptions::default();
    options.hidden = config.scan.hidden;
    options.respect_gitignore = config.scan.respect_gitignore;
    options.include_tests = config.filters.include_tests;
    options.include_generated = config.filters.include_generated;
    options.include_vendor = config.filters.include_vendor;
    options.exclude_patterns = config.filters.exclude.clone();
    options.generated_patterns = config.filters.generated_patterns.clone();
    options.vendor_patterns = config.filters.vendor_patterns.clone();
    options.classification = ClassificationOptions {
        count_doc_comments: config.classification.count_doc_comments,
        count_docstrings_as_comments: config.classification.count_docstrings_as_comments,
        mixed_lines_as_code: config.classification.mixed_lines_as_code,
    };
    options
}

pub fn merge_scan_options(config: &ConfigFile, overrides: &ScanOverrides) -> ScanOptions {
    let mut options = scan_options_from_config(config);

    if let Some(languages) = overrides.languages.clone() {
        options.languages = Some(languages);
    }
    if let Some(exclude_patterns) = overrides.exclude_patterns.clone() {
        options.exclude_patterns.extend(exclude_patterns);
    }
    if let Some(hidden) = overrides.hidden {
        options.hidden = hidden;
    }
    if let Some(respect_gitignore) = overrides.respect_gitignore {
        options.respect_gitignore = respect_gitignore;
    }
    if let Some(include_tests) = overrides.include_tests {
        options.include_tests = include_tests;
    }
    if let Some(include_generated) = overrides.include_generated {
        options.include_generated = include_generated;
    }
    if let Some(include_vendor) = overrides.include_vendor {
        options.include_vendor = include_vendor;
    }

    options
}

pub fn merge_report_config(config: &ConfigFile, overrides: &ReportOverrides) -> ReportConfig {
    let mut report = config.report.clone();
    report.top_files = normalize_top_limit(report.top_files);
    report.top_dirs = normalize_top_limit(report.top_dirs);

    if let Some(format) = &overrides.format {
        report.format = format.clone();
    }
    if overrides.disable_top_files {
        report.top_files = None;
    } else if let Some(top_files) = normalize_top_limit(overrides.top_files) {
        report.top_files = Some(top_files);
    }
    if overrides.disable_top_dirs {
        report.top_dirs = None;
    } else if let Some(top_dirs) = normalize_top_limit(overrides.top_dirs) {
        report.top_dirs = Some(top_dirs);
    }
    if let Some(group_by) = &overrides.group_by {
        report.group_by = group_by.clone();
    }

    report
}

fn normalize_top_limit(limit: Option<usize>) -> Option<usize> {
    limit.filter(|limit| *limit > 0)
}

#[cfg(test)]
mod tests {
    use rloc_core::Language;

    use crate::{ConfigFile, FiltersConfig, ReportConfig, ReportFormat, ReportGroupBy, ScanConfig};

    use super::{
        ReportOverrides, ScanOverrides, merge_report_config, merge_scan_options,
        scan_options_from_config,
    };

    #[test]
    fn applies_scan_and_filter_defaults_from_config() {
        let config = ConfigFile {
            scan: ScanConfig {
                respect_gitignore: false,
                hidden: true,
            },
            filters: FiltersConfig {
                exclude: vec!["**/target/**".to_owned()],
                include_tests: false,
                include_generated: false,
                include_vendor: true,
                ..FiltersConfig::default()
            },
            ..ConfigFile::default()
        };

        let options = scan_options_from_config(&config);
        assert!(options.hidden);
        assert!(!options.respect_gitignore);
        assert!(!options.include_tests);
        assert!(!options.include_generated);
        assert!(options.include_vendor);
        assert_eq!(options.exclude_patterns, vec!["**/target/**"]);
    }

    #[test]
    fn cli_overrides_take_precedence_over_config() {
        let config = ConfigFile {
            scan: ScanConfig {
                respect_gitignore: true,
                hidden: false,
            },
            filters: FiltersConfig {
                include_tests: true,
                include_generated: true,
                include_vendor: false,
                ..FiltersConfig::default()
            },
            ..ConfigFile::default()
        };

        let options = merge_scan_options(
            &config,
            &ScanOverrides {
                languages: Some(vec![Language::Rust]),
                exclude_patterns: Some(vec!["**/dist/**".to_owned()]),
                hidden: Some(true),
                respect_gitignore: Some(false),
                include_tests: Some(false),
                include_generated: Some(false),
                include_vendor: Some(true),
            },
        );

        assert_eq!(options.languages, Some(vec![Language::Rust]));
        assert_eq!(options.exclude_patterns, vec!["**/dist/**"]);
        assert!(options.hidden);
        assert!(!options.respect_gitignore);
        assert!(!options.include_tests);
        assert!(!options.include_generated);
        assert!(options.include_vendor);
    }

    #[test]
    fn report_overrides_take_precedence_over_config() {
        let config = ConfigFile {
            report: ReportConfig {
                format: ReportFormat::Table,
                top_files: Some(10),
                top_dirs: Some(10),
                group_by: vec![ReportGroupBy::Language],
            },
            ..ConfigFile::default()
        };

        let report = merge_report_config(
            &config,
            &ReportOverrides {
                format: Some(ReportFormat::Json),
                top_files: Some(25),
                top_dirs: Some(5),
                disable_top_files: false,
                disable_top_dirs: false,
                group_by: Some(vec![ReportGroupBy::Category, ReportGroupBy::Dir]),
            },
        );

        assert_eq!(report.format, ReportFormat::Json);
        assert_eq!(report.top_files, Some(25));
        assert_eq!(report.top_dirs, Some(5));
        assert_eq!(
            report.group_by,
            vec![ReportGroupBy::Category, ReportGroupBy::Dir]
        );
    }

    #[test]
    fn zero_top_limits_disable_sections() {
        let config = ConfigFile {
            report: ReportConfig {
                top_files: Some(0),
                top_dirs: Some(0),
                ..ReportConfig::default()
            },
            ..ConfigFile::default()
        };

        let report = merge_report_config(&config, &ReportOverrides::default());

        assert_eq!(report.top_files, None);
        assert_eq!(report.top_dirs, None);
    }
}
