//! # vexil-lang
//!
//! Compiler library for the [Vexil](https://github.com/vexil-lang/vexil) schema
//! definition language. Provides lexing, parsing, type checking, and compilation
//! of `.vexil` schema files.
//!
//! ## Quick Start
//!
//! ```rust
//! use vexil_lang::compile;
//!
//! let source = r#"
//!     namespace example
//!     message Point { x @0 : f32  y @1 : f32 }
//! "#;
//!
//! let result = compile(source);
//! assert!(result.diagnostics.iter().all(|d| d.severity != vexil_lang::Severity::Error));
//! ```
//!
//! ## API Tiers
//!
//! - **Tier 1 (stable):** [`compile`], [`compile_project`], [`CompiledSchema`],
//!   [`ProjectResult`], [`CodegenBackend`]
//! - **Tier 2 (semi-stable):** AST types, pipeline stages
//! - **Tier 3 (internal):** Lexer, parser -- subject to change

/// Abstract syntax tree types produced by the parser.
pub mod ast;
/// Canonical form serialisation (spec section 7).
pub mod canonical;
/// Code generation backend trait and error types.
pub mod codegen;
/// Schema compatibility checker (spec section 10).
pub mod compat;
/// Diagnostic types for errors and warnings.
pub mod diagnostic;
/// Intermediate representation: compiled schema, type registry, and type definitions.
pub mod ir;
/// Lexer: tokenises Vexil source text.
pub mod lexer;
/// Lowering: transforms AST into IR.
pub mod lower;
/// Meta-schemas for Vexil's own IR types.
pub mod meta;
/// Parser: builds an AST from a token stream.
pub mod parser;
/// Multi-file project compilation and import graph resolution.
pub mod project;
/// Type remapping utilities for cross-registry type cloning.
pub mod remap;
/// Schema loading abstraction (filesystem, in-memory).
pub mod resolve;
/// Source span types for error reporting.
pub mod span;
/// Type checker: validates IR and computes wire sizes.
pub mod typeck;
/// Semantic validation of parsed schemas.
pub mod validate;

pub use codegen::{CodegenBackend, CodegenError};
pub use ir::{CompiledSchema, ResolvedType, TypeDef, TypeId, TypeRegistry};
pub use meta::{meta_schema, pack_schema};
pub use project::compile_project;
pub use project::ProjectResult;
pub use resolve::SchemaLoader;

pub use diagnostic::{Diagnostic, Severity};

use ast::Schema;

/// The result of parsing a Vexil source string.
///
/// Contains the parsed AST (if parsing succeeded) and any diagnostics
/// produced during lexing, parsing, and validation.
pub struct ParseResult {
    /// The parsed schema AST, or `None` if a fatal parse error occurred.
    pub schema: Option<Schema>,
    /// Errors and warnings from lexing, parsing, and structural validation.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse a Vexil schema source string.
///
/// Runs the lexer, parser, and structural validator. Does **not** lower to IR
/// or type-check. Use [`compile`] for the full pipeline.
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

/// The result of the full compilation pipeline.
///
/// Contains both the source-faithful AST and the lowered/type-checked IR,
/// along with all diagnostics produced at every stage.
pub struct CompileResult {
    /// The parsed schema AST, or `None` if a fatal parse error occurred.
    pub schema: Option<Schema>,
    /// The compiled IR, or `None` if errors prevented lowering or type-checking.
    pub compiled: Option<CompiledSchema>,
    /// All diagnostics (errors and warnings) from every pipeline stage.
    pub diagnostics: Vec<Diagnostic>,
}

/// Compile a Vexil source string through the full pipeline.
///
/// Runs lexing, parsing, structural validation, lowering to IR, and
/// type-checking in sequence. Returns a [`CompileResult`] containing both
/// the AST and compiled IR (when compilation succeeds) plus all diagnostics.
///
/// For multi-file projects with imports, use [`compile_project`] instead.
pub fn compile(source: &str) -> CompileResult {
    compile_impl(source, false)
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
