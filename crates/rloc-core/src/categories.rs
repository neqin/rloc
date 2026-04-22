use std::fs;

use camino::Utf8Path;
use globset::Glob;

use crate::types::{FileCategory, ScanOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryAssessment {
    pub category: FileCategory,
    pub reasons: Vec<String>,
}

pub fn detect_category(path: &Utf8Path) -> FileCategory {
    analyze_category(path).category
}

pub fn analyze_category(path: &Utf8Path) -> CategoryAssessment {
    analyze_category_with_options(path, &ScanOptions::default())
}

pub fn analyze_category_with_options(path: &Utf8Path, options: &ScanOptions) -> CategoryAssessment {
    let name = path.file_name().unwrap_or_default();

    if has_component(path, "vendor")
        || has_component(path, "third_party")
        || matches_any_pattern(path, &options.vendor_patterns)
    {
        let reason = if matches_any_pattern(path, &options.vendor_patterns) {
            "category detected from user-defined vendor pattern"
        } else {
            "category detected from vendor/third_party path component"
        };
        return assessment(FileCategory::Vendor, reason);
    }
    if is_generated_name(name) || matches_any_pattern(path, &options.generated_patterns) {
        let reason = if matches_any_pattern(path, &options.generated_patterns) {
            "category detected from user-defined generated pattern"
        } else {
            "category detected from generated-file naming convention"
        };
        return assessment(FileCategory::Generated, reason);
    }
    if let Some(reason) = generated_header_reason(path) {
        return assessment(FileCategory::Generated, reason);
    }
    if has_component(path, "tests") || has_component(path, "__tests__") || is_test_name(name) {
        return assessment(
            FileCategory::Test,
            "category detected from test naming/path conventions",
        );
    }
    if has_component(path, "examples") {
        return assessment(
            FileCategory::Example,
            "category detected from examples directory",
        );
    }
    if has_component(path, "benches") {
        return assessment(
            FileCategory::Bench,
            "category detected from benches directory",
        );
    }
    if is_docs_name(name) {
        return assessment(
            FileCategory::Docs,
            "category detected from documentation file extension",
        );
    }
    if is_config_name(name) {
        return assessment(
            FileCategory::Config,
            "category detected from config file extension",
        );
    }
    if has_component(path, "scripts") || has_component(path, "tools") {
        return assessment(
            FileCategory::Script,
            "category detected from scripts/tools directory",
        );
    }

    assessment(
        FileCategory::Source,
        "category fell back to source because no more specific rule matched",
    )
}

fn has_component(path: &Utf8Path, needle: &str) -> bool {
    path.components()
        .any(|component| component.as_str() == needle)
}

fn is_generated_name(name: &str) -> bool {
    name.contains(".generated.")
        || name.contains(".gen.")
        || name.ends_with(".pb.rs")
        || name.ends_with("_pb2.py")
        || is_lockfile_name(name)
}

fn is_docs_name(name: &str) -> bool {
    name.ends_with(".md") || name.ends_with(".mdx") || name.ends_with(".markdown")
}

fn is_config_name(name: &str) -> bool {
    name.ends_with(".toml")
        || name.ends_with(".yaml")
        || name.ends_with(".yml")
        || name.ends_with(".json")
        || name.ends_with(".jsonc")
        || name.ends_with(".ini")
        || name.ends_with(".cfg")
        || name.ends_with(".conf")
}

fn is_lockfile_name(name: &str) -> bool {
    name.ends_with(".lock")
        || matches!(
            name,
            "package-lock.json" | "pnpm-lock.yaml" | "pnpm-lock.yml" | "bun.lockb"
        )
}

fn matches_any_pattern(path: &Utf8Path, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        Glob::new(pattern)
            .map(|glob| glob.compile_matcher().is_match(path.as_str()))
            .unwrap_or(false)
    })
}

fn generated_header_reason(path: &Utf8Path) -> Option<&'static str> {
    let header = fs::read(path.as_std_path()).ok()?;
    let header = String::from_utf8_lossy(&header);
    let preview = header.lines().take(8).collect::<Vec<_>>().join("\n");

    if preview.contains("@generated") {
        Some("category detected from generated-file header marker '@generated'")
    } else if preview.contains("Code generated") {
        Some("category detected from generated-file header marker 'Code generated'")
    } else if preview.contains("DO NOT EDIT") {
        Some("category detected from generated-file header marker 'DO NOT EDIT'")
    } else {
        None
    }
}

