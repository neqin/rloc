use std::fs;

use anyhow::{Context, anyhow};
use rloc_config::{ReportFormat, ReportGroupBy};
use rloc_report::{ScanGroupBy, ScanRenderOptions};

use crate::cli::{GroupBy, OutputFormat, ScanArgs};
use crate::error::AppError;

pub fn run(args: ScanArgs) -> Result<(), AppError> {
    ensure_readable_path(&args.path)?;

    let registry = super::default_registry();
    let analyzer = rloc_core::Analyzer::new(registry);
    let resolved_path = rloc_config::resolve_config_path(&args.path, args.config.as_deref());
    let config = rloc_config::load_config(resolved_path.as_deref())
        .with_context(|| match &resolved_path {
            Some(path) => format!("failed to load configuration from {path}"),
            None => format!("failed to load configuration for {}", args.path),
        })
        .map_err(AppError::invalid_input)?;

    let options = rloc_core::ScanOptions {
        unsupported_sample_limit: args.list_unsupported,
        ..rloc_config::merge_scan_options(&config, &scan_overrides(&args))
    };
    let report = analyzer
        .scan(&args.path, &options)
        .map_err(|error| AppError::runtime(anyhow!(error)))?;

    let report_format = args
        .format
        .unwrap_or_else(|| format_from_config(config.report.format));
    let report_defaults = rloc_config::merge_report_config(&config, &report_overrides(&args));
    let render_options = render_options(&args, &options, &report_defaults, report_format);

    match report_format {
        OutputFormat::Table => {
            println!(
                "{}",
                rloc_report::render_table_with_options(&report, &render_options)
            );
        }
        OutputFormat::Json => {
            println!(
                "{}",
                rloc_report::render_json_with_options(&report, &render_options)
                    .map_err(AppError::runtime)?
            );
        }
    }
    Ok(())
}

fn ensure_readable_path(path: &camino::Utf8Path) -> Result<(), AppError> {
    fs::metadata(path.as_std_path())
        .map(|_| ())
        .map_err(|error| AppError::unreadable_path(anyhow!("{path}: {error}")))
}

fn scan_overrides(args: &ScanArgs) -> rloc_config::ScanOverrides {
    rloc_config::ScanOverrides {
        languages: (!args.languages.is_empty()).then(|| {
            args.languages
                .iter()
                .copied()
                .map(rloc_core::Language::from)
                .collect()
        }),
        exclude_patterns: (!args.exclude.is_empty()).then(|| args.exclude.clone()),
        hidden: args.hidden.then_some(true),
        respect_gitignore: args.no_gitignore.then_some(false),
        include_tests: args.no_tests.then_some(false),
        include_generated: args.generated.then_some(true),
        include_vendor: args.no_vendor.then_some(false),
    }
}

fn report_overrides(args: &ScanArgs) -> rloc_config::ReportOverrides {
    rloc_config::ReportOverrides {
        format: args.format.map(report_format),
        top_files: args.top_files.filter(|limit| *limit > 0),
        top_dirs: args.top_dirs.filter(|limit| *limit > 0),
        disable_top_files: args.no_top_files || args.top_files == Some(0),
        disable_top_dirs: args.no_top_dirs || args.top_dirs == Some(0),
        group_by: (!args.group_by.is_empty()).then(|| {
            args.group_by
                .iter()
                .copied()
                .map(report_group_by)
                .collect::<Vec<_>>()
        }),
    }
}

fn format_from_config(value: ReportFormat) -> OutputFormat {
    match value {
        ReportFormat::Json => OutputFormat::Json,
        ReportFormat::Table => OutputFormat::Table,
    }
}

fn output_format_name(value: OutputFormat) -> String {
    match value {
        OutputFormat::Table => "table".to_owned(),
        OutputFormat::Json => "json".to_owned(),
    }
}

fn report_format(value: OutputFormat) -> ReportFormat {
    match value {
        OutputFormat::Table => ReportFormat::Table,
        OutputFormat::Json => ReportFormat::Json,
    }
}

fn report_group_by(value: GroupBy) -> ReportGroupBy {
    match value {
        GroupBy::Language => ReportGroupBy::Language,
        GroupBy::Category => ReportGroupBy::Category,
        GroupBy::Dir => ReportGroupBy::Dir,
        GroupBy::File => ReportGroupBy::File,
    }
}

fn render_options(
    args: &ScanArgs,
    scan: &rloc_core::ScanOptions,
    report: &rloc_config::ReportConfig,
    report_format: OutputFormat,
) -> ScanRenderOptions {
    ScanRenderOptions {
        path: args.path.clone(),
        format: output_format_name(report_format),
        group_by: default_group_by(args, report),
        top_files: default_top_files(args, report),
        top_dirs: default_top_dirs(args, report),
        respect_gitignore: scan.respect_gitignore,
        include_generated: scan.include_generated,
        include_vendor: scan.include_vendor,
        include_tests: scan.include_tests,
    }
}

fn default_group_by(args: &ScanArgs, report: &rloc_config::ReportConfig) -> Vec<ScanGroupBy> {
    if !args.group_by.is_empty() {
        return args.group_by.iter().copied().map(scan_group_by).collect();
    }

    report
        .group_by
        .iter()
        .copied()
        .map(scan_group_by_from_config)
        .collect()
}

fn default_top_files(args: &ScanArgs, report: &rloc_config::ReportConfig) -> Option<usize> {
    if args.no_top_files || args.top_files == Some(0) {
        return None;
    }

    args.top_files
        .filter(|limit| *limit > 0)
        .or(report.top_files)
}

