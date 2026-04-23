use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rloc_core::{BackendFileAnalysis, ClassificationOptions, FileCategory, Utf8Path, Utf8PathBuf};

#[test]
fn double_dash_comments_are_comment_or_mixed_lines() {
    let analysis = classify(
        "double_dash_comments_are_comment_or_mixed_lines",
        "schema.sql",
        concat!("-- schema note\n", "select 1 -- inline note\n"),
    );

    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(analysis.metrics.mixed_lines, 1);
    assert_eq!(line_kind(&analysis, 1), "comment");
    assert_eq!(line_kind(&analysis, 2), "mixed");
}

#[test]
fn block_comments_span_multiple_lines() {
    let analysis = classify(
        "block_comments_span_multiple_lines",
        "schema.sql",
        concat!("/*\n", " * schema note\n", " */\n", "select 1;\n",),
    );

    assert_eq!(analysis.metrics.comment_lines, 3);
    assert_eq!(analysis.metrics.code_lines, 1);
    assert_eq!(line_kind(&analysis, 2), "comment");
}

#[test]
fn comment_markers_inside_strings_do_not_start_comments() {
    let analysis = classify(
        "comment_markers_inside_strings_do_not_start_comments",
        "query.psql",
        concat!(
            "select '-- not comment' as note;\n",
            "select \"/* not comment */\" as ident;\n",
        ),
    );

    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
}

#[test]
fn dollar_quoted_strings_ignore_comment_markers() {
    let analysis = classify(
        "dollar_quoted_strings_ignore_comment_markers",
        "query.psql",
        concat!(
            "select $$-- not a comment$$;\n",
            "select $tag$/* still not comment */$tag$;\n",
        ),
    );

    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
}

fn classify(test_name: &str, relative_path: &str, contents: &str) -> BackendFileAnalysis {
    let root = temp_workspace(test_name);
    let file = root.join(relative_path);
    write_file(&root, relative_path, contents);

    let analysis = rloc_lang_sql::classify::classify_file(
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
        "rloc-lang-sql-{test_name}-{}-{}",
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
