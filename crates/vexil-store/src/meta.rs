use std::sync::OnceLock;
use vexil_lang::diagnostic::Severity;
use vexil_lang::CompiledSchema;

static META_SCHEMA: OnceLock<CompiledSchema> = OnceLock::new();
static PACK_SCHEMA: OnceLock<CompiledSchema> = OnceLock::new();

/// The `vexil.schema` meta-schema, compiled from embedded source.
pub fn meta_schema() -> &'static CompiledSchema {
    META_SCHEMA.get_or_init(|| {
        let source = include_str!("../../../schemas/vexil/schema.vexil");
        let result = vexil_lang::compile_internal(source);
        // Meta-schema must always compile. Panic is intentional —
        // a broken meta-schema is a build-time bug, not a runtime error.
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

/// The `vexil.pack` meta-schema, compiled from embedded source.
pub fn pack_schema() -> &'static CompiledSchema {
    PACK_SCHEMA.get_or_init(|| {
        let source = include_str!("../../../schemas/vexil/pack.vexil");
        let result = vexil_lang::compile_internal(source);
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
