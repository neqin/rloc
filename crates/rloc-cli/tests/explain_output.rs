use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;

#[test]
fn explain_respects_classification_toggles_from_config() {
    let root = temp_workspace("explain_respects_classification_toggles_from_config");
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
        ),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args([
            "explain",
            "src/lib.rs",
            "--format",
            "json",
            "--config",
            ".rloc.toml",
        ])
        .current_dir(root.as_std_path())
        .output()
        .unwrap();

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["metrics"]["doc_lines"], 0);
    assert_eq!(json["metrics"]["mixed_lines"], 0);
    assert_eq!(json["metrics"]["comment_lines"], 1);
    assert_eq!(json["metrics"]["code_lines"], 2);

    cleanup_workspace(&root);
}

#[test]
fn explain_uses_parent_config_from_nested_working_directory() {
    let root = temp_workspace("explain_uses_parent_config_from_nested_working_directory");
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
        ),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["explain", "lib.rs", "--format", "json"])
        .current_dir(root.join("src").as_std_path())
        .output()
        .unwrap();

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["metrics"]["doc_lines"], 0);
    assert_eq!(json["metrics"]["mixed_lines"], 0);
    assert_eq!(json["metrics"]["comment_lines"], 1);
    assert_eq!(json["metrics"]["code_lines"], 2);

    cleanup_workspace(&root);
}

#[test]
fn explain_supports_markdown_docs() {
    let root = temp_workspace("explain_supports_markdown_docs");
    write_file(&root, "README.md", "# Title\n\nProject notes\n");

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["explain", "README.md", "--format", "json"])
        .current_dir(root.as_std_path())
        .output()
        .unwrap();

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["language"], "markdown");
    assert_eq!(json["category"], "docs");
    assert_eq!(json["metrics"]["doc_lines"], 2);
    assert_eq!(json["metrics"]["code_lines"], 0);
    assert_eq!(json["metrics"]["blank_lines"], 1);

    cleanup_workspace(&root);
}

#[test]
fn explain_respects_custom_category_patterns_from_config() {
    let root = temp_workspace("explain_respects_custom_category_patterns_from_config");
    write_file(&root, "custom-generated/out.rs", "fn generated() {}\n");
    write_file(&root, "external/vendorish/lib.rs", "fn vendored() {}\n");
    write_file(
        &root,
        ".rloc.toml",
        concat!(
            "[filters]\n",
            "generated_patterns = [\"**/custom-generated/**\"]\n",
            "vendor_patterns = [\"**/external/vendorish/**\"]\n",
        ),
    );

    let generated_output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args([
            "explain",
            "custom-generated/out.rs",
            "--format",
            "json",
            "--config",
            ".rloc.toml",
        ])
        .current_dir(root.as_std_path())
        .output()
        .unwrap();
    let generated_json: Value = serde_json::from_slice(&generated_output.stdout).unwrap();

    assert!(generated_output.status.success());
    assert_eq!(generated_json["category"], "generated");
    assert!(
        generated_json["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| {
                reason
                    .as_str()
                    .is_some_and(|reason| reason.contains("user-defined generated pattern"))
            })
    );

    let vendor_output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args([
            "explain",
            "external/vendorish/lib.rs",
            "--format",
            "json",
            "--config",
            ".rloc.toml",
        ])
        .current_dir(root.as_std_path())
        .output()
        .unwrap();
    let vendor_json: Value = serde_json::from_slice(&vendor_output.stdout).unwrap();

    assert!(vendor_output.status.success());
    assert_eq!(vendor_json["category"], "vendor");
    assert!(
        vendor_json["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| {
                reason
                    .as_str()
                    .is_some_and(|reason| reason.contains("user-defined vendor pattern"))
            })
    );

    cleanup_workspace(&root);
}

#[test]
fn explain_supports_shell_dotfiles() {
    let root = temp_workspace("explain_supports_shell_dotfiles");
    write_file(
        &root,
        ".bashrc",
        concat!("export PATH=\"$PATH:$HOME/bin\"\n", "# shell note\n",),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["explain", ".bashrc", "--format", "json"])
        .current_dir(root.as_std_path())
        .output()
        .unwrap();

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["language"], "shell");
    assert_eq!(json["category"], "source");
    assert_eq!(json["metrics"]["code_lines"], 1);
    assert_eq!(json["metrics"]["comment_lines"], 1);

    cleanup_workspace(&root);
}

#[test]
fn explain_supports_psql_files() {
    let root = temp_workspace("explain_supports_psql_files");
    write_file(
        &root,
        "query.psql",
        concat!("-- sql note\n", "select 1;\n",),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["explain", "query.psql", "--format", "json"])
        .current_dir(root.as_std_path())
        .output()
        .unwrap();

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["language"], "sql");
    assert_eq!(json["category"], "source");
    assert_eq!(json["metrics"]["code_lines"], 1);
    assert_eq!(json["metrics"]["comment_lines"], 1);

    cleanup_workspace(&root);
}

#[test]
fn explain_supports_go_sources() {
    let root = temp_workspace("explain_supports_go_sources");
    write_file(
        &root,
        "main.go",
        concat!("package main\n", "func main() {}\n",),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["explain", "main.go", "--format", "json"])
        .current_dir(root.as_std_path())
        .output()
        .unwrap();

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(output.status.success());
    assert_eq!(json["language"], "go");
    assert_eq!(json["category"], "source");
    assert_eq!(json["metrics"]["code_lines"], 2);

    cleanup_workspace(&root);
}

#[test]
fn explain_supports_html_and_css_sources() {
    let root = temp_workspace("explain_supports_html_and_css_sources");
    write_file(&root, "index.html", "<main>Hello</main>\n");
    write_file(&root, "app.css", "body { color: red; }\n");

    let html_output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["explain", "index.html", "--format", "json"])
        .current_dir(root.as_std_path())
        .output()
        .unwrap();
    let html_json: Value = serde_json::from_slice(&html_output.stdout).unwrap();

    assert!(html_output.status.success());
    assert_eq!(html_json["language"], "html");
    assert_eq!(html_json["category"], "source");
    assert_eq!(html_json["metrics"]["code_lines"], 1);

    let css_output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["explain", "app.css", "--format", "json"])
        .current_dir(root.as_std_path())
        .output()
        .unwrap();
    let css_json: Value = serde_json::from_slice(&css_output.stdout).unwrap();

    assert!(css_output.status.success());
    assert_eq!(css_json["language"], "css");
    assert_eq!(css_json["category"], "source");
    assert_eq!(css_json["metrics"]["code_lines"], 1);

    cleanup_workspace(&root);
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