fn default_top_dirs(args: &ScanArgs, report: &rloc_config::ReportConfig) -> Option<usize> {
    if args.no_top_dirs || args.top_dirs == Some(0) {
        return None;
    }

    args.top_dirs.filter(|limit| *limit > 0).or(report.top_dirs)
}

fn scan_group_by(value: GroupBy) -> ScanGroupBy {
    match value {
        GroupBy::Language => ScanGroupBy::Language,
        GroupBy::Category => ScanGroupBy::Category,
        GroupBy::Dir => ScanGroupBy::Dir,
        GroupBy::File => ScanGroupBy::File,
    }
}

fn scan_group_by_from_config(value: ReportGroupBy) -> ScanGroupBy {
    match value {
        ReportGroupBy::Language => ScanGroupBy::Language,
        ReportGroupBy::Category => ScanGroupBy::Category,
        ReportGroupBy::Dir => ScanGroupBy::Dir,
        ReportGroupBy::File => ScanGroupBy::File,
    }
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;
    use rloc_config::{ReportFormat, ReportGroupBy};
    use rloc_core::Language;
    use rloc_report::ScanGroupBy;

    use super::{
        default_group_by, default_top_dirs, default_top_files, format_from_config,
        report_overrides, scan_overrides,
    };
    use crate::cli::{GroupBy, LanguageArg, OutputFormat, ScanArgs};

    #[test]
    fn scan_overrides_capture_cli_filter_flags() {
        let mut args = default_args();
        args.languages = vec![LanguageArg::Rust, LanguageArg::Typescript];
        args.exclude = vec!["**/dist/**".to_owned(), "**/target/**".to_owned()];
        args.hidden = true;
        args.no_gitignore = true;
        args.no_tests = true;
        args.generated = true;
        args.no_vendor = true;

        let overrides = scan_overrides(&args);

        assert_eq!(
            overrides.languages,
            Some(vec![Language::Rust, Language::TypeScript])
        );
        assert_eq!(
            overrides.exclude_patterns,
            Some(vec!["**/dist/**".to_owned(), "**/target/**".to_owned()])
        );
        assert_eq!(overrides.hidden, Some(true));
        assert_eq!(overrides.respect_gitignore, Some(false));
        assert_eq!(overrides.include_tests, Some(false));
        assert_eq!(overrides.include_generated, Some(true));
        assert_eq!(overrides.include_vendor, Some(false));
    }

    #[test]
    fn report_overrides_capture_cli_reporting_flags() {
        let mut args = default_args();
        args.format = Some(OutputFormat::Json);
        args.top_files = Some(25);
        args.top_dirs = Some(5);
        args.group_by = vec![GroupBy::Category, GroupBy::Dir];

        let overrides = report_overrides(&args);

        assert_eq!(overrides.format, Some(ReportFormat::Json));
        assert_eq!(overrides.top_files, Some(25));
        assert_eq!(overrides.top_dirs, Some(5));
        assert!(!overrides.disable_top_files);
        assert!(!overrides.disable_top_dirs);
        assert_eq!(
            overrides.group_by,
            Some(vec![ReportGroupBy::Category, ReportGroupBy::Dir])
        );
    }

    #[test]
    fn report_overrides_can_disable_top_sections() {
        let mut args = default_args();
        args.no_top_files = true;
        args.no_top_dirs = true;

        let overrides = report_overrides(&args);

        assert_eq!(overrides.top_files, None);
        assert_eq!(overrides.top_dirs, None);
        assert!(overrides.disable_top_files);
        assert!(overrides.disable_top_dirs);
    }

    #[test]
    fn format_from_config_supports_json_output() {
        assert!(matches!(
            format_from_config(ReportFormat::Json),
            OutputFormat::Json
        ));
        assert!(matches!(
            format_from_config(ReportFormat::Table),
            OutputFormat::Table
        ));
    }

    #[test]
    fn report_defaults_are_taken_from_config_when_present() {
        let args = default_args();
        let report = rloc_config::ReportConfig {
            format: ReportFormat::Json,
            top_files: Some(7),
            top_dirs: Some(3),
            group_by: vec![ReportGroupBy::Category, ReportGroupBy::Dir],
        };

        assert_eq!(
            default_group_by(&args, &report),
            vec![ScanGroupBy::Category, ScanGroupBy::Dir]
        );
        assert_eq!(default_top_files(&args, &report), Some(7));
        assert_eq!(default_top_dirs(&args, &report), Some(3));
    }

    #[test]
    fn report_defaults_fall_back_to_built_in_values_without_config_file() {
        let args = default_args();
        let report = rloc_config::ReportConfig::default();

        assert_eq!(
            default_group_by(&args, &report),
            vec![ScanGroupBy::Language]
        );
        assert_eq!(default_top_files(&args, &report), Some(10));
        assert_eq!(default_top_dirs(&args, &report), Some(10));
    }

    #[test]
    fn no_top_flags_disable_default_report_limits() {
        let mut args = default_args();
        let report = rloc_config::ReportConfig::default();
        args.no_top_files = true;
        args.no_top_dirs = true;

        assert_eq!(default_top_files(&args, &report), None);
        assert_eq!(default_top_dirs(&args, &report), None);
    }

    fn default_args() -> ScanArgs {
        ScanArgs {
            path: Utf8PathBuf::from("."),
            format: None,
            group_by: Vec::new(),
            top_files: None,
            top_dirs: None,
            no_top_files: false,
            no_top_dirs: false,
            list_unsupported: None,
            languages: Vec::new(),
            exclude: Vec::new(),
            no_tests: false,
            generated: false,
            no_vendor: false,
            config: None,
            no_gitignore: false,
            hidden: false,
        }
    }
}
