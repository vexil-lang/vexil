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
pub use ir::{CompiledSchema, ResolvedType, TypeDef, TypeId, TypeRegistry};
pub use project::compile_project;
pub use project::ProjectResult;
pub use resolve::SchemaLoader;

use ast::Schema;
use diagnostic::{Diagnostic, Severity};

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
    compile_impl(source, false)
}

/// Full pipeline for internal/meta schemas that may use the reserved `vexil`
/// namespace prefix. Skips the namespace-reservation check only; all other
/// validation, lowering, and type-checking still applies.
pub fn compile_internal(source: &str) -> CompileResult {
    compile_impl(source, true)
}

fn compile_impl(source: &str, allow_reserved: bool) -> CompileResult {
    let (tokens, mut diagnostics) = lexer::lex(source);
    let (schema, parse_diags) = parser::parse(source, tokens);
    diagnostics.extend(parse_diags);
    if let Some(ref schema) = schema {
        let validate_diags = if allow_reserved {
            validate::validate_allow_reserved(schema)
        } else {
            validate::validate(schema)
        };
        diagnostics.extend(validate_diags);
    }
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return CompileResult {
            schema,
            compiled: None,
            diagnostics,
        };
    }
    let Some(schema) = schema else {
        return CompileResult {
            schema: None,
            compiled: None,
            diagnostics,
        };
    };
    let (mut compiled, lower_diags) = lower::lower(&schema);
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
