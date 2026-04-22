use std::fs;

use anyhow::anyhow;
use camino::Utf8Path;

use crate::cli::{ExplainArgs, OutputFormat};
use crate::error::AppError;

pub fn run(args: ExplainArgs) -> Result<(), AppError> {
    let metadata = fs::metadata(args.file.as_std_path())
        .map_err(|error| AppError::unreadable_path(anyhow!("{}: {error}", args.file)))?;
    if !metadata.is_file() {
        return Err(AppError::unreadable_path(anyhow!(
            "{} is not a readable file",
            args.file
        )));
    }

    let registry = super::default_registry();
    let language = registry.detect_language(&args.file);
    if matches!(language, rloc_core::Language::Unknown) {
        return Err(AppError::unsupported_explain_target(anyhow!(
            "{} is not a supported source file",
            args.file
        )));
    }

    let config_root = Utf8Path::new(".");
    let resolved_config = rloc_config::resolve_config_path(config_root, args.config.as_deref());
    let config =
        rloc_config::load_config(resolved_config.as_deref()).map_err(AppError::invalid_input)?;
    let options = rloc_config::scan_options_from_config(&config);

    let analyzer = rloc_core::Analyzer::new(registry);
    let report = analyzer
        .explain_with_options(&args.file, &options)
        .map_err(|error| AppError::runtime(anyhow!(error)))?;

    match args.format {
        OutputFormat::Table => println!("{}", rloc_report::render_explain_table(&report)),
        OutputFormat::Json => {
            println!(
                "{}",
                rloc_report::render_explain_json(&report).map_err(AppError::runtime)?
            )
        }
    }

    Ok(())
}
