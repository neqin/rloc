use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use camino::{Utf8Path, Utf8PathBuf};

#[test]
fn detect_lists_unsupported_paths_when_requested() {
    let root = temp_workspace("detect_lists_unsupported_paths_when_requested");
    write_file(&root, "src/lib.rs", "fn main() {}\n");
    write_file(&root, "README.txt", "ignored\n");
    write_file(&root, "notes.log", "ignored\n");

    let output = Command::new(env!("CARGO_BIN_EXE_rloc"))
        .args(["detect", root.as_str(), "--list-unsupported"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("warnings:"));
    assert!(stdout.contains("2 files with unsupported extensions were skipped"));
    assert!(stdout.contains("README.txt: unsupported extension skipped"));
    assert!(stdout.contains("notes.log: unsupported extension skipped"));

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
