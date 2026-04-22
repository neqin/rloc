use camino::{Utf8Path, Utf8PathBuf};
use ignore::{WalkBuilder, overrides::OverrideBuilder};

use crate::{
    filters,
    types::{AnalysisWarning, ScanOptions},
};

#[derive(Debug, Clone, Default)]
pub struct DiscoveryResult {
    pub files: Vec<Utf8PathBuf>,
    pub warnings: Vec<AnalysisWarning>,
}

pub fn discover_candidate_files(
    root: &Utf8Path,
    options: &ScanOptions,
) -> Result<DiscoveryResult, String> {
    let mut builder = WalkBuilder::new(root);
    builder.hidden(!options.hidden);
    builder.git_ignore(options.respect_gitignore);
    builder.git_global(options.respect_gitignore);
    builder.git_exclude(options.respect_gitignore);
    apply_exclude_overrides(&mut builder, root, options)?;

    let walker = builder.build();
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) if error.is_io() => {
                warnings.push(AnalysisWarning::new(format!(
                    "skipped unreadable path during discovery: {error}"
                )));
                continue;
            }
            Err(error) => return Err(error.to_string()),
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let utf8 = Utf8PathBuf::from_path_buf(path.to_path_buf())
            .map_err(|non_utf8| format!("non-utf8 path encountered: {}", non_utf8.display()))?;

        if filters::is_ignored(&utf8) {
            continue;
        }

        files.push(utf8);
    }

    files.sort();
    Ok(DiscoveryResult { files, warnings })
}

pub fn collect_candidate_files(
    root: &Utf8Path,
    options: &ScanOptions,
) -> Result<Vec<Utf8PathBuf>, String> {
    Ok(discover_candidate_files(root, options)?.files)
}

fn apply_exclude_overrides(
    builder: &mut WalkBuilder,
    root: &Utf8Path,
    options: &ScanOptions,
) -> Result<(), String> {
    if options.exclude_patterns.is_empty() {
        return Ok(());
    }

    let mut override_builder = OverrideBuilder::new(root);
    for pattern in &options.exclude_patterns {
        override_builder
            .add(&format!("!{pattern}"))
            .map_err(|error| format!("invalid exclude pattern '{pattern}': {error}"))?;
    }

    let overrides = override_builder
        .build()
        .map_err(|error| format!("failed to build exclude overrides: {error}"))?;
    builder.overrides(overrides);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use camino::{Utf8Path, Utf8PathBuf};

    use super::{collect_candidate_files, discover_candidate_files};
    use crate::ScanOptions;

    #[test]
    fn honors_custom_exclude_patterns() {
        let root = temp_workspace("honors_custom_exclude_patterns");
        write_file(&root, "src/lib.rs", "fn main() {}\n");
        write_file(&root, "generated/out.rs", "fn generated() {}\n");

        let files = collect_candidate_files(
            &root,
            &ScanOptions {
                exclude_patterns: vec!["**/generated/**".to_owned()],
                ..ScanOptions::default()
            },
        )
        .unwrap();

        assert_eq!(files, vec![root.join("src/lib.rs")]);

        cleanup_workspace(&root);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_directory_is_reported_as_warning() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_workspace("unreadable_directory_is_reported_as_warning");
        write_file(&root, "src/lib.rs", "fn main() {}\n");
        write_file(&root, "blocked/secret.rs", "fn secret() {}\n");

        let blocked_dir = root.join("blocked");
        fs::set_permissions(blocked_dir.as_std_path(), fs::Permissions::from_mode(0o000)).unwrap();

        let result = (|| {
            let discovered = discover_candidate_files(&root, &ScanOptions::default()).unwrap();
            assert_eq!(discovered.files, vec![root.join("src/lib.rs")]);
            assert_eq!(discovered.warnings.len(), 1);
            assert!(discovered.warnings[0].message.contains("blocked"));
            assert!(discovered.warnings[0].message.contains("Permission denied"));
        })();

        fs::set_permissions(blocked_dir.as_std_path(), fs::Permissions::from_mode(0o755)).unwrap();
        cleanup_workspace(&root);
        result
    }

    fn temp_workspace(test_name: &str) -> Utf8PathBuf {
        let unique = format!(
            "rloc-discover-{test_name}-{}-{}",
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

    fn write_file(root: &Utf8Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent.as_std_path()).unwrap();
        }
        fs::write(path.as_std_path(), contents).unwrap();
    }

    fn cleanup_workspace(root: &Utf8Path) {
        if root.exists() {
            fs::remove_dir_all(root.as_std_path()).unwrap();
        }
    }
}
