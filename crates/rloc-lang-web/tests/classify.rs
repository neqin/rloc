use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, Utf8Path, Utf8PathBuf,
};

#[test]
fn html_comments_are_comment_lines_and_markup_is_code() {
    let analysis = classify(
        "html_comments_are_comment_lines_and_markup_is_code",
        Language::Html,
        "index.html",
        concat!("<div>Hello</div>\n", "<!-- note -->\n"),
    );

    assert_eq!(analysis.metrics.code_lines, 1);
    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(line_kind(&analysis, 1), "code");
    assert_eq!(line_kind(&analysis, 2), "comment");
}

#[test]
fn html_text_content_is_counted_as_code() {
    let analysis = classify(
        "html_text_content_is_counted_as_code",
        Language::Html,
        "page.gohtml",
        concat!("<main>\n", "  Hello world\n", "</main>\n"),
    );

    assert_eq!(analysis.metrics.code_lines, 3);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(line_kind(&analysis, 2), "code");
}

#[test]
fn css_block_comments_are_comment_lines() {
    let analysis = classify(
        "css_block_comments_are_comment_lines",
        Language::Css,
        "app.css",
        concat!("/* theme note */\n", "body { color: red; }\n"),
    );

    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(analysis.metrics.code_lines, 1);
    assert_eq!(line_kind(&analysis, 1), "comment");
}

#[test]
fn css_comment_markers_inside_strings_do_not_start_comments() {
    let analysis = classify(
        "css_comment_markers_inside_strings_do_not_start_comments",
        Language::Css,
        "app.css",
        concat!("body::before { content: \"/* not comment */\"; }\n"),
    );

    assert_eq!(analysis.metrics.code_lines, 1);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
}

fn classify(
    test_name: &str,
    language: Language,
    relative_path: &str,
    contents: &str,
) -> BackendFileAnalysis {
    let root = temp_workspace(test_name);
    let file = root.join(relative_path);
    write_file(&root, relative_path, contents);

    let analysis = rloc_lang_web::classify::classify_file(
        &file,
        language,
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
        "rloc-lang-web-{test_name}-{}-{}",
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
