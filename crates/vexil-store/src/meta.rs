/// Re-exports the compiled meta-schemas from `vexil-lang`.
///
/// The meta-schemas live in `vexil-lang` because they are part of the language
/// implementation; `vexil-store` uses them to encode/decode compiled schemas
/// as Vexil values, but does not own the schema definitions.
pub use vexil_lang::meta_schema;
pub use vexil_lang::pack_schema;
