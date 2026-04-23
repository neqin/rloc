use std::fs;

use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, FileMetrics, Language, LineBreakdown,
    LineExplanation, Utf8Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Blank,
    Code,
    Comment,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScannerState {
    Code,
    SingleQuote,
    DoubleQuote,
    DollarQuote(Vec<u8>),
    BlockComment,
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
    let mut state = ScannerState::Code;

    for (index, line) in contents.lines().enumerate() {
        let raw_kind = classify_line(line, &mut state);
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
            LineBreakdown {
                total: total_lines,
                blank: blank_lines,
                code: code_lines,
                comment: comment_lines,
                mixed: mixed_lines,
                ..LineBreakdown::default()
            },
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

fn classify_line(line: &str, state: &mut ScannerState) -> LineKind {
    if line.trim().is_empty() && matches!(state, ScannerState::Code) {
        return LineKind::Blank;
    }

    let mut has_code = carried_code(state);
    let mut has_comment = carried_comment(state);
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        match state {
            ScannerState::Code => {
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
                    *state = ScannerState::BlockComment;
                    index += 2;
                    continue;
                }

                if let Some(delimiter) = match_dollar_quote_start(bytes, index) {
                    has_code = true;
                    index += delimiter.len();
                    *state = ScannerState::DollarQuote(delimiter);
                    continue;
                }

                if bytes[index] == b'\'' {
                    has_code = true;
                    *state = ScannerState::SingleQuote;
                    index += 1;
                    continue;
                }

                if bytes[index] == b'"' {
                    has_code = true;
                    *state = ScannerState::DoubleQuote;
                    index += 1;
                    continue;
                }

                has_code = true;
                index += 1;
            }
            ScannerState::SingleQuote => {
                has_code = true;
                if bytes[index] == b'\'' {
                    if bytes.get(index + 1) == Some(&b'\'') {
                        index += 2;
                    } else {
                        *state = ScannerState::Code;
                        index += 1;
                    }
                } else {
                    index += 1;
                }
            }
            ScannerState::DoubleQuote => {
                has_code = true;
                if bytes[index] == b'"' {
                    if bytes.get(index + 1) == Some(&b'"') {
                        index += 2;
                    } else {
                        *state = ScannerState::Code;
                        index += 1;
                    }
                } else {
                    index += 1;
                }
            }
            ScannerState::DollarQuote(delimiter) => {
                has_code = true;
                if starts_with(bytes, index, delimiter) {
                    index += delimiter.len();
                    *state = ScannerState::Code;
                } else {
                    index += 1;
                }
            }
            ScannerState::BlockComment => {
                has_comment = true;
                if starts_with(bytes, index, b"*/") {
                    *state = ScannerState::Code;
                    index += 2;
                } else {
                    index += 1;
                }
            }
        }
    }

    match (has_code, has_comment) {
        (false, false) => LineKind::Blank,
        (true, false) => LineKind::Code,
        (false, true) => LineKind::Comment,
        (true, true) => LineKind::Mixed,
    }
}

fn starts_with(bytes: &[u8], index: usize, needle: &[u8]) -> bool {
    bytes
        .get(index..index + needle.len())
        .is_some_and(|window| window == needle)
}

fn match_dollar_quote_start(bytes: &[u8], index: usize) -> Option<Vec<u8>> {
    if bytes.get(index) != Some(&b'$') {
        return None;
    }

    let mut cursor = index + 1;
    while let Some(&byte) = bytes.get(cursor) {
        if byte == b'$' {
            return Some(bytes[index..=cursor].to_vec());
        }
        if !(byte.is_ascii_alphanumeric() || byte == b'_') {
            return None;
        }
        cursor += 1;
    }

    None
}

fn carried_code(state: &ScannerState) -> bool {
    matches!(
        state,
        ScannerState::SingleQuote | ScannerState::DoubleQuote | ScannerState::DollarQuote(_)
    )
}

fn carried_comment(state: &ScannerState) -> bool {
    matches!(state, ScannerState::BlockComment)
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
