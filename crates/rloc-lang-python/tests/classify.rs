use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rloc_core::{BackendFileAnalysis, ClassificationOptions, FileCategory, Utf8Path, Utf8PathBuf};

#[test]
fn module_class_and_function_docstrings_are_doc_lines() {
    let analysis = classify(
        "module_class_and_function_docstrings_are_doc_lines",
        concat!(
            "\"\"\"module\n",
            "details\n",
            "doc\"\"\"\n",
            "class Greeter:\n",
            "    \"\"\"class\n",
            "    details\n",
            "    doc\"\"\"\n",
            "    def hello(self):\n",
            "        \"\"\"function\n",
            "        details\n",
            "        doc\"\"\"\n",
            "        return \"hi\"\n",
        ),
    );

    assert_eq!(analysis.metrics.doc_lines, 9);
    assert_eq!(analysis.metrics.code_lines, 3);
    assert_eq!(line_kind(&analysis, 2), "doc");
    assert_eq!(line_kind(&analysis, 6), "doc");
    assert_eq!(line_kind(&analysis, 10), "doc");
}

#[test]
fn regular_multiline_strings_inside_code_are_not_docstrings() {
    let analysis = classify(
        "regular_multiline_strings_inside_code_are_not_docstrings",
        concat!(
            "def build_query():\n",
            "    query = \"\"\"select\n",
            "    *\n",
            "    from widgets\"\"\"\n",
            "    return query\n",
        ),
    );

    assert_eq!(analysis.metrics.doc_lines, 0);
    assert_eq!(analysis.metrics.code_lines, 5);
    assert_eq!(line_kind(&analysis, 2), "code");
    assert_eq!(line_kind(&analysis, 3), "code");
    assert_eq!(line_kind(&analysis, 4), "code");
}

#[test]
fn inline_hash_comments_are_mixed_lines() {
    let analysis = classify(
        "inline_hash_comments_are_mixed_lines",
        concat!(
            "def build():\n",
            "    value = 1  # note\n",
            "    return value\n",
        ),
    );

    assert_eq!(analysis.metrics.mixed_lines, 1);
    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(line_kind(&analysis, 2), "mixed");
}

#[test]
fn mixed_lines_can_be_counted_as_comments() {
    let mut options = ClassificationOptions::default();
    options.mixed_lines_as_code = false;

    let analysis = classify_with_options(
        "mixed_lines_can_be_counted_as_comments",
        concat!(
            "def build():\n",
            "    value = 1  # note\n",
            "    return value\n",
        ),
        options,
    );

    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(line_kind(&analysis, 2), "comment");
}

#[test]
fn docstrings_can_be_counted_as_code() {
    let mut options = ClassificationOptions::default();
    options.count_docstrings_as_comments = false;

    let analysis = classify_with_options(
        "docstrings_can_be_counted_as_code",
        concat!("\"\"\"module\n", "docs\n", "block\"\"\"\n",),
        options,
    );

    assert_eq!(analysis.metrics.doc_lines, 0);
    assert_eq!(analysis.metrics.code_lines, 3);
    assert_eq!(line_kind(&analysis, 2), "code");
}

fn classify(test_name: &str, contents: &str) -> BackendFileAnalysis {
    classify_with_options(test_name, contents, ClassificationOptions::default())
}

fn classify_with_options(
    test_name: &str,
    contents: &str,
    options: ClassificationOptions,
) -> BackendFileAnalysis {
    let root = temp_workspace(test_name);
    let file = root.join("src/module.py");
    write_file(&root, "src/module.py", contents);

    let analysis =
        rloc_lang_python::classify::classify_file(&file, FileCategory::Source, &options).unwrap();

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
        "rloc-lang-python-{test_name}-{}-{}",
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
