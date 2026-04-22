use std::fs;

use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, FileMetrics, LineExplanation,
    Utf8Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Blank,
    Code,
    Comment,
    Doc,
    Mixed,
}

#[derive(Debug, Clone, Copy, Default)]
struct LineFlags {
    has_code: bool,
    has_comment: bool,
    has_doc: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScannerState {
    Normal,
    String { escaped: bool },
    RawString { hashes: usize },
    BlockComment { depth: usize, doc: bool },
}

pub fn classify_file(
    path: &Utf8Path,
    category: FileCategory,
    options: &ClassificationOptions,
) -> Result<BackendFileAnalysis, String> {
    let bytes = fs::read(path.as_std_path()).map_err(|error| error.to_string())?;
    let contents = String::from_utf8_lossy(&bytes);
    let mut total_lines = 0_u32;
    let mut blank_lines = 0_u32;
    let mut code_lines = 0_u32;
    let mut comment_lines = 0_u32;
    let mut doc_lines = 0_u32;
    let mut mixed_lines = 0_u32;
    let mut line_explanations = Vec::new();
    let mut state = ScannerState::Normal;

    for (index, line) in contents.lines().enumerate() {
        total_lines += 1;
        let raw_kind = classify_line(line, &mut state);
        let kind = normalize_kind(raw_kind, options);
        match kind {
            LineKind::Blank => blank_lines += 1,
            LineKind::Code => code_lines += 1,
            LineKind::Comment => comment_lines += 1,
            LineKind::Doc => doc_lines += 1,
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

    Ok(BackendFileAnalysis {
        metrics: FileMetrics::from_line_breakdown(
            path.to_path_buf(),
            rloc_core::Language::Rust,
            category,
            bytes.len() as u64,
            total_lines,
            blank_lines,
            code_lines,
            comment_lines,
            doc_lines,
            mixed_lines,
            0,
        ),
        line_explanations,
        warnings: Vec::new(),
    })
}

pub fn classifier_status() -> &'static str {
    "deterministic rust backend classifier"
}

fn normalize_kind(kind: LineKind, options: &ClassificationOptions) -> LineKind {
    match kind {
        LineKind::Doc if !options.count_doc_comments => LineKind::Code,
        LineKind::Mixed if !options.mixed_lines_as_code => LineKind::Comment,
        other => other,
    }
}

fn classify_line(line: &str, state: &mut ScannerState) -> LineKind {
    let mut flags = carried_flags(*state);
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        match *state {
            ScannerState::Normal => {
                if bytes[index].is_ascii_whitespace() {
                    index += 1;
                    continue;
                }

                if let Some(doc) = match_line_comment(bytes, index) {
                    mark_comment(&mut flags, doc);
                    break;
                }

                if let Some(doc) = match_block_comment_start(bytes, index) {
                    mark_comment(&mut flags, doc);
                    *state = ScannerState::BlockComment { depth: 1, doc };
                    index += 2;
                    continue;
                }

                if let Some(hashes) = match_raw_string_start(bytes, index) {
                    flags.has_code = true;
                    *state = ScannerState::RawString { hashes };
                    index += 2 + hashes;
                    continue;
                }

                if bytes[index] == b'"' {
                    flags.has_code = true;
                    *state = ScannerState::String { escaped: false };
                    index += 1;
                    continue;
                }

                flags.has_code = true;
                index += 1;
            }
            ScannerState::String { ref mut escaped } => {
                flags.has_code = true;
                match bytes[index] {
                    b'\\' if !*escaped => {
                        *escaped = true;
                        index += 1;
                    }
                    b'"' if !*escaped => {
                        *state = ScannerState::Normal;
                        index += 1;
                    }
                    _ => {
                        *escaped = false;
                        index += 1;
                    }
                }
            }
            ScannerState::RawString { hashes } => {
                flags.has_code = true;
                if is_raw_string_end(bytes, index, hashes) {
                    *state = ScannerState::Normal;
                    index += hashes + 1;
                } else {
                    index += 1;
                }
            }
            ScannerState::BlockComment { ref mut depth, doc } => {
                mark_comment(&mut flags, doc);

                if starts_with(bytes, index, b"/*") {
                    *depth += 1;
                    index += 2;
                } else if starts_with(bytes, index, b"*/") {
                    *depth -= 1;
                    index += 2;
                    if *depth == 0 {
                        *state = ScannerState::Normal;
                    }
                } else {
                    index += 1;
                }
            }
        }
    }

    if matches!(*state, ScannerState::Normal)
        && !flags.has_code
        && !flags.has_comment
        && !flags.has_doc
    {
        return LineKind::Blank;
    }

    if flags.has_code && (flags.has_comment || flags.has_doc) {
        return LineKind::Mixed;
    }
    if flags.has_doc {
        return LineKind::Doc;
    }
    if flags.has_comment {
        return LineKind::Comment;
    }
    if flags.has_code {
        return LineKind::Code;
    }

    LineKind::Blank
}

fn carried_flags(state: ScannerState) -> LineFlags {
    let mut flags = LineFlags::default();
    match state {
        ScannerState::Normal => {}
        ScannerState::String { .. } | ScannerState::RawString { .. } => {
            flags.has_code = true;
        }
        ScannerState::BlockComment { doc, .. } => {
            mark_comment(&mut flags, doc);
        }
    }
    flags
}

fn mark_comment(flags: &mut LineFlags, doc: bool) {
    if doc {
        flags.has_doc = true;
    } else {
        flags.has_comment = true;
    }
}

fn match_line_comment(bytes: &[u8], index: usize) -> Option<bool> {
    if !starts_with(bytes, index, b"//") {
        return None;
    }

    Some(is_doc_line_comment(bytes, index))
}

fn match_block_comment_start(bytes: &[u8], index: usize) -> Option<bool> {
    if !starts_with(bytes, index, b"/*") {
        return None;
    }

    Some(is_doc_block_comment(bytes, index))
}

fn is_doc_line_comment(bytes: &[u8], index: usize) -> bool {
    (starts_with(bytes, index, b"///") && !starts_with(bytes, index, b"////"))
        || starts_with(bytes, index, b"//!")
}

fn is_doc_block_comment(bytes: &[u8], index: usize) -> bool {
    (starts_with(bytes, index, b"/**") && !starts_with(bytes, index, b"/***"))
        || starts_with(bytes, index, b"/*!")
}

fn match_raw_string_start(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index) != Some(&b'r') {
        return None;
    }

