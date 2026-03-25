pub mod token;

use token::Token;
use crate::diagnostic::Diagnostic;

/// Tokenise source into a flat token list plus any lexer diagnostics.
/// Stub implementation — returns an empty list.
pub fn lex(_source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    (Vec::new(), Vec::new())
}
