use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rloc_core::{BackendFileAnalysis, ClassificationOptions, FileCategory, Utf8Path, Utf8PathBuf};

#[test]
fn slash_comments_and_inline_comments_are_classified() {
    let analysis = classify(
        "slash_comments_and_inline_comments_are_classified",
        concat!("// package note\n", "package main // inline note\n"),
    );

    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(analysis.metrics.mixed_lines, 1);
    assert_eq!(line_kind(&analysis, 1), "comment");
    assert_eq!(line_kind(&analysis, 2), "mixed");
}

#[test]
fn raw_strings_do_not_trigger_false_comment_detection() {
    let analysis = classify(
        "raw_strings_do_not_trigger_false_comment_detection",
        concat!(
            "package main\n",
            "var template = `// not comment\n",
            "/* still string */`\n",
        ),
    );

    assert_eq!(analysis.metrics.code_lines, 3);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 2), "code");
}

#[test]
fn rune_literals_do_not_trigger_false_comment_detection() {
    let analysis = classify(
        "rune_literals_do_not_trigger_false_comment_detection",
        concat!("package main\n", "var slash = '/'\n", "var star = '*'\n",),
    );

    assert_eq!(analysis.metrics.code_lines, 3);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 2), "code");
}

fn classify(test_name: &str, contents: &str) -> BackendFileAnalysis {
    let root = temp_workspace(test_name);
    let file = root.join("main.go");
    write_file(&root, "main.go", contents);

    let analysis = rloc_lang_go::classify::classify_file(
        &file,
        FileCategory::Source,
        &ClassificationOptions::default(),
    )
    .unwrap();

    cleanup_workspace(&root);
    analysis
}

fn line_kind<'a>(analysis: &'a BackendFileAnalysis, line_number: u32) -> &'a str {
    analysis
        .line_explanations
        .iter()
        .find(|line| line.line_number == line_number)
        .map(|line| line.kind.as_str())
        .unwrap()
}

fn temp_workspace(test_name: &str) -> Utf8PathBuf {
    let unique = format!(
        "rloc-lang-go-{test_name}-{}-{}",
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
