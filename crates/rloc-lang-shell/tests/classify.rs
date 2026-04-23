use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rloc_core::{BackendFileAnalysis, ClassificationOptions, FileCategory, Utf8Path, Utf8PathBuf};

#[test]
fn shebang_is_counted_as_code() {
    let analysis = classify(
        "shebang_is_counted_as_code",
        "script.sh",
        concat!("#!/usr/bin/env bash\n", "echo hello\n"),
    );

    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
}

#[test]
fn inline_hash_comment_becomes_mixed() {
    let analysis = classify(
        "inline_hash_comment_becomes_mixed",
        "script.sh",
        concat!("echo hello # note\n", "printf '%s\\n' done\n"),
    );

    assert_eq!(analysis.metrics.code_lines, 1);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 1);
    assert_eq!(line_kind(&analysis, 1), "mixed");
}

#[test]
fn hash_inside_quotes_does_not_start_a_comment() {
    let analysis = classify(
        "hash_inside_quotes_does_not_start_a_comment",
        ".bashrc",
        concat!("export PROMPT='# ready'\n", "echo \"# still code\"\n",),
    );

    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
}

#[test]
fn heredoc_bodies_are_not_classified_as_comments() {
    let analysis = classify(
        "heredoc_bodies_are_not_classified_as_comments",
        "script.sh",
        concat!(
            "cat <<'EOF'\n",
            "# not a comment\n",
            "value=true\n",
            "EOF\n",
        ),
    );

    assert_eq!(analysis.metrics.code_lines, 4);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 2), "code");
}

fn classify(test_name: &str, relative_path: &str, contents: &str) -> BackendFileAnalysis {
    let root = temp_workspace(test_name);
    let file = root.join(relative_path);
    write_file(&root, relative_path, contents);

    let analysis = rloc_lang_shell::classify::classify_file(
        &file,
        FileCategory::Source,
        &ClassificationOptions::default(),
    )
    .unwrap();

    cleanup_workspace(&root);
    analysis
}

fn line_kind(analysis: &BackendFileAnalysis, line_number: u32) -> &str {
    analysis
        .line_explanations
        .iter()
        .find(|line| line.line_number == line_number)
        .map(|line| line.kind.as_str())
        .unwrap()
}

fn temp_workspace(test_name: &str) -> Utf8PathBuf {
    let unique = format!(
        "rloc-lang-shell-{test_name}-{}-{}",
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
