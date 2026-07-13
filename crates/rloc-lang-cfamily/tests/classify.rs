use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, LanguageBackend, Utf8Path,
    Utf8PathBuf,
};
use rloc_lang_cfamily::{
    c_backend, cpp_backend, java_backend, objective_c_backend, swift_backend, zig_backend,
};

#[test]
fn c_comments_literals_and_mixed_lines_are_classified() {
    let analysis = classify(
        "c_comments_literals_and_mixed_lines_are_classified",
        "main.c",
        &c_backend(),
        concat!(
            "// line comment\n",
            "/* block comment\n",
            "continued */\n",
            "int value = 1;\n",
            "const char *url = \"http://example/*path*/\";\n",
            "char slash = '/';\n",
            "int next = 2; // trailing comment\n",
        ),
    );

    assert_eq!(analysis.metrics.language, Language::C);
    assert_eq!(analysis.metrics.comment_lines, 3);
    assert_eq!(analysis.metrics.code_lines, 3);
    assert_eq!(analysis.metrics.mixed_lines, 1);
    assert_eq!(line_kind(&analysis, 5), "code");
    assert_eq!(line_kind(&analysis, 6), "code");
    assert_eq!(line_kind(&analysis, 7), "mixed");
}

#[test]
fn every_c_family_backend_uses_the_shared_scanner() {
    for (name, extension, backend, language) in [
        (
            "cpp",
            "cpp",
            Box::new(cpp_backend()) as Box<dyn LanguageBackend>,
            Language::Cpp,
        ),
        (
            "java",
            "java",
            Box::new(java_backend()) as Box<dyn LanguageBackend>,
            Language::Java,
        ),
        (
            "swift",
            "swift",
            Box::new(swift_backend()) as Box<dyn LanguageBackend>,
            Language::Swift,
        ),
        (
            "objective_c",
            "m",
            Box::new(objective_c_backend()) as Box<dyn LanguageBackend>,
            Language::ObjectiveC,
        ),
    ] {
        let analysis = classify(
            name,
            &format!("main.{extension}"),
            backend.as_ref(),
            "value = 1;\n// note\nvalue = 2; /* note */\n",
        );

        assert_eq!(analysis.metrics.language, language);
        assert_eq!(analysis.metrics.code_lines, 1);
        assert_eq!(analysis.metrics.comment_lines, 1);
        assert_eq!(analysis.metrics.mixed_lines, 1);
    }
}

#[test]
fn zig_does_not_treat_block_markers_as_comments() {
    let analysis = classify(
        "zig_does_not_treat_block_markers_as_comments",
        "main.zig",
        &zig_backend(),
        "/* not a Zig comment */\n\\\\ multiline string content\n// Zig comment\n",
    );

    assert_eq!(analysis.metrics.language, Language::Zig);
    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
    assert_eq!(line_kind(&analysis, 2), "code");
}

#[test]
fn cpp_digit_separators_do_not_open_character_literals() {
    let analysis = classify(
        "cpp_digit_separators_do_not_open_character_literals",
        "main.cpp",
        &cpp_backend(),
        "auto value = 1'000; // trailing\n// next comment\n",
    );

    assert_eq!(analysis.metrics.mixed_lines, 1);
    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(line_kind(&analysis, 1), "mixed");
    assert_eq!(line_kind(&analysis, 2), "comment");
}

#[test]
fn continued_c_string_clears_escape_before_the_next_line() {
    let analysis = classify(
        "continued_c_string_clears_escape_before_the_next_line",
        "main.c",
        &c_backend(),
        concat!(
            "const char *value = \"continued\\\n",
            "\"; // trailing comment\n",
            "// next comment\n",
        ),
    );

    assert_eq!(analysis.metrics.code_lines, 1);
    assert_eq!(analysis.metrics.mixed_lines, 1);
    assert_eq!(analysis.metrics.comment_lines, 1);
    assert_eq!(line_kind(&analysis, 2), "mixed");
    assert_eq!(line_kind(&analysis, 3), "comment");
}

#[test]
fn backend_descriptors_report_expected_extensions() {
    assert_eq!(c_backend().descriptor().extensions, &["c", "h"]);
    assert_eq!(cpp_backend().descriptor().extensions, &["cpp"]);
    assert_eq!(java_backend().descriptor().extensions, &["java"]);
    assert_eq!(swift_backend().descriptor().extensions, &["swift"]);
    assert_eq!(objective_c_backend().descriptor().extensions, &["m"]);
    assert_eq!(zig_backend().descriptor().extensions, &["zig"]);
}

fn classify(
    test_name: &str,
    file_name: &str,
    backend: &dyn LanguageBackend,
    contents: &str,
) -> BackendFileAnalysis {
    let root = temp_workspace(test_name);
    let file = root.join(file_name);
    fs::write(file.as_std_path(), contents).unwrap();

    let analysis = backend
        .classify_file(
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
        "rloc-lang-cfamily-{test_name}-{}-{}",
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

fn cleanup_workspace(root: &Utf8Path) {
    if root.exists() {
        fs::remove_dir_all(root.as_std_path()).unwrap();
    }
}
