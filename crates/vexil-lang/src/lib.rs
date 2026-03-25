pub mod span;
pub mod diagnostic;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod validate;

use diagnostic::Diagnostic;
use ast::Schema;

pub struct ParseResult {
    pub schema: Option<Schema>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse a Vexil schema source string.
pub fn parse(source: &str) -> ParseResult {
    let (tokens, mut diagnostics) = lexer::lex(source);
    let (schema, parse_diags) = parser::parse(source, tokens);
    diagnostics.extend(parse_diags);
    if let Some(ref schema) = schema {
        let validate_diags = validate::validate(schema);
        diagnostics.extend(validate_diags);
    }
    ParseResult { schema, diagnostics }
}
