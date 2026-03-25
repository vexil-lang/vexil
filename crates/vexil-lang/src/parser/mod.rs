pub mod decl;
pub mod expr;
pub mod import;

use crate::ast::Schema;
use crate::diagnostic::Diagnostic;
use crate::lexer::token::Token;

/// Parse a token stream into a Schema AST.
/// Stub implementation — returns None with no diagnostics.
pub fn parse(_source: &str, _tokens: Vec<Token>) -> (Option<Schema>, Vec<Diagnostic>) {
    (None, Vec::new())
}
