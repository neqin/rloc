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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CssState {
    Code,
    SingleQuote { escaped: bool },
    DoubleQuote { escaped: bool },
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
    let mut html_comment = false;
    let mut css_state = CssState::Code;

    for (index, line) in contents.lines().enumerate() {
        total_lines += 1;
        let raw_kind = match language {
            Language::Html => classify_html_line(line, &mut html_comment),
            Language::Css => classify_css_line(line, &mut css_state),
            other => return Err(format!("unsupported web language {other}")),
        };
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
                reason: line_reason(language, raw_kind, kind).to_owned(),
            });
        }
    }

    Ok(BackendFileAnalysis {
        metrics: FileMetrics::from_line_breakdown(
            path.to_path_buf(),
            language,
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

fn classify_html_line(line: &str, in_comment: &mut bool) -> LineKind {
    if line.trim().is_empty() && !*in_comment {
        return LineKind::Blank;
    }

    let bytes = line.as_bytes();
    let mut has_code = false;
    let mut has_comment = *in_comment;
    let mut index = 0;

    while index < bytes.len() {
        if *in_comment {
            has_comment = true;
            if starts_with(bytes, index, b"-->") {
                *in_comment = false;
                index += 3;
            } else {
                index += 1;
            }
            continue;
        }

        if starts_with(bytes, index, b"<!--") {
            has_comment = true;
            *in_comment = true;
            index += 4;
            continue;
        }

        if !bytes[index].is_ascii_whitespace() {
            has_code = true;
        }
        index += 1;
    }

    match (has_code, has_comment) {
        (false, false) => LineKind::Blank,
        (true, false) => LineKind::Code,
        (false, true) => LineKind::Comment,
        (true, true) => LineKind::Mixed,
    }
}

fn classify_css_line(line: &str, state: &mut CssState) -> LineKind {
    if line.trim().is_empty() && matches!(*state, CssState::Code) {
        return LineKind::Blank;
    }

    let bytes = line.as_bytes();
    let mut has_code = matches!(
        *state,
        CssState::SingleQuote { .. } | CssState::DoubleQuote { .. }
    );
    let mut has_comment = matches!(*state, CssState::BlockComment);
    let mut index = 0;

    while index < bytes.len() {
        match *state {
            CssState::Code => {
                if bytes[index].is_ascii_whitespace() {
                    index += 1;
                    continue;
                }

                if starts_with(bytes, index, b"/*") {
                    has_comment = true;
                    *state = CssState::BlockComment;
                    index += 2;
                    continue;
                }

                if bytes[index] == b'\'' {
                    has_code = true;
                    *state = CssState::SingleQuote { escaped: false };
                    index += 1;
                    continue;
                }

                if bytes[index] == b'"' {
                    has_code = true;
                    *state = CssState::DoubleQuote { escaped: false };
                    index += 1;
                    continue;
                }

                has_code = true;
                index += 1;
            }
            CssState::SingleQuote { escaped } => {
                has_code = true;
                match bytes[index] {
                    b'\\' if !escaped => {
                        *state = CssState::SingleQuote { escaped: true };
                        index += 1;
                    }
                    b'\'' if !escaped => {
                        *state = CssState::Code;
                        index += 1;
                    }
                    _ => {
                        *state = CssState::SingleQuote { escaped: false };
                        index += 1;
                    }
                }
            }
            CssState::DoubleQuote { escaped } => {
                has_code = true;
                match bytes[index] {
                    b'\\' if !escaped => {
                        *state = CssState::DoubleQuote { escaped: true };
                        index += 1;
                    }
                    b'"' if !escaped => {
                        *state = CssState::Code;
                        index += 1;
                    }
                    _ => {
                        *state = CssState::DoubleQuote { escaped: false };
                        index += 1;
                    }
                }
            }
            CssState::BlockComment => {
                has_comment = true;
                if starts_with(bytes, index, b"*/") {
                    *state = CssState::Code;
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

fn line_kind_name(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Blank => "blank",
        LineKind::Code => "code",
        LineKind::Comment => "comment",
        LineKind::Mixed => "mixed",
    }
}

fn line_reason(language: Language, raw_kind: LineKind, effective_kind: LineKind) -> &'static str {
    match (raw_kind, effective_kind) {
        (LineKind::Mixed, LineKind::Comment) => match language {
            Language::Html => {
                "line contains HTML markup/text and comment segments, but classification policy excludes mixed lines from code"
            }
            Language::Css => {
                "line contains CSS code and comment segments, but classification policy excludes mixed lines from code"
            }
            _ => unreachable!(),
        },
        _ => match language {
            Language::Html => html_reason_for_kind(effective_kind),
            Language::Css => css_reason_for_kind(effective_kind),
            _ => unreachable!(),
        },
    }
}

fn html_reason_for_kind(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Blank => "line contains only whitespace",
        LineKind::Code => "line contains HTML markup or text content without comments",
        LineKind::Comment => "line is part of an HTML comment",
        LineKind::Mixed => "line contains HTML markup/text and comment segments",
    }
}

fn css_reason_for_kind(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Blank => "line contains only whitespace",
        LineKind::Code => {
            "line contains CSS selectors, declarations, or quoted content without comments"
        }
        LineKind::Comment => "line is part of a CSS comment",
        LineKind::Mixed => "line contains CSS code and comment segments",
    }
}
