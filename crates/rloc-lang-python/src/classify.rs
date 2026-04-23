use std::{collections::BTreeSet, fs};

use rloc_core::{
    AnalysisWarning, BackendFileAnalysis, ClassificationOptions, FileCategory, FileMetrics,
    LineBreakdown, LineExplanation, Utf8Path,
};
use tree_sitter::Node;

use crate::parser;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScannerState {
    Normal,
    String {
        quote: u8,
        triple: bool,
        raw: bool,
        escaped: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StringStart {
    consumed: usize,
    quote: u8,
    triple: bool,
    raw: bool,
}

pub fn classify_file(
    path: &Utf8Path,
    category: FileCategory,
    options: &ClassificationOptions,
) -> Result<BackendFileAnalysis, String> {
    let bytes = fs::read(path.as_std_path()).map_err(|error| error.to_string())?;
    let contents = String::from_utf8_lossy(&bytes);
    let mut warnings = Vec::new();
    let docstring_lines = match collect_docstring_lines(&contents) {
        Ok(mut lines) => {
            lines.extend(collect_structural_docstring_lines(&contents));
            lines
        }
        Err(error) => {
            warnings.push(AnalysisWarning::for_file(
                path.to_path_buf(),
                rloc_core::Language::Python,
                format!(
                    "docstring detection fell back to line scanning because {}",
                    error
                ),
            ));
            collect_structural_docstring_lines(&contents)
        }
    };

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
        let line_number = index + 1;
        let raw_kind = classify_line(line, &mut state);
        let kind = normalize_kind(raw_kind, line_number, &docstring_lines, options);

        match kind {
            LineKind::Blank => blank_lines += 1,
            LineKind::Code => code_lines += 1,
            LineKind::Comment => comment_lines += 1,
            LineKind::Doc => doc_lines += 1,
            LineKind::Mixed => mixed_lines += 1,
        }

        if !matches!(kind, LineKind::Blank) {
            line_explanations.push(LineExplanation {
                line_number: line_number as u32,
                kind: line_kind_name(kind).to_owned(),
                snippet: line.trim().to_owned(),
                reason: line_reason(raw_kind, kind).to_owned(),
            });
        }
    }

    Ok(BackendFileAnalysis {
        metrics: FileMetrics::from_line_breakdown(
            path.to_path_buf(),
            rloc_core::Language::Python,
            category,
            bytes.len() as u64,
            LineBreakdown {
                total: total_lines,
                blank: blank_lines,
                code: code_lines,
                comment: comment_lines,
                doc: doc_lines,
                mixed: mixed_lines,
                ..LineBreakdown::default()
            },
        ),
        line_explanations,
        warnings,
    })
}

pub fn classifier_status() -> &'static str {
    "tree-sitter-backed python classifier"
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

                if bytes[index] == b'#' {
                    flags.has_comment = true;
                    break;
                }

                if let Some(start) = match_string_start(bytes, index) {
                    flags.has_code = true;
                    *state = ScannerState::String {
                        quote: start.quote,
                        triple: start.triple,
                        raw: start.raw,
                        escaped: false,
                    };
                    index += start.consumed;
                    continue;
                }

                flags.has_code = true;
                index += 1;
            }
            ScannerState::String {
                quote,
                triple,
                raw,
                ref mut escaped,
            } => {
                flags.has_code = true;

                if triple && is_triple_string_end(bytes, index, quote) {
                    *state = ScannerState::Normal;
                    index += 3;
                    continue;
                }

                if !triple && bytes[index] == quote && !*escaped {
                    *state = ScannerState::Normal;
                    index += 1;
                    continue;
                }

                if !raw && bytes[index] == b'\\' && !*escaped {
                    *escaped = true;
                    index += 1;
                    continue;
                }

                *escaped = false;
                index += 1;
            }
        }
    }

    if matches!(*state, ScannerState::String { triple: false, .. }) {
        *state = ScannerState::Normal;
    }

    if !flags.has_code && !flags.has_comment {
        return LineKind::Blank;
    }
    if flags.has_code && flags.has_comment {
        return LineKind::Mixed;
    }
    if flags.has_comment {
        return LineKind::Comment;
    }
    LineKind::Code
}

fn carried_flags(state: ScannerState) -> LineFlags {
    match state {
        ScannerState::Normal => LineFlags::default(),
        ScannerState::String { .. } => LineFlags {
            has_code: true,
            has_comment: false,
        },
    }
}

fn normalize_kind(
    kind: LineKind,
    line_number: usize,
    docstring_lines: &BTreeSet<usize>,
    options: &ClassificationOptions,
) -> LineKind {
    let kind = if docstring_lines.contains(&line_number) {
        if options.count_docstrings_as_comments {
            match kind {
                LineKind::Code => LineKind::Doc,
                other => other,
            }
        } else {
            match kind {
                LineKind::Doc => LineKind::Code,
                other => other,
            }
        }
    } else {
        kind
    };

    if matches!(kind, LineKind::Mixed) && !options.mixed_lines_as_code {
        LineKind::Comment
    } else {
        kind
    }
}

fn collect_docstring_lines(source: &str) -> Result<BTreeSet<usize>, String> {
    let Some(tree) = parser::parse(source)? else {
        return Err(format!(
            "{} could not produce a parse tree",
            parser::parser_name()
        ));
    };

    let mut lines = BTreeSet::new();
    collect_docstring_lines_from_node(tree.root_node(), &mut lines);
    Ok(lines)
}

