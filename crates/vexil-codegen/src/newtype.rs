use std::collections::HashSet;

use vexil_lang::ir::{FieldEncoding, NewtypeDef, TypeId, TypeRegistry};

use crate::annotations::emit_type_annotations;
use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::rust_type;

/// Emit a newtype struct with `Pack` and `Unpack` implementations.
///
/// The struct wraps `inner_type` as its public field.  Encoding and decoding
/// delegate entirely to `terminal_type` (the fully unwrapped primitive or
/// named type at the base of the newtype chain) using a default `FieldEncoding`.
pub fn emit_newtype(
    w: &mut CodeWriter,
    nt: &NewtypeDef,
    registry: &TypeRegistry,
    needs_box: &HashSet<(TypeId, usize)>,
) {
    let name = nt.name.as_str();

    // ── Type-level annotations (doc, since, deprecated, non_exhaustive) ─────
    emit_type_annotations(w, &nt.annotations);

    // ── Struct definition ────────────────────────────────────────────────────
    w.line("#[derive(Debug, Clone, PartialEq)]");
    let inner_rust = rust_type(&nt.inner_type, registry, needs_box, None);
    w.line(&format!("pub struct {name}(pub {inner_rust});"));
    w.blank();

    // ── Pack impl ────────────────────────────────────────────────────────────
    w.open_block(&format!("impl vexil_runtime::Pack for {name}"));
    w.open_block(
        "fn pack(&self, w: &mut vexil_runtime::BitWriter) -> Result<(), vexil_runtime::EncodeError>",
    );
    let enc = FieldEncoding::default_encoding();
    emit_write(w, "self.0", &nt.terminal_type, &enc, registry, name);
    w.line("w.flush_to_byte_boundary();");
    w.line("Ok(())");
    w.close_block();
    w.close_block();
    w.blank();

    // ── Unpack impl ──────────────────────────────────────────────────────────
    w.open_block(&format!("impl vexil_runtime::Unpack for {name}"));
    w.open_block(
        "fn unpack(r: &mut vexil_runtime::BitReader<'_>) -> Result<Self, vexil_runtime::DecodeError>",
    );
    emit_read(w, "value", &nt.terminal_type, &enc, registry, name);
    w.line("r.flush_to_byte_boundary();");
    w.line("Ok(Self(value))");
    w.close_block();
    w.close_block();
    w.blank();
}
