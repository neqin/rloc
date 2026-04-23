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
struct HereDocState {
    delimiter: String,
    strip_tabs: bool,
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
    let mut heredoc = None;

    for (index, line) in contents.lines().enumerate() {
        let raw_kind = classify_line(line, &mut heredoc);
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
            Language::Shell,
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

fn classify_line(line: &str, heredoc: &mut Option<HereDocState>) -> LineKind {
    if let Some(state) = heredoc.clone() {
        if is_heredoc_terminator(line, &state) {
            *heredoc = None;
        }
        return if line.trim().is_empty() {
            LineKind::Blank
        } else {
            LineKind::Code
        };
    }

    if line.trim().is_empty() {
        return LineKind::Blank;
    }

    let bytes = line.as_bytes();
    let mut has_code = false;
    let mut has_comment = false;
    let mut in_single = false;
    let mut in_double = false;
    let mut double_escaped = false;
    let mut index = 0;
    let mut next_heredoc = None;

    while index < bytes.len() {
        let byte = bytes[index];

        if in_single {
            has_code = true;
            if byte == b'\'' {
                in_single = false;
            }
            index += 1;
            continue;
        }

        if in_double {
            has_code = true;
            match byte {
                b'\\' if !double_escaped => {
                    double_escaped = true;
                }
                b'"' if !double_escaped => {
                    in_double = false;
                }
                _ => {
                    double_escaped = false;
                }
            }
            index += 1;
            continue;
        }

        match byte {
            b'<' => {
                if let Some((state, next_index)) = match_heredoc_start(bytes, index) {
                    has_code = true;
                    next_heredoc = Some(state);
                    index = next_index;
                    continue;
                }
            }
            b'#' if is_shebang(bytes, index) => {
                has_code = true;
                break;
            }
            b'#' if starts_comment(bytes, index) => {
                has_comment = true;
                break;
            }
            b'\'' => {
                has_code = true;
                in_single = true;
            }
            b'"' => {
                has_code = true;
                in_double = true;
                double_escaped = false;
            }
            byte if !byte.is_ascii_whitespace() => {
                has_code = true;
            }
            _ => {}
        }

        index += 1;
    }

    *heredoc = next_heredoc;

    match (has_code, has_comment) {
        (false, false) => LineKind::Blank,
        (true, true) => LineKind::Mixed,
        (false, true) => LineKind::Comment,
        (true, false) => LineKind::Code,
    }
}

fn is_shebang(bytes: &[u8], index: usize) -> bool {
    index == 0 && bytes.get(index + 1) == Some(&b'!')
}

fn starts_comment(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }

    matches!(bytes[index - 1], b' ' | b'\t' | b';')
}

fn match_heredoc_start(bytes: &[u8], index: usize) -> Option<(HereDocState, usize)> {
    if !starts_with(bytes, index, b"<<") {
        return None;
    }

    let mut cursor = index + 2;
    let strip_tabs = if bytes.get(cursor) == Some(&b'-') {
        cursor += 1;
        true
    } else {
        false
    };

    while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
        cursor += 1;
    }

    let &first = bytes.get(cursor)?;

    let (delimiter, next_index) = match first {
        b'\'' | b'"' => {
            let quote = first;
            cursor += 1;
            let start = cursor;
            while let Some(&byte) = bytes.get(cursor) {
                if byte == quote {
                    let delimiter = String::from_utf8(bytes[start..cursor].to_vec()).ok()?;
                    return Some((
                        HereDocState {
                            delimiter,
                            strip_tabs,
                        },
                        cursor + 1,
                    ));
                }
                cursor += 1;
            }
            return None;
        }
        _ => {
            let start = cursor;
            while let Some(&byte) = bytes.get(cursor) {
                if byte.is_ascii_whitespace() || matches!(byte, b';' | b'|' | b'&' | b'#') {
                    break;
                }
                cursor += 1;
            }
            let delimiter = String::from_utf8(bytes[start..cursor].to_vec()).ok()?;
            (delimiter, cursor)
        }
    };

    if delimiter.is_empty() {
        return None;
    }

    Some((
        HereDocState {
            delimiter,
            strip_tabs,
        },
        next_index,
    ))
}

fn is_heredoc_terminator(line: &str, state: &HereDocState) -> bool {
    let candidate = if state.strip_tabs {
        line.trim_start_matches('\t')
    } else {
        line
    };

    candidate.trim_end() == state.delimiter
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
            "line contains shell code and comment segments, but classification policy excludes mixed lines from code"
        }
        _ => line_reason_for_kind(effective_kind),
    }
}

fn line_reason_for_kind(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Blank => "line contains only whitespace",
        LineKind::Code => "line contains shell code or quoted content without a comment",
        LineKind::Comment => "line is a shell comment line",
        LineKind::Mixed => "line contains shell code followed by a comment",
    }
}
