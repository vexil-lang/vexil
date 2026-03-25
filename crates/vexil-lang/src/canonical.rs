use crate::ir::CompiledSchema;

/// Compute the canonical form of a single-file schema per spec §7.
/// Returns a deterministic UTF-8 string — single-space-delimited, no newlines.
pub fn canonical_form(compiled: &CompiledSchema) -> String {
    let mut out = String::new();
    // namespace
    out.push_str("namespace ");
    out.push_str(&compiled.namespace.join("."));
    // TODO: schema-level annotations, declarations
    out
}

/// Compute the BLAKE3 hash of the canonical form.
pub fn schema_hash(compiled: &CompiledSchema) -> [u8; 32] {
    let form = canonical_form(compiled);
    *blake3::hash(form.as_bytes()).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_namespace_only() {
        let result = crate::compile("namespace test.minimal\nmessage Empty {}");
        let compiled = result.compiled.unwrap();
        let form = canonical_form(&compiled);
        assert!(form.starts_with("namespace test.minimal"));
        // Hash is 32 bytes
        let hash = schema_hash(&compiled);
        assert_eq!(hash.len(), 32);
    }
}
