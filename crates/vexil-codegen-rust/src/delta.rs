use vexil_lang::ast::PrimitiveType;
use vexil_lang::ir::{Encoding, FieldEncoding, MessageDef, ResolvedType, TypeRegistry};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::rust_type;

use std::collections::HashSet;
use vexil_lang::ir::TypeId;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns true if a field uses delta encoding.
fn is_delta(enc: &FieldEncoding) -> bool {
    matches!(enc.encoding, Encoding::Delta(_))
}

/// Returns true if the type is a float primitive (f32 or f64).
fn is_float(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(PrimitiveType::F32) | ResolvedType::Primitive(PrimitiveType::F64)
    )
}

/// Returns the zero literal for delta prev-state initialisation.
fn zero_literal(ty: &ResolvedType) -> &'static str {
    match ty {
        ResolvedType::Primitive(PrimitiveType::F32) => "0.0_f32",
        ResolvedType::Primitive(PrimitiveType::F64) => "0.0_f64",
        _ => "0",
    }
}

/// Strip the outer `Delta` wrapper from a `FieldEncoding`, returning the inner
/// encoding.  If the encoding is not `Delta`, returns a clone unchanged.
fn strip_delta(enc: &FieldEncoding) -> FieldEncoding {
    match &enc.encoding {
        Encoding::Delta(inner) => FieldEncoding {
            encoding: *inner.clone(),
            limit: enc.limit,
        },
        _ => enc.clone(),
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Emit `{Name}Encoder` and `{Name}Decoder` structs for a message that
/// contains at least one `@delta` field.
///
/// If the message has no delta fields this function is a no-op.
pub fn emit_delta(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry) {
    // Collect delta fields up-front.
    let delta_fields: Vec<_> = msg
        .fields
        .iter()
        .filter(|f| is_delta(&f.encoding))
        .collect();

    if delta_fields.is_empty() {
        return;
    }

    let name = msg.name.as_str();
    // We never need Box wrapping for the generated encoder/decoder fields — they
    // only store scalar prev-state values.
    let empty_needs_box: HashSet<(TypeId, usize)> = HashSet::new();

    // -----------------------------------------------------------------------
    // {Name}Encoder
    // -----------------------------------------------------------------------

    w.open_block(&format!("pub struct {name}Encoder"));
    for field in &delta_fields {
        let rust_ty = rust_type(&field.resolved_type, registry, &empty_needs_box, None);
        w.line(&format!("prev_{}: {},", field.name, rust_ty));
    }
    w.close_block();
    w.blank();

    w.open_block(&format!("impl {name}Encoder"));

    // new()
    w.open_block("pub fn new() -> Self");
    w.open_block("Self");
    for field in &delta_fields {
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("prev_{}: {},", field.name, zero));
    }
    w.close_block();
    w.close_block();
    w.blank();

    // pack()
    w.open_block(&format!(
        "pub fn pack(&mut self, val: &{name}, w: &mut vexil_runtime::BitWriter) -> Result<(), vexil_runtime::EncodeError>"
    ));
    for field in &msg.fields {
        let fname = field.name.as_str();
        if is_delta(&field.encoding) {
            if is_float(&field.resolved_type) {
                w.line(&format!(
                    "let delta_{fname} = val.{fname} - self.prev_{fname};"
                ));
            } else {
                w.line(&format!(
                    "let delta_{fname} = val.{fname}.wrapping_sub(self.prev_{fname});"
                ));
            }
            // Emit write using the INNER encoding (without the Delta wrapper).
            let inner_enc = strip_delta(&field.encoding);
            emit_write(
                w,
                &format!("delta_{fname}"),
                &field.resolved_type,
                &inner_enc,
                registry,
                fname,
            );
            w.line(&format!("self.prev_{fname} = val.{fname};"));
        } else {
            let access = format!("val.{fname}");
            emit_write(
                w,
                &access,
                &field.resolved_type,
                &field.encoding,
                registry,
                fname,
            );
        }
    }
    w.line("w.flush_to_byte_boundary();");
    w.line("Ok(())");
    w.close_block();
    w.blank();

    // reset()
    w.open_block("pub fn reset(&mut self)");
    for field in &delta_fields {
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("self.prev_{} = {};", field.name, zero));
    }
    w.close_block();

    w.close_block(); // impl {Name}Encoder
    w.blank();

    // -----------------------------------------------------------------------
    // {Name}Decoder
    // -----------------------------------------------------------------------

    w.open_block(&format!("pub struct {name}Decoder"));
    for field in &delta_fields {
        let rust_ty = rust_type(&field.resolved_type, registry, &empty_needs_box, None);
        w.line(&format!("prev_{}: {},", field.name, rust_ty));
    }
    w.close_block();
    w.blank();

    w.open_block(&format!("impl {name}Decoder"));

    // new()
    w.open_block("pub fn new() -> Self");
    w.open_block("Self");
    for field in &delta_fields {
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("prev_{}: {},", field.name, zero));
    }
    w.close_block();
    w.close_block();
    w.blank();

    // unpack()
    w.open_block(&format!(
        "pub fn unpack(&mut self, r: &mut vexil_runtime::BitReader<'_>) -> Result<{name}, vexil_runtime::DecodeError>"
    ));
    for field in &msg.fields {
        let fname = field.name.as_str();
        if is_delta(&field.encoding) {
            let inner_enc = strip_delta(&field.encoding);
            let delta_var = format!("delta_{fname}");
            // Read the delta value using the INNER encoding.
            emit_read(
                w,
                &delta_var,
                &field.resolved_type,
                &inner_enc,
                registry,
                fname,
            );
            // Reconstruct the original value.
            if is_float(&field.resolved_type) {
                w.line(&format!("let {fname} = self.prev_{fname} + {delta_var};"));
            } else {
                w.line(&format!(
                    "let {fname} = self.prev_{fname}.wrapping_add({delta_var});"
                ));
            }
            w.line(&format!("self.prev_{fname} = {fname};"));
        } else {
            emit_read(
                w,
                fname,
                &field.resolved_type,
                &field.encoding,
                registry,
                fname,
            );
        }
    }
    w.line("r.flush_to_byte_boundary();");
    w.open_block(&format!("Ok({name}"));
    for field in &msg.fields {
        w.line(&format!("{},", field.name));
    }
    w.dedent();
    w.line("})");

    w.close_block(); // fn unpack
    w.blank();

    // reset()
    w.open_block("pub fn reset(&mut self)");
    for field in &delta_fields {
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("self.prev_{} = {};", field.name, zero));
    }
    w.close_block();

    w.close_block(); // impl {Name}Decoder
    w.blank();
}
