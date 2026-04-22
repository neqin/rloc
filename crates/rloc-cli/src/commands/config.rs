use std::fs;

use anyhow::{Context, anyhow};
use camino::Utf8Path;

use crate::cli::{ConfigArgs, ConfigCommand};
use crate::error::AppError;

pub fn run(args: ConfigArgs) -> Result<(), AppError> {
    let root = Utf8Path::new(".");
    match args.command {
        Some(ConfigCommand::Init) => println!("{}", init_default_config(root)?),
        None => println!("{}", render_resolved_config(root)?),
    }
    Ok(())
}

fn init_default_config(root: &Utf8Path) -> Result<String, AppError> {
    let config_path = rloc_config::default_config_path(root);
    if config_path.exists() {
        return Err(AppError::invalid_input(anyhow!(
            "{config_path} already exists"
        )));
    }

    let template = rloc_config::default_config_template().map_err(AppError::runtime)?;
    fs::write(config_path.as_std_path(), template)
        .with_context(|| format!("failed to write {config_path}"))
        .map_err(AppError::runtime)?;

    Ok(format!("created {config_path}"))
}

fn render_resolved_config(root: &Utf8Path) -> Result<String, AppError> {
    let resolved_path = rloc_config::resolve_config_path(root, None);
    let config = rloc_config::load_config(resolved_path.as_deref())
        .with_context(|| match &resolved_path {
            Some(path) => format!("failed to load configuration from {path}"),
            None => "failed to load default configuration".to_owned(),
        })
        .map_err(AppError::invalid_input)?;
    let rendered = rloc_config::render_config(&config).map_err(AppError::runtime)?;

    Ok(format!(
        "{}\n{rendered}",
        config_source_comment(resolved_path.as_deref())
    ))
}

fn config_source_comment(path: Option<&Utf8Path>) -> String {
    match path {
        Some(path) => format!("# source: {path}"),
        None => "# source: built-in defaults (no .rloc.toml found)".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use camino::Utf8PathBuf;

    use super::{init_default_config, render_resolved_config};

    #[test]
    fn init_default_config_writes_template_into_workspace() {
        let root = temp_dir("init_default_config_writes_template_into_workspace");
        let config_path = rloc_config::default_config_path(&root);

        let message = init_default_config(&root).unwrap();
        let written = fs::read_to_string(config_path.as_std_path()).unwrap();

        assert_eq!(message, format!("created {config_path}"));
        assert_eq!(written, rloc_config::default_config_template().unwrap());

        cleanup(&root);
    }

    #[test]
    fn render_resolved_config_reads_workspace_default_file() {
        let root = temp_dir("render_resolved_config_reads_workspace_default_file");
        let config_path = rloc_config::default_config_path(&root);
        fs::write(config_path.as_std_path(), "[scan]\nhidden = true\n").unwrap();

        let rendered = render_resolved_config(&root).unwrap();

        assert!(rendered.contains("hidden = true"));
        assert!(rendered.contains("respect_gitignore = true"));

        cleanup(&root);
    }

    #[test]
    fn render_resolved_config_reports_workspace_config_path() {
        let root = temp_dir("render_resolved_config_reports_workspace_config_path");
        let config_path = rloc_config::default_config_path(&root);
        fs::write(config_path.as_std_path(), "[scan]\nhidden = true\n").unwrap();

        let rendered = render_resolved_config(&root).unwrap();

        assert!(rendered.starts_with(&format!("# source: {config_path}\n")));

        cleanup(&root);
    }

    #[test]
    fn render_resolved_config_reads_parent_workspace_default_file() {
        let root = temp_dir("render_resolved_config_reads_parent_workspace_default_file");
        let nested = root.join("frontend/src");
        fs::create_dir_all(nested.as_std_path()).unwrap();
        let config_path = rloc_config::default_config_path(&root);
        fs::write(config_path.as_std_path(), "[scan]\nhidden = true\n").unwrap();

        let rendered = render_resolved_config(&nested).unwrap();

        assert!(rendered.contains("hidden = true"));
        assert!(rendered.contains("respect_gitignore = true"));

        cleanup(&root);
    }

    #[test]
    fn render_resolved_config_reports_parent_config_path() {
        let root = temp_dir("render_resolved_config_reports_parent_config_path");
        let nested = root.join("frontend/src");
        fs::create_dir_all(nested.as_std_path()).unwrap();
        let config_path = rloc_config::default_config_path(&root);
        fs::write(config_path.as_std_path(), "[scan]\nhidden = true\n").unwrap();

        let rendered = render_resolved_config(&nested).unwrap();

        assert!(rendered.starts_with(&format!("# source: {config_path}\n")));

        cleanup(&root);
    }

    #[test]
    fn render_resolved_config_uses_builtin_scan_report_defaults() {
        let root = temp_dir("render_resolved_config_uses_builtin_scan_report_defaults");

        let rendered = render_resolved_config(&root).unwrap();

        assert!(rendered.contains("group_by = [\"language\"]"));
        assert!(rendered.contains("top_files = 10"));
        assert!(rendered.contains("top_dirs = 10"));

        cleanup(&root);
    }

    #[test]
    fn render_resolved_config_reports_builtin_defaults_when_no_file_exists() {
        let root = temp_dir("render_resolved_config_reports_builtin_defaults_when_no_file_exists");

        let rendered = render_resolved_config(&root).unwrap();

        assert!(rendered.starts_with("# source: built-in defaults (no .rloc.toml found)\n"));

        cleanup(&root);
    }

    fn temp_dir(test_name: &str) -> Utf8PathBuf {
        let unique = format!(
            "rloc-cli-config-{test_name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        Utf8PathBuf::from_path_buf(path).unwrap()
    }

    fn cleanup(root: &Utf8PathBuf) {
        if root.exists() {
            fs::remove_dir_all(root.as_std_path()).unwrap();
        }
    }
}
