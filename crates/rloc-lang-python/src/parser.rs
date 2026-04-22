use tree_sitter::{Parser, Tree};

pub const PARSER_NAME: &str = "tree-sitter-python";

pub fn parser_name() -> &'static str {
    PARSER_NAME
}

pub fn parse(source: &str) -> Result<Option<Tree>, String> {
    let mut parser = Parser::new();
    let language = tree_sitter_python::LANGUAGE;
    parser
        .set_language(&language.into())
        .map_err(|error| format!("failed to load {PARSER_NAME}: {error}"))?;

    Ok(parser.parse(source, None))
}