fn is_test_name(name: &str) -> bool {
    name.ends_with(".test.ts")
        || name.ends_with(".test.tsx")
        || name.ends_with(".test.js")
        || name.ends_with(".test.jsx")
        || name.starts_with("test_")
        || name.ends_with("_test.py")
}

fn assessment(category: FileCategory, reason: impl Into<String>) -> CategoryAssessment {
    CategoryAssessment {
        category,
        reasons: vec![reason.into()],
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use camino::Utf8Path;

    use super::{analyze_category, analyze_category_with_options, detect_category};
    use crate::{FileCategory, ScanOptions};

    #[test]
    fn generated_takes_priority_over_test_patterns() {
        let path = Utf8Path::new("tests/generated/foo.generated.rs");
        assert_eq!(detect_category(path), FileCategory::Generated);
    }

    #[test]
    fn vendor_takes_priority_over_other_path_signals() {
        let path = Utf8Path::new("vendor/tests/foo.rs");
        assert_eq!(detect_category(path), FileCategory::Vendor);
    }

    #[test]
    fn root_level_tests_directory_is_detected() {
        let path = Utf8Path::new("tests/foo.rs");
        assert_eq!(detect_category(path), FileCategory::Test);
    }

    #[test]
    fn tools_directory_is_treated_as_script() {
        let path = Utf8Path::new("tools/generate.rs");
        assert_eq!(detect_category(path), FileCategory::Script);
    }

    #[test]
    fn generated_header_marks_existing_file_as_generated() {
        let root = temp_workspace("generated_header_marks_existing_file_as_generated");
        let path = root.join("src/generated.rs");
        write_file(
            &root,
            "src/generated.rs",
            "// Code generated by fixture. DO NOT EDIT.\nfn generated() {}\n",
        );

        let category = analyze_category(&path);
        assert_eq!(category.category, FileCategory::Generated);
        assert!(category.reasons[0].contains("generated-file header"));

        cleanup_workspace(&root);
    }

    #[test]
    fn custom_patterns_override_default_source_category() {
        let root = temp_workspace("custom_patterns_override_default_source_category");
        let generated = root.join("custom-generated/out.rs");
        let vendor = root.join("external/vendorish/helper.rs");
        write_file(&root, "custom-generated/out.rs", "fn generated() {}\n");
        write_file(&root, "external/vendorish/helper.rs", "fn vendored() {}\n");

        let options = ScanOptions {
            generated_patterns: vec!["**/custom-generated/**".to_owned()],
            vendor_patterns: vec!["**/external/vendorish/**".to_owned()],
            ..ScanOptions::default()
        };

        assert_eq!(
            analyze_category_with_options(&generated, &options).category,
            FileCategory::Generated
        );
        assert_eq!(
            analyze_category_with_options(&vendor, &options).category,
            FileCategory::Vendor
        );

        cleanup_workspace(&root);
    }

    #[test]
    fn markdown_files_are_categorized_as_docs() {
        let path = Utf8Path::new("docs/README.md");
        assert_eq!(detect_category(path), FileCategory::Docs);
    }

    #[test]
    fn config_extensions_are_categorized_as_config() {
        assert_eq!(
            detect_category(Utf8Path::new("Cargo.toml")),
            FileCategory::Config
        );
        assert_eq!(
            detect_category(Utf8Path::new("pnpm-workspace.yaml")),
            FileCategory::Config
        );
    }

    #[test]
    fn lock_files_are_categorized_as_generated() {
        assert_eq!(
            detect_category(Utf8Path::new("Cargo.lock")),
            FileCategory::Generated
        );
        assert_eq!(
            detect_category(Utf8Path::new("uv.lock")),
            FileCategory::Generated
        );
        assert_eq!(
            detect_category(Utf8Path::new("package-lock.json")),
            FileCategory::Generated
        );
    }

    fn temp_workspace(test_name: &str) -> camino::Utf8PathBuf {
        let unique = format!(
            "rloc-categories-{test_name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        camino::Utf8PathBuf::from_path_buf(path).unwrap()
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
