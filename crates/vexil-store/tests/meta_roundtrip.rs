use vexil_lang::diagnostic::Severity;

#[test]
fn meta_schema_compiles() {
    let source = include_str!("../../../schemas/vexil/schema.vexil");
    let result = vexil_lang::compile_internal(source);
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    assert!(
        !has_errors,
        "meta-schema compilation errors: {:?}",
        result.diagnostics
    );
    let compiled = result
        .compiled
        .expect("meta-schema should produce a CompiledSchema");
    // namespace is Vec<SmolStr> — compare via iterator
    let ns: Vec<&str> = compiled.namespace.iter().map(|s| s.as_str()).collect();
    assert_eq!(ns, vec!["vexil", "schema"]);

    // Verify key types exist
    assert!(compiled.registry.lookup("CompiledSchema").is_some());
    assert!(compiled.registry.lookup("TypeDef").is_some());
    assert!(compiled.registry.lookup("ResolvedType").is_some());
    assert!(compiled.registry.lookup("SchemaStore").is_some());
}

#[test]
fn pack_schema_compiles() {
    let source = include_str!("../../../schemas/vexil/pack.vexil");
    let result = vexil_lang::compile_internal(source);
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    assert!(
        !has_errors,
        "pack schema compilation errors: {:?}",
        result.diagnostics
    );
    let compiled = result
        .compiled
        .expect("pack schema should produce a CompiledSchema");
    let ns: Vec<&str> = compiled.namespace.iter().map(|s| s.as_str()).collect();
    assert_eq!(ns, vec!["vexil", "pack"]);
    assert!(compiled.registry.lookup("DataPack").is_some());
    assert!(compiled.registry.lookup("DataEntry").is_some());
}

#[test]
fn meta_schema_loads_via_api() {
    let schema = vexil_store::meta_schema();
    // namespace is Vec<SmolStr>
    let ns: Vec<&str> = schema.namespace.iter().map(|s| s.as_str()).collect();
    assert_eq!(ns, vec!["vexil", "schema"]);
    // Should return the same reference on repeated calls
    let schema2 = vexil_store::meta_schema();
    assert!(std::ptr::eq(schema, schema2));
}

#[test]
fn pack_schema_loads_via_api() {
    let schema = vexil_store::pack_schema();
    let ns: Vec<&str> = schema.namespace.iter().map(|s| s.as_str()).collect();
    assert_eq!(ns, vec!["vexil", "pack"]);
}
