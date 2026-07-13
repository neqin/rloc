use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, LanguageBackend, Utf8Path,
    Utf8PathBuf,
};

#[test]
fn line_block_code_and_mixed_lines_are_classified() {
    let analysis = classify(
        "line_block_code_and_mixed_lines_are_classified",
        concat!(
            "# line comment\n",
            "<# block comment\n",
            "continued\n",
            "#>\n",
            "$value = 1\n",
            "$next = 2 # trailing comment\n",
        ),
    );

    assert_eq!(analysis.metrics.language, Language::PowerShell);
    assert_eq!(analysis.metrics.comment_lines, 4);
    assert_eq!(analysis.metrics.code_lines, 1);
    assert_eq!(analysis.metrics.mixed_lines, 1);
    assert_eq!(line_kind(&analysis, 1), "comment");
    assert_eq!(line_kind(&analysis, 5), "code");
    assert_eq!(line_kind(&analysis, 6), "mixed");
}

#[test]
fn comment_markers_inside_strings_are_code() {
    let analysis = classify(
        "comment_markers_inside_strings_are_code",
        concat!("$double = \"a#b\"\n", "$single = '<# #>'\n"),
    );

    assert_eq!(analysis.metrics.code_lines, 2);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
    assert_eq!(line_kind(&analysis, 2), "code");
}

#[test]
fn backtick_escaped_quote_keeps_hash_inside_the_string() {
    let analysis = classify(
        "backtick_escaped_quote_keeps_hash_inside_the_string",
        concat!(r#"$value = "x `" y # z""#, "\n"),
    );

    assert_eq!(analysis.metrics.code_lines, 1);
    assert_eq!(analysis.metrics.comment_lines, 0);
    assert_eq!(analysis.metrics.mixed_lines, 0);
    assert_eq!(line_kind(&analysis, 1), "code");
}

#[test]
fn backend_descriptor_reports_ps1() {
    assert_eq!(
        rloc_lang_powershell::backend().descriptor().extensions,
        &["ps1"]
    );
}

fn classify(test_name: &str, contents: &str) -> BackendFileAnalysis {
    let root = temp_workspace(test_name);
    let file = root.join("script.ps1");
    fs::write(file.as_std_path(), contents).unwrap();

    let analysis = rloc_lang_powershell::backend()
        .classify_file(
            &file,
            FileCategory::Script,
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
        "rloc-lang-powershell-{test_name}-{}-{}",
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
