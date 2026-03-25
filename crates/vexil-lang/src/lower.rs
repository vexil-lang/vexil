use crate::ast::Schema;
use crate::diagnostic::Diagnostic;
use crate::ir::CompiledSchema;

/// Lower an AST Schema to the compiled IR.
pub fn lower(_schema: &Schema) -> (Option<CompiledSchema>, Vec<Diagnostic>) {
    // TODO: implement in Task 4
    (None, Vec::new())
}
