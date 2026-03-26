/// Compiled meta-schemas for the `vexil.*` namespace.
///
/// The meta-schema describes Vexil's own IR types and is used by `vexil-store`
/// to encode/decode compiled schemas as first-class Vexil values.
/// Compilation is done once at first use and cached for the process lifetime.
use std::sync::OnceLock;

use crate::diagnostic::Severity;
use crate::ir::CompiledSchema;

static META_SCHEMA: OnceLock<CompiledSchema> = OnceLock::new();
static PACK_SCHEMA: OnceLock<CompiledSchema> = OnceLock::new();

/// The compiled `vexil.schema` meta-schema.
///
/// Describes Vexil's own IR types (`MessageDef`, `EnumDef`, `CompiledSchema`, …).
/// Panics at first call if the embedded source fails to compile — that would
/// be a build-time bug in the language implementation, not a runtime error.
pub fn meta_schema() -> &'static CompiledSchema {
    META_SCHEMA.get_or_init(|| {
        let source = include_str!("../../../schemas/vexil/schema.vexil");
        let result = super::compile_impl(source, true);
        let has_errors = result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error);
        assert!(
            !has_errors,
            "BUG: embedded vexil.schema failed to compile: {:?}",
            result.diagnostics
        );
        result
            .compiled
            .expect("BUG: vexil.schema produced no compiled output")
    })
}

/// The compiled `vexil.pack` meta-schema.
///
/// Describes the `DataEntry`/`DataPack` types used for multi-value binary packs.
/// Panics at first call if the embedded source fails to compile.
pub fn pack_schema() -> &'static CompiledSchema {
    PACK_SCHEMA.get_or_init(|| {
        let source = include_str!("../../../schemas/vexil/pack.vexil");
        let result = super::compile_impl(source, true);
        let has_errors = result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error);
        assert!(
            !has_errors,
            "BUG: embedded vexil.pack failed to compile: {:?}",
            result.diagnostics
        );
        result
            .compiled
            .expect("BUG: vexil.pack produced no compiled output")
    })
}
