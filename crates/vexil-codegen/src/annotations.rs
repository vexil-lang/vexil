use crate::emit::CodeWriter;
use vexil_lang::ir::{ResolvedAnnotations, TombstoneDef};

/// Emit `@doc` as `///` doc comments.
pub fn emit_doc(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    for doc in &annotations.doc {
        w.line(&format!("/// {doc}"));
    }
}

/// Emit `@since` as a doc comment.
pub fn emit_since(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    if let Some(ref since) = annotations.since {
        w.line(&format!("/// @since {since}"));
    }
}

/// Emit `@deprecated` as `#[deprecated(...)]`.
pub fn emit_deprecated(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    if let Some(ref dep) = annotations.deprecated {
        match &dep.since {
            Some(since) => w.line(&format!(
                "#[deprecated(since = \"{since}\", note = \"{}\")]",
                dep.reason
            )),
            None => w.line(&format!("#[deprecated(note = \"{}\")]", dep.reason)),
        }
    }
}

/// Emit `@non_exhaustive` as `#[non_exhaustive]`.
pub fn emit_non_exhaustive(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    if annotations.non_exhaustive {
        w.line("#[non_exhaustive]");
    }
}

/// Emit all type-level annotations in standard order.
pub fn emit_type_annotations(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    emit_doc(w, annotations);
    emit_since(w, annotations);
    emit_deprecated(w, annotations);
    emit_non_exhaustive(w, annotations);
}

/// Emit field-level annotations.
pub fn emit_field_annotations(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    emit_doc(w, annotations);
    emit_since(w, annotations);
    emit_deprecated(w, annotations);
}

/// Emit protocol-level annotations (@revision).
pub fn emit_protocol_annotations(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    if let Some(rev) = annotations.revision {
        w.line(&format!("/// @revision({rev})"));
    }
}

pub fn emit_tombstones(w: &mut CodeWriter, type_name: &str, tombstones: &[TombstoneDef]) {
    if tombstones.is_empty() {
        return;
    }
    for t in tombstones {
        let since_str = t.since.as_deref().unwrap_or("unknown");
        w.line(&format!(
            "// REMOVED @{} (since {}): {}",
            t.ordinal, since_str, t.reason
        ));
    }
    w.write(&format!(
        "pub const {}_REMOVED_ORDINALS: &[(u16, &str, &str)] = &[",
        type_name.to_uppercase()
    ));
    w.append("\n");
    w.indent();
    for t in tombstones {
        let since_str = t.since.as_deref().unwrap_or("unknown");
        w.line(&format!(
            "({}, \"{}\", \"{}\"),",
            t.ordinal, since_str, t.reason
        ));
    }
    w.dedent();
    w.line("];");
}
