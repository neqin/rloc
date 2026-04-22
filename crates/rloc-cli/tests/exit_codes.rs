use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use camino::{Utf8Path, Utf8PathBuf};

#[test]
fn missing_input_path_returns_exit_code_3() {
    let missing = Utf8PathBuf::from(format!(
        "/tmp/rloc-cli-missing-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let status = Command::new(bin_path())
        .args(["scan", missing.as_str()])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(3));
}

#[test]
fn unsupported_explain_target_returns_exit_code_4() {
    let root = temp_dir("unsupported_explain_target_returns_exit_code_4");
    let file = root.join("notes.txt");
    write_file(&root, "notes.txt", "plain text\n");

    let status = Command::new(bin_path())
        .args(["explain", file.as_str()])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(4));

    cleanup(&root);
}

#[test]
fn invalid_config_returns_exit_code_2() {
    let root = temp_dir("invalid_config_returns_exit_code_2");
    write_file(&root, ".rloc.toml", "[scan\nhidden = true\n");

    let status = Command::new(bin_path())
        .arg("config")
        .current_dir(root.as_std_path())
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(2));

    cleanup(&root);
}

#[test]
fn invalid_report_config_returns_exit_code_2_for_scan() {
    let root = temp_dir("invalid_report_config_returns_exit_code_2_for_scan");
    write_file(&root, ".rloc.toml", "[report]\nformat = \"yaml\"\n");
    write_file(&root, "main.rs", "fn main() {}\n");

    let status = Command::new(bin_path())
        .args(["scan", root.as_str()])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(2));

    cleanup(&root);
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_rloc")
}

fn temp_dir(test_name: &str) -> Utf8PathBuf {
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

fn cleanup(root: &Utf8Path) {
    if root.exists() {
        fs::remove_dir_all(root.as_std_path()).unwrap();
    }
}
