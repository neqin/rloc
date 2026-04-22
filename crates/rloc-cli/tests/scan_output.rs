use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;

#[test]
fn scan_uses_report_defaults_from_config_when_flags_are_absent() {
    let root = temp_workspace("scan_uses_report_defaults_from_config_when_flags_are_absent");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "tests/test_app.py", "def test_ok():\n    pass\n");
    write_file(
        &root,
        ".rloc.toml",
        "[report]\nformat = \"json\"\ngroup_by = [\"category\"]\ntop_files = 1\ntop_dirs = 1\n",
    );

    let output = run_scan(&root, &[]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["meta"]["format"], "json");
    assert_eq!(json["top_files"].as_array().unwrap().len(), 1);
    assert_eq!(json["top_dirs"].as_array().unwrap().len(), 1);
    assert!(
        json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .all(|group| group["group_by"] == "category")
    );

    cleanup_workspace(&root);
}

#[test]
fn scan_group_by_flag_changes_rendered_output() {
    let root = temp_workspace("scan_group_by_flag_changes_rendered_output");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "tests/test_app.py", "def test_ok():\n    pass\n");
    write_file(&root, "frontend/App.tsx", "const App = () => <div />;\n");

    let baseline = run_scan(&root, &["--format", "json"]);
    let grouped = run_scan(
        &root,
        &[
            "--format",
            "json",
            "--group-by",
            "category",
            "--top-files",
            "1",
            "--top-dirs",
            "1",
        ],
    );

    assert!(baseline.status.success());
    assert!(grouped.status.success());
    assert_ne!(baseline.stdout, grouped.stdout);

    let json: Value = serde_json::from_slice(&grouped.stdout).unwrap();
    assert_eq!(json["top_files"].as_array().unwrap().len(), 1);
    assert_eq!(json["top_dirs"].as_array().unwrap().len(), 1);
    assert!(
        json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .all(|group| group["group_by"] == "category")
    );

    cleanup_workspace(&root);
}

#[test]
fn scan_shows_top_sections_by_default() {
    let root = temp_workspace("scan_shows_top_sections_by_default");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "tests/test_app.py", "def test_ok():\n    pass\n");

    let output = run_scan(&root, &["--format", "json"]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["top_files"].as_array().unwrap().len(), 2);
    assert_eq!(json["top_dirs"].as_array().unwrap().len(), 2);

    cleanup_workspace(&root);
}

#[test]
fn scan_no_top_flags_hide_default_top_sections() {
    let root = temp_workspace("scan_no_top_flags_hide_default_top_sections");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "tests/test_app.py", "def test_ok():\n    pass\n");

    let output = run_scan(
        &root,
        &["--format", "json", "--no-top-files", "--no-top-dirs"],
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert!(json["top_files"].as_array().unwrap().is_empty());
    assert!(json["top_dirs"].as_array().unwrap().is_empty());

    cleanup_workspace(&root);
}

#[test]
fn scan_config_can_disable_default_top_sections() {
    let root = temp_workspace("scan_config_can_disable_default_top_sections");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "tests/test_app.py", "def test_ok():\n    pass\n");
    write_file(
        &root,
        ".rloc.toml",
        "[report]\nformat = \"json\"\ntop_files = 0\ntop_dirs = 0\n",
    );

    let output = run_scan(&root, &[]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert!(json["top_files"].as_array().unwrap().is_empty());
    assert!(json["top_dirs"].as_array().unwrap().is_empty());

    cleanup_workspace(&root);
}

#[test]
fn scan_respects_custom_generated_and_vendor_patterns_from_config() {
    let root = temp_workspace("scan_respects_custom_generated_and_vendor_patterns_from_config");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(
        &root,
        "custom-generated/out.rs",
        "fn generated_fixture() {}\n",
    );
    write_file(
        &root,
        "external/vendorish/helper.rs",
        "fn vendor_fixture() {}\n",
    );
    write_file(
        &root,
        ".rloc.toml",
        "[filters]\ninclude_generated = false\ninclude_vendor = false\ngenerated_patterns = [\"**/custom-generated/**\"]\nvendor_patterns = [\"**/external/vendorish/**\"]\n\n[report]\nformat = \"json\"\n",
    );

    let output = run_scan(&root, &[]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["summary"]["files"], 1);
    assert_eq!(json["summary"]["sloc"], 1);

    cleanup_workspace(&root);
}

#[test]
fn scan_excludes_generated_files_by_default() {
    let root = temp_workspace("scan_excludes_generated_files_by_default");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(
        &root,
        "Cargo.lock",
        "# This file is automatically @generated\n",
    );

    let output = run_scan(&root, &["--format", "json", "--group-by", "category"]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["meta"]["generated_included"], false);
    assert_eq!(json["summary"]["files"], 1);
    assert_eq!(json["summary"]["sloc"], 1);
    assert!(
        !json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|group| group["group_by"] == "category" && group["key"] == "generated")
    );

    cleanup_workspace(&root);
}

