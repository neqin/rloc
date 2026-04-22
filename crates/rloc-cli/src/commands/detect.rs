use std::fs;

use anyhow::anyhow;

use crate::cli::DetectArgs;
use crate::error::AppError;

pub fn run(args: DetectArgs) -> Result<(), AppError> {
    fs::metadata(args.path.as_std_path())
        .map_err(|error| AppError::unreadable_path(anyhow!("{}: {error}", args.path)))?;

    let analyzer = rloc_core::Analyzer::new(super::default_registry());
    let options = rloc_core::ScanOptions {
        unsupported_sample_limit: args.list_unsupported,
        ..rloc_core::ScanOptions::default()
    };
    let mut report = analyzer
        .detect(&args.path, &options)
        .map_err(|error| AppError::runtime(anyhow!(error)))?;
    report.presets = rloc_config::detect_presets(&args.path, &report.languages)
        .into_iter()
        .map(|preset| preset.to_string())
        .collect();

    println!("{}", rloc_report::render_detect_table(&report));
    Ok(())
}
