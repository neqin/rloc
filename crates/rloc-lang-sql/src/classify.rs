use std::fs;

use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, FileMetrics, Language,
    LineExplanation, Utf8Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Blank,
    Code,
    Comment,
    Mixed,
}

pub fn classify_file(
    path: &Utf8Path,
    category: FileCategory,
    options: &ClassificationOptions,
) -> Result<BackendFileAnalysis, String> {
    let bytes = fs::read(path.as_std_path()).map_err(|error| error.to_string())?;
    let contents = String::from_utf8_lossy(&bytes);
    let mut blank_lines = 0_u32;
    let mut code_lines = 0_u32;
    let mut comment_lines = 0_u32;
    let mut mixed_lines = 0_u32;
    let mut line_explanations = Vec::new();
    let mut in_block_comment = false;

    for (index, line) in contents.lines().enumerate() {
        let (raw_kind, next_in_block_comment) = classify_line(line, in_block_comment);
        in_block_comment = next_in_block_comment;
        let kind = normalize_kind(raw_kind, options);
        match kind {
            LineKind::Blank => blank_lines += 1,
            LineKind::Code => code_lines += 1,
            LineKind::Comment => comment_lines += 1,
            LineKind::Mixed => mixed_lines += 1,
        }

        if !matches!(kind, LineKind::Blank) {
            line_explanations.push(LineExplanation {
                line_number: (index + 1) as u32,
                kind: line_kind_name(kind).to_owned(),
                snippet: line.trim().to_owned(),
                reason: line_reason(raw_kind, kind).to_owned(),
            });
        }
    }

    let total_lines = contents.lines().count() as u32;

    Ok(BackendFileAnalysis {
        metrics: FileMetrics::from_line_breakdown(
            path.to_path_buf(),
            Language::Sql,
            category,
            bytes.len() as u64,
            total_lines,
            blank_lines,
            code_lines,
            comment_lines,
            0,
            mixed_lines,
            0,
        ),
        line_explanations,
        warnings: Vec::new(),
    })
}

fn normalize_kind(kind: LineKind, options: &ClassificationOptions) -> LineKind {
    match kind {
        LineKind::Mixed if !options.mixed_lines_as_code => LineKind::Comment,
        other => other,
    }
}

fn classify_line(line: &str, in_block_comment: bool) -> (LineKind, bool) {
    if line.trim().is_empty() && !in_block_comment {
        return (LineKind::Blank, false);
    }

    let bytes = line.as_bytes();
    let mut has_code = false;
    let mut has_comment = in_block_comment;
    let mut block_comment = in_block_comment;
    let mut index = 0;

    while index < bytes.len() {
        if block_comment {
            has_comment = true;
            if starts_with(bytes, index, b"*/") {
                block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }

        if starts_with(bytes, index, b"--") {
            has_comment = true;
            break;
        }

        if starts_with(bytes, index, b"/*") {
            has_comment = true;
            block_comment = true;
            index += 2;
            continue;
        }

        if bytes[index] == b'\'' {
            has_code = true;
            index = skip_quoted(bytes, index, b'\'');
            continue;
        }

        if bytes[index] == b'"' {
            has_code = true;
            index = skip_quoted(bytes, index, b'"');
            continue;
        }

        has_code = true;
        index += 1;
    }

    let kind = match (has_code, has_comment) {
        (false, false) => LineKind::Blank,
        (true, false) => LineKind::Code,
        (false, true) => LineKind::Comment,
        (true, true) => LineKind::Mixed,
    };

    (kind, block_comment)
}

fn skip_quoted(bytes: &[u8], mut index: usize, quote: u8) -> usize {
    index += 1;

    while index < bytes.len() {
        if bytes[index] == quote {
            if bytes.get(index + 1) == Some(&quote) {
                index += 2;
                continue;
            }
            index += 1;
            break;
        }
        index += 1;
    }

    index
}

fn starts_with(bytes: &[u8], index: usize, needle: &[u8]) -> bool {
    bytes
        .get(index..index + needle.len())
        .is_some_and(|window| window == needle)
}

fn line_kind_name(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Blank => "blank",
        LineKind::Code => "code",
        LineKind::Comment => "comment",
        LineKind::Mixed => "mixed",
    }
}

fn line_reason(raw_kind: LineKind, effective_kind: LineKind) -> &'static str {
    match (raw_kind, effective_kind) {
        (LineKind::Mixed, LineKind::Comment) => {
            "line contains SQL code and comment segments, but classification policy excludes mixed lines from code"
        }
        _ => line_reason_for_kind(effective_kind),
    }
}

fn line_reason_for_kind(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Blank => "line contains only whitespace",
        LineKind::Code => "line contains SQL code or quoted SQL content without comments",
        LineKind::Comment => "line is part of a SQL comment",
        LineKind::Mixed => "line contains SQL code and comment segments",
    }
}
