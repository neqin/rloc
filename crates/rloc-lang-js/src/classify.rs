use std::fs;

use rloc_core::{
    AnalysisWarning, BackendFileAnalysis, ClassificationOptions, FileCategory, FileMetrics,
    Language, LineBreakdown, LineExplanation, Utf8Path,
};

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
    has_doc: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodeContext {
    Normal,
    TemplateExpr { brace_depth: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Code(CodeContext),
    String { quote: u8, escaped: bool },
    Template { escaped: bool },
    Regex { escaped: bool, in_char_class: bool },
    BlockComment { doc: bool },
}

pub fn classify_file(
    path: &Utf8Path,
    language: Language,
    category: FileCategory,
    options: &ClassificationOptions,
) -> Result<BackendFileAnalysis, String> {
    let bytes = fs::read(path.as_std_path()).map_err(|error| error.to_string())?;
    let contents = String::from_utf8_lossy(&bytes);
    let mut warnings = Vec::new();

    match parser::parse(language, &contents) {
        Ok(Some(_tree)) => {}
        Ok(None) => warnings.push(AnalysisWarning::for_file(
            path.to_path_buf(),
            language,
            format!(
                "{} could not produce a parse tree; using scanner-only classification",
                parser::parser_name(language)
            ),
        )),
        Err(error) => warnings.push(AnalysisWarning::for_file(
            path.to_path_buf(),
            language,
            format!(
                "failed to initialize {}: {error}; using scanner-only classification",
                parser::parser_name(language)
            ),
        )),
    }

    let mut total_lines = 0_u32;
    let mut blank_lines = 0_u32;
    let mut code_lines = 0_u32;
    let mut comment_lines = 0_u32;
    let mut doc_lines = 0_u32;
    let mut mixed_lines = 0_u32;
    let mut line_explanations = Vec::new();
    let mut stack = vec![Mode::Code(CodeContext::Normal)];

    for (index, line) in contents.lines().enumerate() {
        total_lines += 1;
        let raw_kind = classify_line(line, &mut stack);
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
            language,
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
    "deterministic js-family backend classifier"
}

fn normalize_kind(kind: LineKind, options: &ClassificationOptions) -> LineKind {
    match kind {
        LineKind::Doc if !options.count_doc_comments => LineKind::Code,
        LineKind::Mixed if !options.mixed_lines_as_code => LineKind::Comment,
        other => other,
    }
}

fn classify_line(line: &str, stack: &mut Vec<Mode>) -> LineKind {
    let mut flags = carried_flags(stack);
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let top_index = stack.len() - 1;
        match stack[top_index] {
            Mode::Code(CodeContext::Normal) => {
                if bytes[index].is_ascii_whitespace() {
                    index += 1;
                    continue;
                }

                if starts_with(bytes, index, b"//") {
                    flags.has_comment = true;
                    break;
                }

                if let Some(doc) = match_block_comment_start(bytes, index) {
                    mark_comment(&mut flags, doc);
                    stack.push(Mode::BlockComment { doc });
                    index += 2;
                    continue;
                }

                if should_start_regex(bytes, index) {
                    flags.has_code = true;
                    stack.push(Mode::Regex {
                        escaped: false,
                        in_char_class: false,
                    });
                    index += 1;
                    continue;
                }

                if bytes[index] == b'\'' || bytes[index] == b'"' {
                    flags.has_code = true;
                    stack.push(Mode::String {
                        quote: bytes[index],
                        escaped: false,
                    });
                    index += 1;
                    continue;
                }

                if bytes[index] == b'`' {
                    flags.has_code = true;
                    stack.push(Mode::Template { escaped: false });
                    index += 1;
                    continue;
                }

                flags.has_code = true;
                index += 1;
            }
            Mode::Code(CodeContext::TemplateExpr { brace_depth }) => {
                if bytes[index].is_ascii_whitespace() {
                    index += 1;
                    continue;
                }

                if starts_with(bytes, index, b"//") {
                    flags.has_comment = true;
                    break;
                }

                if let Some(doc) = match_block_comment_start(bytes, index) {
                    mark_comment(&mut flags, doc);
                    stack.push(Mode::BlockComment { doc });
                    index += 2;
                    continue;
                }

                if should_start_regex(bytes, index) {
                    flags.has_code = true;
                    stack.push(Mode::Regex {
                        escaped: false,
                        in_char_class: false,
                    });
                    index += 1;
                    continue;
                }

                if bytes[index] == b'\'' || bytes[index] == b'"' {
                    flags.has_code = true;
                    stack.push(Mode::String {
                        quote: bytes[index],
                        escaped: false,
                    });
                    index += 1;
                    continue;
                }

                if bytes[index] == b'`' {
                    flags.has_code = true;
                    stack.push(Mode::Template { escaped: false });
                    index += 1;
                    continue;
                }

                if bytes[index] == b'{' {
                    flags.has_code = true;
                    stack[top_index] = Mode::Code(CodeContext::TemplateExpr {
                        brace_depth: brace_depth + 1,
                    });
                    index += 1;
                    continue;
                }

                if bytes[index] == b'}' {
                    if brace_depth == 0 {
                        stack.pop();
                        index += 1;
                    } else {
                        flags.has_code = true;
                        stack[top_index] = Mode::Code(CodeContext::TemplateExpr {
                            brace_depth: brace_depth - 1,
                        });
                        index += 1;
                    }
                    continue;
                }

                flags.has_code = true;
                index += 1;
            }
            Mode::String { quote, escaped } => {
                flags.has_code = true;
                match bytes[index] {
                    b'\\' if !escaped => {
                        stack[top_index] = Mode::String {
                            quote,
                            escaped: true,
                        };
                        index += 1;
                    }
                    byte if byte == quote && !escaped => {
                        stack.pop();
                        index += 1;
                    }
                    _ => {
                        stack[top_index] = Mode::String {
                            quote,
                            escaped: false,
                        };
                        index += 1;
                    }
                }
            }
            Mode::Template { escaped } => {
                flags.has_code = true;
                if bytes[index] == b'`' && !escaped {
                    stack.pop();
                    index += 1;
                    continue;
                }

                if bytes[index] == b'$' && bytes.get(index + 1) == Some(&b'{') && !escaped {
                    stack.push(Mode::Code(CodeContext::TemplateExpr { brace_depth: 0 }));
                    index += 2;
                    continue;
                }

                if bytes[index] == b'\\' && !escaped {
                    stack[top_index] = Mode::Template { escaped: true };
                    index += 1;
                } else {
                    stack[top_index] = Mode::Template { escaped: false };
                    index += 1;
                }
            }
            Mode::Regex {
                escaped,
                in_char_class,
            } => {
                flags.has_code = true;
                match bytes[index] {
                    b'\\' if !escaped => {
                        stack[top_index] = Mode::Regex {
                            escaped: true,
                            in_char_class,
                        };
                        index += 1;
                    }
                    b'[' if !escaped => {
                        stack[top_index] = Mode::Regex {
                            escaped: false,
                            in_char_class: true,
                        };
                        index += 1;
                    }
                    b']' if !escaped && in_char_class => {
                        stack[top_index] = Mode::Regex {
                            escaped: false,
                            in_char_class: false,
                        };
                        index += 1;
                    }
                    b'/' if !escaped && !in_char_class => {
                        stack.pop();
                        index += 1;
                    }
                    _ => {
                        stack[top_index] = Mode::Regex {
                            escaped: false,
                            in_char_class,
                        };
                        index += 1;
                    }
                }
            }
            Mode::BlockComment { doc } => {
                mark_comment(&mut flags, doc);
                if starts_with(bytes, index, b"*/") {
                    stack.pop();
                    index += 2;
                } else {
                    index += 1;
                }
            }
        }
    }

    if !flags.has_code && !flags.has_comment && !flags.has_doc {
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
    LineKind::Code
}

fn carried_flags(stack: &[Mode]) -> LineFlags {
    match stack.last().copied() {
        Some(Mode::String { .. }) | Some(Mode::Template { .. }) | Some(Mode::Regex { .. }) => {
            LineFlags {
                has_code: true,
                has_comment: false,
                has_doc: false,
            }
        }
        Some(Mode::BlockComment { doc: true }) => LineFlags {
            has_code: false,
            has_comment: false,
            has_doc: true,
        },
        Some(Mode::BlockComment { doc: false }) => LineFlags {
            has_code: false,
            has_comment: true,
            has_doc: false,
        },
        _ => LineFlags::default(),
    }
}

fn match_block_comment_start(bytes: &[u8], index: usize) -> Option<bool> {
    if !starts_with(bytes, index, b"/*") {
        return None;
    }

    Some(is_doc_block_comment(bytes, index))
}

fn is_doc_block_comment(bytes: &[u8], index: usize) -> bool {
    starts_with(bytes, index, b"/**") && !starts_with(bytes, index, b"/***")
}

fn mark_comment(flags: &mut LineFlags, doc: bool) {
    if doc {
        flags.has_doc = true;
    } else {
        flags.has_comment = true;
    }
}

fn starts_with(bytes: &[u8], index: usize, needle: &[u8]) -> bool {
    bytes
        .get(index..index + needle.len())
        .is_some_and(|window| window == needle)
}

fn should_start_regex(bytes: &[u8], index: usize) -> bool {
    if bytes[index] != b'/' || starts_with(bytes, index, b"//") || starts_with(bytes, index, b"/*")
    {
        return false;
    }

    match previous_significant_byte(bytes, index) {
        None => true,
        Some(byte) => is_regex_prefix(byte),
    }
}

fn previous_significant_byte(bytes: &[u8], index: usize) -> Option<u8> {
    bytes[..index]
        .iter()
        .rev()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
}

fn is_regex_prefix(byte: u8) -> bool {
    matches!(
        byte,
        b'(' | b'['
            | b'{'
            | b'='
            | b':'
            | b','
            | b'!'
            | b'?'
            | b'+'
            | b'-'
            | b'*'
            | b'%'
            | b'&'
            | b'|'
            | b'^'
            | b'~'
            | b'<'
            | b'>'
    )
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
            "line matched a JSDoc-style block comment, but classification policy counts docs as code"
        }
        (LineKind::Mixed, LineKind::Comment) => {
            "line contains JS-family code and comment segments, but classification policy excludes mixed lines from code"
        }
        _ => line_reason_for_kind(effective_kind),
    }
}

fn line_reason_for_kind(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Blank => "line contains only whitespace",
        LineKind::Code => {
            "line contains JS-family code, string content, or template content without comments"
        }
        LineKind::Comment => "line is part of a regular JS-family comment",
        LineKind::Doc => "line is part of a JSDoc-style block comment",
        LineKind::Mixed => "line contains JS-family code and comment segments",
    }
}
