use rloc_core::Language;
use tree_sitter::{Parser, Tree};

pub const JAVASCRIPT_PARSER: &str = "tree-sitter-javascript";
pub const TYPESCRIPT_PARSER: &str = "tree-sitter-typescript";
pub const TSX_PARSER: &str = "tree-sitter-tsx";

pub fn parser_name(language: Language) -> &'static str {
    match language {
        Language::JavaScript | Language::Jsx => JAVASCRIPT_PARSER,
        Language::TypeScript => TYPESCRIPT_PARSER,
        Language::Tsx => TSX_PARSER,
        Language::Rust
        | Language::Python
        | Language::Markdown
        | Language::Config
        | Language::Unknown => JAVASCRIPT_PARSER,
    }
}

pub fn parse(language: Language, source: &str) -> Result<Option<Tree>, String> {
    let mut parser = Parser::new();
    match language {
        Language::JavaScript | Language::Jsx => {
            let grammar = tree_sitter_javascript::LANGUAGE;
            parser
                .set_language(&grammar.into())
                .map_err(|error| format!("failed to load {JAVASCRIPT_PARSER}: {error}"))?;
        }
        Language::TypeScript => {
            let grammar = tree_sitter_typescript::LANGUAGE_TYPESCRIPT;
            parser
                .set_language(&grammar.into())
                .map_err(|error| format!("failed to load {TYPESCRIPT_PARSER}: {error}"))?;
        }
        Language::Tsx => {
            let grammar = tree_sitter_typescript::LANGUAGE_TSX;
            parser
                .set_language(&grammar.into())
                .map_err(|error| format!("failed to load {TSX_PARSER}: {error}"))?;
        }
        Language::Rust
        | Language::Python
        | Language::Markdown
        | Language::Config
        | Language::Unknown => {
            return Err(format!(
                "unsupported JS-family parser selection for language {language}"
            ));
        }
    }

    Ok(parser.parse(source, None))
}
