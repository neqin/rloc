use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rloc_core::{BackendFileAnalysis, ClassificationOptions, FileCategory, Utf8Path, Utf8PathBuf};

#[test]
fn classifies_nested_block_comments_and_doc_comments() {
    let analysis = classify(
        "classifies_nested_block_comments_and_doc_comments",
        "/**\n\
         * crate docs\n\
         */\n\
        fn main() {\n\
            /* outer\n\
               /* nested */\n\
               still outer\n\
            */\n\
        }\n",
    );

    assert_eq!(analysis.metrics.doc_lines, 3);
    assert_eq!(analysis.metrics.comment_lines, 4);
    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(line_kind(&analysis, 1), "doc");
    assert_eq!(line_kind(&analysis, 2), "doc");
    assert_eq!(line_kind(&analysis, 3), "doc");
    assert_eq!(line_kind(&analysis, 7), "comment");
    assert_eq!(line_kind(&analysis, 8), "comment");
}

#[test]
fn ignores_comment_markers_inside_raw_strings() {
    let analysis = classify(
        "ignores_comment_markers_inside_raw_strings",
        "fn main() {\n\
            let template = r#\"// not comment /* still string */\"#;\n\
        }\n",
    );

    assert_eq!(analysis.metrics.code_lines, 3);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 2), "code");
}

#[test]
fn detects_inline_comment_as_mixed() {
    let analysis = classify(
        "detects_inline_comment_as_mixed",
        "fn main() {\n\
            let value = /* units */ 1;\n\
        }\n",
    );

    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 1);
    assert_eq!(line_kind(&analysis, 2), "mixed");
}

#[test]
fn doc_comments_can_be_counted_as_code() {
    let mut options = ClassificationOptions::default();
    options.count_doc_comments = false;

    let analysis = classify_with_options(
        "doc_comments_can_be_counted_as_code",
        "/// crate docs\nfn main() {}\n",
        options,
    );

    assert_eq!(analysis.metrics.doc_lines, 0);
    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(line_kind(&analysis, 1), "code");
}

#[test]
fn mixed_lines_can_be_counted_as_comments() {
    let mut options = ClassificationOptions::default();
    options.mixed_lines_as_code = false;

    let analysis = classify_with_options(
        "mixed_lines_can_be_counted_as_comments",
        "fn main() {\n    let value = 1; // note\n}\n",
        options,
    );

    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(line_kind(&analysis, 2), "comment");
}

fn classify(test_name: &str, contents: &str) -> BackendFileAnalysis {
    classify_with_options(test_name, contents, default_options())
}

fn classify_with_options(
    test_name: &str,
    contents: &str,
    options: ClassificationOptions,
) -> BackendFileAnalysis {
    let root = temp_workspace(test_name);
    let file = root.join("src/lib.rs");
    write_file(&root, "src/lib.rs", contents);

    let analysis =
        rloc_lang_rust::classify::classify_file(&file, FileCategory::Source, &options).unwrap();

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

fn default_options() -> ClassificationOptions {
    ClassificationOptions::default()
}

fn temp_workspace(test_name: &str) -> Utf8PathBuf {
    let unique = format!(
        "rloc-lang-rust-{test_name}-{}-{}",
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
