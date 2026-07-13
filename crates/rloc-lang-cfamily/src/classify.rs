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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScannerState {
    Code,
    String { escaped: bool },
    Character { escaped: bool },
    BlockComment,
}

pub fn classify_file(
    path: &Utf8Path,
    language: Language,
    category: FileCategory,
    options: &ClassificationOptions,
) -> Result<BackendFileAnalysis, String> {
    let bytes = fs::read(path.as_std_path()).map_err(|error| error.to_string())?;
    let contents = String::from_utf8_lossy(&bytes);
    let mut total_lines = 0_u32;
    let mut blank_lines = 0_u32;
    let mut code_lines = 0_u32;
    let mut comment_lines = 0_u32;
    let mut mixed_lines = 0_u32;
    let mut line_explanations = Vec::new();
    let mut state = ScannerState::Code;

    for (index, line) in contents.lines().enumerate() {
        total_lines += 1;
        let raw_kind = classify_line(line, &mut state, language);
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
                reason: line_reason(raw_kind, kind, language),
            });
        }
    }

    Ok(BackendFileAnalysis {
        metrics: FileMetrics::from_line_breakdown(
            path.to_path_buf(),
            language,
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

fn classify_line(line: &str, state: &mut ScannerState, language: Language) -> LineKind {
    clear_line_continuation_escape(state);

    if line.trim().is_empty() && matches!(*state, ScannerState::Code) {
        return LineKind::Blank;
    }

    let bytes = line.as_bytes();
    let mut has_code = carried_code(*state);
    let mut has_comment = carried_comment(*state);
    let mut index = 0;

    while index < bytes.len() {
        match *state {
            ScannerState::Code => {
                if bytes[index].is_ascii_whitespace() {
                    index += 1;
                    continue;
                }

                if starts_with(bytes, index, b"//") {
                    has_comment = true;
                    break;
                }

                if !matches!(language, Language::Zig) && starts_with(bytes, index, b"/*") {
                    has_comment = true;
                    *state = ScannerState::BlockComment;
                    index += 2;
                    continue;
                }

                if bytes[index] == b'"' {
                    has_code = true;
                    *state = ScannerState::String { escaped: false };
                    index += 1;
                    continue;
                }

                if bytes[index] == b'\'' {
                    has_code = true;
                    if matches!(language, Language::Cpp) && is_cpp_digit_separator(bytes, index) {
                        index += 1;
                        continue;
                    }
                    *state = ScannerState::Character { escaped: false };
                    index += 1;
                    continue;
                }

                has_code = true;
                index += 1;
            }
            ScannerState::String { escaped } => {
                has_code = true;
                match bytes[index] {
                    b'\\' if !escaped => {
                        *state = ScannerState::String { escaped: true };
                        index += 1;
                    }
                    b'"' if !escaped => {
                        *state = ScannerState::Code;
                        index += 1;
                    }
                    _ => {
                        *state = ScannerState::String { escaped: false };
                        index += 1;
                    }
                }
            }
            ScannerState::Character { escaped } => {
                has_code = true;
                match bytes[index] {
                    b'\\' if !escaped => {
                        *state = ScannerState::Character { escaped: true };
                        index += 1;
                    }
                    b'\'' if !escaped => {
                        *state = ScannerState::Code;
                        index += 1;
                    }
                    _ => {
                        *state = ScannerState::Character { escaped: false };
                        index += 1;
                    }
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

fn clear_line_continuation_escape(state: &mut ScannerState) {
    *state = match *state {
        ScannerState::String { escaped: true } => ScannerState::String { escaped: false },
        ScannerState::Character { escaped: true } => ScannerState::Character { escaped: false },
        other => other,
    };
}

fn is_cpp_digit_separator(bytes: &[u8], index: usize) -> bool {
    let Some(previous) = index.checked_sub(1).and_then(|offset| bytes.get(offset)) else {
        return false;
    };
    let Some(next) = bytes.get(index + 1) else {
        return false;
    };
    if !previous.is_ascii_alphanumeric() || !next.is_ascii_alphanumeric() {
        return false;
    }

    let mut token_start = index;
    while token_start > 0 {
        let candidate = bytes[token_start - 1];
        if candidate.is_ascii_alphanumeric() || matches!(candidate, b'.' | b'_') {
            token_start -= 1;
        } else {
            break;
        }
    }

    bytes[token_start].is_ascii_digit()
}

fn carried_code(state: ScannerState) -> bool {
    matches!(
        state,
        ScannerState::String { .. } | ScannerState::Character { .. }
    )
}

fn carried_comment(state: ScannerState) -> bool {
    matches!(state, ScannerState::BlockComment)
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

fn line_reason(raw_kind: LineKind, effective_kind: LineKind, language: Language) -> String {
    match (raw_kind, effective_kind) {
        (LineKind::Mixed, LineKind::Comment) => format!(
            "line contains {} code and comment segments, but classification policy excludes mixed lines from code",
            language.as_str()
        ),
        _ => line_reason_for_kind(effective_kind, language),
    }
}

fn line_reason_for_kind(kind: LineKind, language: Language) -> String {
    match kind {
        LineKind::Blank => "line contains only whitespace".to_owned(),
        LineKind::Code => format!(
            "line contains {} code or quoted literals without comments",
            language.as_str()
        ),
        LineKind::Comment => format!("line is part of a {} comment", language.as_str()),
        LineKind::Mixed => format!(
            "line contains {} code and comment segments",
            language.as_str()
        ),
    }
}
