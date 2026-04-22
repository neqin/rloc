use std::{fs, io};

use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;

pub mod file_config;
pub mod merge;
pub mod presets;

pub use file_config::{
    ClassificationConfig, ConfigFile, FiltersConfig, ReportConfig, ReportFormat, ReportGroupBy,
    ScanConfig,
};
pub use merge::{
    ReportOverrides, ScanOverrides, merge_report_config, merge_scan_options,
    scan_options_from_config,
};
pub use presets::{Preset, detect_presets};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}")]
    Read {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse config file {path}")]
    Parse {
        path: Utf8PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to serialize configuration")]
    Serialize(#[from] toml::ser::Error),
}

pub fn default_config_path(root: &Utf8Path) -> Utf8PathBuf {
    root.join(".rloc.toml")
}

pub fn resolve_config_path(root: &Utf8Path, explicit: Option<&Utf8Path>) -> Option<Utf8PathBuf> {
    explicit
        .map(Utf8Path::to_path_buf)
        .or_else(|| find_default_config_in_ancestors(root))
}

fn find_default_config_in_ancestors(root: &Utf8Path) -> Option<Utf8PathBuf> {
    let canonical = fs::canonicalize(root.as_std_path()).ok()?;
    let canonical = Utf8PathBuf::from_path_buf(canonical).ok()?;
    let mut current = if canonical.is_dir() {
        Some(canonical)
    } else {
        canonical.parent().map(Utf8Path::to_path_buf)
    };

    while let Some(directory) = current {
        let candidate = default_config_path(&directory);
        if candidate.exists() {
            return Some(candidate);
        }
        current = directory.parent().map(Utf8Path::to_path_buf);
    }

    None
}

pub fn load_config(path: Option<&Utf8Path>) -> Result<ConfigFile, ConfigError> {
    let Some(path) = path else {
        return Ok(ConfigFile::default());
    };

    let contents = fs::read_to_string(path.as_std_path()).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn render_config(config: &ConfigFile) -> Result<String, ConfigError> {
    Ok(toml::to_string_pretty(config)?)
}

pub fn default_config_template() -> Result<String, ConfigError> {
    render_config(&ConfigFile::default())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use camino::Utf8PathBuf;

    use super::{
        ConfigFile, default_config_path, default_config_template, load_config, resolve_config_path,
    };

    #[test]
    fn default_template_round_trips_through_toml() {
        let template = default_config_template().unwrap();
        let parsed: ConfigFile = toml::from_str(&template).unwrap();
        assert_eq!(parsed, ConfigFile::default());
    }

    #[test]
    fn load_config_reads_explicit_path() {
        let directory = temp_dir("load_config_reads_explicit_path");
        let path = directory.join("custom.rloc.toml");
        fs::write(
            path.as_std_path(),
            "[scan]\nhidden = true\n\n[filters]\ninclude_vendor = true\n",
        )
        .unwrap();

        let config = load_config(Some(&path)).unwrap();
        assert!(config.scan.hidden);
        assert!(config.filters.include_vendor);

        fs::remove_dir_all(directory.as_std_path()).unwrap();
    }

    #[test]
    fn resolve_config_path_prefers_explicit_path() {
        let root = temp_dir("resolve_config_path_prefers_explicit_path");
        let explicit = root.join("custom.rloc.toml");

        let resolved = resolve_config_path(&root, Some(&explicit));
        assert_eq!(resolved, Some(explicit.clone()));

        fs::remove_dir_all(root.as_std_path()).unwrap();
    }

    #[test]
    fn resolve_config_path_falls_back_to_default_file_when_present() {
        let root = temp_dir("resolve_config_path_falls_back_to_default_file_when_present");
        let default = default_config_path(&root);
        fs::write(default.as_std_path(), "[scan]\nhidden = true\n").unwrap();

        let resolved = resolve_config_path(&root, None);
        assert_eq!(resolved, Some(default.clone()));

        fs::remove_dir_all(root.as_std_path()).unwrap();
    }

    #[test]
    fn resolve_config_path_returns_none_when_default_file_is_absent() {
        let root = temp_dir("resolve_config_path_returns_none_when_default_file_is_absent");

        let resolved = resolve_config_path(&root, None);
        assert_eq!(resolved, None);

        fs::remove_dir_all(root.as_std_path()).unwrap();
    }

    #[test]
    fn resolve_config_path_searches_parent_directories() {
        let root = temp_dir("resolve_config_path_searches_parent_directories");
        let nested = root.join("frontend/src");
        fs::create_dir_all(nested.as_std_path()).unwrap();
        let default = default_config_path(&root);
        fs::write(default.as_std_path(), "[scan]\nhidden = true\n").unwrap();

        let resolved = resolve_config_path(&nested, None);
        assert_eq!(resolved, Some(default.clone()));

        fs::remove_dir_all(root.as_std_path()).unwrap();
    }

    #[test]
    fn load_config_rejects_invalid_report_format() {
        let directory = temp_dir("load_config_rejects_invalid_report_format");
        let path = directory.join("invalid.rloc.toml");
        fs::write(path.as_std_path(), "[report]\nformat = \"yaml\"\n").unwrap();

        let error = load_config(Some(&path)).unwrap_err();

        assert!(matches!(error, super::ConfigError::Parse { .. }));

        fs::remove_dir_all(directory.as_std_path()).unwrap();
    }

    #[test]
    fn load_config_rejects_invalid_report_group_by() {
        let directory = temp_dir("load_config_rejects_invalid_report_group_by");
        let path = directory.join("invalid.rloc.toml");
        fs::write(path.as_std_path(), "[report]\ngroup_by = [\"madeup\"]\n").unwrap();

        let error = load_config(Some(&path)).unwrap_err();

        assert!(matches!(error, super::ConfigError::Parse { .. }));

        fs::remove_dir_all(directory.as_std_path()).unwrap();
    }

    fn temp_dir(test_name: &str) -> Utf8PathBuf {
        let unique = format!(
            "rloc-config-{test_name}-{}-{}",
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
}
