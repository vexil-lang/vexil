pub mod ast;
pub mod canonical;
pub mod codegen;
pub mod diagnostic;
pub mod ir;
pub mod lexer;
pub mod lower;
pub mod parser;
pub mod project;
pub mod remap;
pub mod resolve;
pub mod span;
pub mod typeck;
pub mod validate;

pub use codegen::{CodegenBackend, CodegenError};
pub use project::compile_project;
pub use project::ProjectResult;
pub use resolve::SchemaLoader;

use ast::Schema;
use diagnostic::{Diagnostic, Severity};
use ir::CompiledSchema;

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
    ParseResult {
        schema,
        diagnostics,
    }
}

pub struct CompileResult {
    pub schema: Option<Schema>,
    pub compiled: Option<CompiledSchema>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Full pipeline: parse -> validate -> lower -> type-check.
pub fn compile(source: &str) -> CompileResult {
    let parse_result = parse(source);
    if parse_result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return CompileResult {
            schema: parse_result.schema,
            compiled: None,
            diagnostics: parse_result.diagnostics,
        };
    }
    let Some(schema) = parse_result.schema else {
        return CompileResult {
            schema: None,
            compiled: None,
            diagnostics: parse_result.diagnostics,
        };
    };
    let (mut compiled, lower_diags) = lower::lower(&schema);
    let mut diagnostics = parse_result.diagnostics;
    diagnostics.extend(lower_diags);
    if let Some(ref mut compiled) = compiled {
        let check_diags = typeck::check(compiled);
        diagnostics.extend(check_diags);
    }
    CompileResult {
        schema: Some(schema),
        compiled,
        diagnostics,
    }
}
