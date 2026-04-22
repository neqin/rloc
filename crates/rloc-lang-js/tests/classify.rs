use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, Utf8Path, Utf8PathBuf,
};

#[test]
fn jsdoc_blocks_are_doc_lines() {
    let analysis = classify(
        "jsdoc_blocks_are_doc_lines",
        Language::JavaScript,
        "src/app.js",
        concat!(
            "/**\n",
            " * Greets a user.\n",
            " * Keeps docs visible.\n",
            " */\n",
            "export function greet() {\n",
            "  return \"hi\";\n",
            "}\n",
        ),
    );

    assert_eq!(analysis.metrics.doc_lines, 4);
    assert_eq!(analysis.metrics.code_lines, 3);
    assert_eq!(line_kind(&analysis, 2), "doc");
}

#[test]
fn template_literals_do_not_trigger_false_comment_detection() {
    let analysis = classify(
        "template_literals_do_not_trigger_false_comment_detection",
        Language::TypeScript,
        "src/snippet.ts",
        concat!(
            "const snippet = `https://example.com/path`;\n",
            "console.log(snippet);\n",
        ),
    );

    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
}

#[test]
fn tsx_and_jsx_inline_comments_become_mixed_lines() {
    let jsx = classify(
        "tsx_and_jsx_inline_comments_become_mixed_lines_jsx",
        Language::Jsx,
        "src/App.jsx",
        concat!(
            "export const App = () => (\n",
            "  <div>{/* note */}Hello</div>\n",
            ");\n",
        ),
    );
    let tsx = classify(
        "tsx_and_jsx_inline_comments_become_mixed_lines_tsx",
        Language::Tsx,
        "src/App.tsx",
        concat!(
            "export const App = () => (\n",
            "  <div>{/* note */}Hello</div>\n",
            ");\n",
        ),
    );

    assert_eq!(jsx.metrics.mixed_lines, 1);
    assert_eq!(tsx.metrics.mixed_lines, 1);
    assert_eq!(line_kind(&jsx, 2), "mixed");
    assert_eq!(line_kind(&tsx, 2), "mixed");
}

#[test]
fn jsdoc_blocks_can_be_counted_as_code() {
    let mut options = ClassificationOptions::default();
    options.count_doc_comments = false;

    let analysis = classify_with_options(
        "jsdoc_blocks_can_be_counted_as_code",
        Language::JavaScript,
        "src/app.js",
        concat!("/** docs */\n", "export const answer = 42;\n",),
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
        Language::TypeScript,
        "src/app.ts",
        concat!("const answer = 42; // note\n", "console.log(answer);\n",),
        options,
    );

    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(analysis.metrics.code_lines, 1);
    assert_eq!(line_kind(&analysis, 1), "comment");
}

#[test]
fn regex_literals_do_not_trigger_false_line_comment_detection() {
    let analysis = classify(
        "regex_literals_do_not_trigger_false_line_comment_detection",
        Language::TypeScript,
        "src/app.ts",
        concat!(
            "const re = /https?:\\/\\//;\n",
            "console.log(re.test(url));\n",
        ),
    );

    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
}

#[test]
fn regex_literals_do_not_trigger_false_block_comment_detection() {
    let analysis = classify(
        "regex_literals_do_not_trigger_false_block_comment_detection",
        Language::JavaScript,
        "src/app.js",
        concat!("const re = /\\/\\*foo/;\n", "console.log(re);\n",),
    );

    assert_eq!(analysis.metrics.code_lines, 2);
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
    classify_with_options(
        test_name,
        language,
        relative_path,
        contents,
        ClassificationOptions::default(),
    )
}

fn classify_with_options(
    test_name: &str,
    language: Language,
    relative_path: &str,
    contents: &str,
    options: ClassificationOptions,
) -> BackendFileAnalysis {
    let root = temp_workspace(test_name);
    let file = root.join(relative_path);
    write_file(&root, relative_path, contents);

    let analysis =
        rloc_lang_js::classify::classify_file(&file, language, FileCategory::Source, &options)
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
        "rloc-lang-js-{test_name}-{}-{}",
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