    let mut cursor = index + 1;
    let mut hashes = 0;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }

    if bytes.get(cursor) == Some(&b'"') {
        Some(hashes)
    } else {
        None
    }
}

fn is_raw_string_end(bytes: &[u8], index: usize, hashes: usize) -> bool {
    if bytes.get(index) != Some(&b'"') {
        return false;
    }

    for offset in 0..hashes {
        if bytes.get(index + 1 + offset) != Some(&b'#') {
            return false;
        }
    }

    true
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
        LineKind::Doc => "doc",
        LineKind::Mixed => "mixed",
    }
}

fn line_reason(raw_kind: LineKind, effective_kind: LineKind) -> &'static str {
    match (raw_kind, effective_kind) {
        (LineKind::Doc, LineKind::Code) => {
            "line matched a Rust documentation comment, but classification policy counts docs as code"
        }
        (LineKind::Mixed, LineKind::Comment) => {
            "line contains both Rust code and comment segments, but classification policy excludes mixed lines from code"
        }
        _ => line_reason_for_kind(effective_kind),
    }
}

fn line_reason_for_kind(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Blank => "line contains only whitespace",
        LineKind::Code => "line contains Rust code or string literal content without comments",
        LineKind::Comment => "line is part of a regular Rust comment",
        LineKind::Doc => "line is part of a Rust documentation comment",
        LineKind::Mixed => "line contains both Rust code and comment segments",
    }
}