#[test]
fn scan_includes_generated_files_when_flag_is_set() {
    let root = temp_workspace("scan_includes_generated_files_when_flag_is_set");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(
        &root,
        "Cargo.lock",
        "# This file is automatically @generated\n",
    );

    let output = run_scan(
        &root,
        &["--format", "json", "--group-by", "category", "--generated"],
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["meta"]["generated_included"], true);
    assert_eq!(json["summary"]["files"], 2);
    assert_eq!(json["summary"]["comment"], 1);
    assert_eq!(json["summary"]["sloc"], 1);
    assert!(
        json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|group| group["group_by"] == "category" && group["key"] == "generated")
    );

    cleanup_workspace(&root);
}

#[test]
fn scan_treats_lock_files_as_generated_by_default() {
    let root = temp_workspace("scan_treats_lock_files_as_generated_by_default");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "uv.lock", "version = 1\n");

    let output = run_scan(&root, &["--format", "json", "--group-by", "category"]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["summary"]["files"], 1);
    assert!(
        !json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|group| group["group_by"] == "category" && group["key"] == "config")
    );

    cleanup_workspace(&root);
}

#[test]
fn scan_respects_classification_toggles_from_config() {
    let root = temp_workspace("scan_respects_classification_toggles_from_config");
    write_file(
        &root,
        "src/lib.rs",
        concat!("/// crate docs\n", "fn main() { // note\n", "}\n",),
    );
    write_file(
        &root,
        ".rloc.toml",
        concat!(
            "[classification]\n",
            "count_doc_comments = false\n",
            "mixed_lines_as_code = false\n",
            "\n",
            "[report]\n",
            "format = \"json\"\n",
        ),
    );

    let output = run_scan(&root, &[]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["summary"]["doc"], 0);
    assert_eq!(json["summary"]["mixed"], 0);
    assert_eq!(json["summary"]["comment"], 1);
    assert_eq!(json["summary"]["code"], 2);
    assert_eq!(json["summary"]["sloc"], 2);

    cleanup_workspace(&root);
}

#[test]
fn scan_uses_parent_config_for_nested_scan_path() {
    let root = temp_workspace("scan_uses_parent_config_for_nested_scan_path");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(
        &root,
        ".rloc.toml",
        "[report]\nformat = \"json\"\ngroup_by = [\"file\"]\n",
    );

    let output = run_scan(&root.join("src"), &[]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["meta"]["format"], "json");
    assert!(
        json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .all(|group| group["group_by"] == "file")
    );

    cleanup_workspace(&root);
}

#[test]
fn scan_shorthand_accepts_flags_without_scan_subcommand() {
    let root = temp_workspace("scan_shorthand_accepts_flags_without_scan_subcommand");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "dist/ignored.rs", "fn ignored() {}\n");

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["--format", "json", "--exclude", "**/dist/**", root.as_str()])
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["summary"]["files"], 1);
    assert_eq!(json["meta"]["format"], "json");

    cleanup_workspace(&root);
}

#[test]
fn scan_bare_top_flags_use_default_limit() {
    let root = temp_workspace("scan_bare_top_flags_use_default_limit");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "tests/test_app.py", "def test_ok():\n    pass\n");

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args([
            "scan",
            root.as_str(),
            "--format",
            "json",
            "--top-files",
            "--top-dirs",
        ])
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["top_files"].as_array().unwrap().len(), 2);
    assert_eq!(json["top_dirs"].as_array().unwrap().len(), 2);

    cleanup_workspace(&root);
}

#[test]
fn scan_counts_markdown_and_config_files_as_supported_inputs() {
    let root = temp_workspace("scan_counts_markdown_and_config_files_as_supported_inputs");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "README.md", "# Demo\n\nProject notes\n");
    write_file(&root, "Cargo.toml", "[package]\nname = \"demo\"\n");

    let output = run_scan(&root, &["--format", "json", "--group-by", "category"]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["summary"]["files"], 3);
    assert_eq!(json["summary"]["doc"], 2);
    assert_eq!(json["summary"]["code"], 3);
    assert!(json["warnings"].as_array().unwrap().is_empty());
    assert!(
        json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|group| group["group_by"] == "category" && group["key"] == "docs")
    );
    assert!(
        json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|group| group["group_by"] == "category" && group["key"] == "config")
    );

    cleanup_workspace(&root);
}

#[test]
fn scan_lists_unsupported_paths_when_requested() {
    let root = temp_workspace("scan_lists_unsupported_paths_when_requested");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "README.txt", "ignored\n");
    write_file(&root, "notes.log", "ignored\n");

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["scan", root.as_str(), "--list-unsupported"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Warnings"));
    assert!(stdout.contains("2 files with unsupported extensions were skipped"));
    assert!(stdout.contains("README.txt: unsupported extension skipped"));
    assert!(stdout.contains("notes.log: unsupported extension skipped"));

    cleanup_workspace(&root);
}

fn run_scan(root: &Utf8Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rloc"));
    command.arg("scan").arg(root.as_str());
    command.args(args);
    command.output().unwrap()
}

fn temp_workspace(test_name: &str) -> Utf8PathBuf {
    let unique = format!(
        "rloc-cli-{test_name}-{}-{}",
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
