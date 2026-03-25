pub mod annotations;
pub mod boxing;
pub mod config;
pub mod delta;
pub mod emit;
pub mod enum_gen;
pub mod flags;
pub mod message;
pub mod newtype;
pub mod types;
pub mod union_gen;

use vexil_lang::ir::{CompiledSchema, TypeId};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CodegenError {
    #[error("unresolved type {type_id:?} referenced by {referenced_by}")]
    UnresolvedType {
        type_id: TypeId,
        referenced_by: String,
    },
}

pub fn generate(compiled: &CompiledSchema) -> Result<String, CodegenError> {
    todo!("implemented in Task 17")
}