fn collect_docstring_lines_from_node(node: Node<'_>, lines: &mut BTreeSet<usize>) {
    match node.kind() {
        "module" => {
            if let Some(statement) = first_named_child(node) {
                mark_docstring_statement(statement, lines);
            }
        }
        "class_definition" | "function_definition" => {
            if let Some(body) = node.child_by_field_name("body") {
                if let Some(statement) = first_named_child(body) {
                    mark_docstring_statement(statement, lines);
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_docstring_lines_from_node(child, lines);
    }
}

fn first_named_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).next()
}

fn mark_docstring_statement(statement: Node<'_>, lines: &mut BTreeSet<usize>) {
    if statement.kind() != "expression_statement" || statement.named_child_count() != 1 {
        return;
    }

    let Some(value) = statement.named_child(0) else {
        return;
    };
    if value.kind() != "string" {
        return;
    }

    let start_line = statement.start_position().row + 1;
    let end_line = statement.end_position().row + 1;
    for line in start_line..=end_line {
        lines.insert(line);
    }
}

fn collect_structural_docstring_lines(source: &str) -> BTreeSet<usize> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut docstring_lines = BTreeSet::new();
    let mut pending_bodies = Vec::new();
    let mut module_pending = true;
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        let indent = indentation(line);

        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
            continue;
        }

        while matches!(pending_bodies.last(), Some(parent_indent) if indent <= *parent_indent) {
            pending_bodies.pop();
        }

        if module_pending && indent == 0 {
            if let Some(end_index) = docstring_statement_end(&lines, index) {
                mark_line_range(&mut docstring_lines, index, end_index);
                module_pending = false;
                index = end_index + 1;
                continue;
            }
            module_pending = false;
        }

        if let Some(parent_indent) = pending_bodies.last().copied() {
            if indent > parent_indent {
                if let Some(end_index) = docstring_statement_end(&lines, index) {
                    mark_line_range(&mut docstring_lines, index, end_index);
                    pending_bodies.pop();
                    index = end_index + 1;
                    continue;
                }
                pending_bodies.pop();
            }
        }

        if is_class_or_function_header(trimmed) {
            pending_bodies.push(indent);
        }

        index += 1;
    }

    docstring_lines
}

fn mark_line_range(lines: &mut BTreeSet<usize>, start_index: usize, end_index: usize) {
    for line in (start_index + 1)..=(end_index + 1) {
        lines.insert(line);
    }
}

fn docstring_statement_end(lines: &[&str], start_index: usize) -> Option<usize> {
    let trimmed = lines[start_index].trim_start();
    let start = match_string_start(trimmed.as_bytes(), 0)?;
    if !start.triple {
        return None;
    }

    let mut line_index = start_index;
    let mut byte_index = start.consumed;
    let quote = start.quote;
    let raw = start.raw;
    let mut escaped = false;

    loop {
        let current = if line_index == start_index {
            trimmed
        } else {
            lines[line_index]
        };
        let bytes = current.as_bytes();

        while byte_index < bytes.len() {
            if is_triple_string_end(bytes, byte_index, quote) {
                let rest = &bytes[byte_index + 3..];
                return rest
                    .iter()
                    .all(|byte| byte.is_ascii_whitespace())
                    .then_some(line_index);
            }

            if !raw && bytes[byte_index] == b'\\' && !escaped {
                escaped = true;
                byte_index += 1;
                continue;
            }

            escaped = false;
            byte_index += 1;
        }

        line_index += 1;
        if line_index >= lines.len() {
            return None;
        }
        byte_index = 0;
        escaped = false;
    }
}

fn indentation(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

fn is_class_or_function_header(trimmed: &str) -> bool {
    (trimmed.starts_with("class ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("async def "))
        && trimmed.ends_with(':')
}

fn match_string_start(bytes: &[u8], index: usize) -> Option<StringStart> {
    if index > 0 && is_identifier_continue(bytes[index - 1]) {
        return None;
    }

    let mut cursor = index;
    while matches!(
        bytes.get(cursor),
        Some(b'r' | b'R' | b'u' | b'U' | b'b' | b'B' | b'f' | b'F')
    ) {
        cursor += 1;
    }

    let quote = *bytes.get(cursor)?;
    if quote != b'\'' && quote != b'"' {
        if cursor == index {
            return None;
        }
        return None;
    }

    let triple = bytes.get(cursor + 1) == Some(&quote) && bytes.get(cursor + 2) == Some(&quote);
    let raw = bytes[index..cursor]
        .iter()
        .any(|byte| matches!(byte, b'r' | b'R'));
    let consumed = if triple {
        cursor - index + 3
    } else {
        cursor - index + 1
    };

    Some(StringStart {
        consumed,
        quote,
        triple,
        raw,
    })
}

fn is_triple_string_end(bytes: &[u8], index: usize, quote: u8) -> bool {
    bytes.get(index) == Some(&quote)
        && bytes.get(index + 1) == Some(&quote)
        && bytes.get(index + 2) == Some(&quote)
}

fn is_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
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
        (LineKind::Mixed, LineKind::Comment) => {
            "line contains Python code and an inline comment, but classification policy excludes mixed lines from code"
        }
        _ => line_reason_for_kind(effective_kind),
    }
}

fn line_reason_for_kind(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Blank => "line contains only whitespace",
        LineKind::Code => "line contains Python code or string literal content without comments",
        LineKind::Comment => "line is a regular Python comment",
        LineKind::Doc => "line is part of a Python docstring",
        LineKind::Mixed => "line contains Python code and an inline comment",
    }
}
