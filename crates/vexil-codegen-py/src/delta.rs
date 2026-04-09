use vexil_lang::ast::PrimitiveType;
use vexil_lang::ir::{Encoding, FieldEncoding, MessageDef, ResolvedType, TypeRegistry};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::{py_type, to_pascal_case};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_delta(enc: &FieldEncoding) -> bool {
    matches!(enc.encoding, Encoding::Delta(_))
}

fn zero_literal(ty: &ResolvedType) -> &'static str {
    match ty {
        ResolvedType::Primitive(PrimitiveType::F32 | PrimitiveType::F64) => "0.0",
        _ => "0",
    }
}

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

/// Emit `{Name}Encoder` and `{Name}Decoder` classes for a message that
/// contains at least one `@delta` field.
pub fn emit_delta(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry) {
    let delta_fields: Vec<_> = msg
        .fields
        .iter()
        .filter(|f| is_delta(&f.encoding))
        .collect();

    if delta_fields.is_empty() {
        return;
    }

    let name = msg.name.as_str();

    // -----------------------------------------------------------------------
    // {Name}Encoder
    // -----------------------------------------------------------------------

    w.open_block(&format!("class {name}Encoder"));
    w.blank();

    // __init__
    w.open_block("def __init__(self)");
    for field in &delta_fields {
        let py_ty = py_type(&field.resolved_type, registry);
        let attr = format!("_prev_{}", field.name);
        w.line(&format!(
            "self.{attr}: {py_ty} = {}",
            zero_literal(&field.resolved_type)
        ));
    }
    w.close_block();
    w.blank();

    // encode method
    w.open_block(&format!("def encode(self, val: {name}) -> bytes"));
    w.line("w = _BitWriter()");
    for field in &msg.fields {
        let fname = field.name.as_str();
        let pascal = to_pascal_case(fname);
        if is_delta(&field.encoding) {
            let attr = format!("_prev_{fname}");
            let inner_enc = strip_delta(&field.encoding);
            // Compute delta
            w.line(&format!("delta_{fname} = val.{pascal} - self.{attr}"));
            // Write the delta
            emit_write(
                w,
                &format!("delta_{fname}"),
                &field.resolved_type,
                &inner_enc,
                registry,
                "w",
            );
            // Update prev
            w.line(&format!("self.{attr} = val.{pascal}"));
        } else {
            let access = format!("val.{pascal}");
            emit_write(
                w,
                &access,
                &field.resolved_type,
                &field.encoding,
                registry,
                "w",
            );
        }
    }
    w.line("w.flush_to_byte_boundary()");
    w.line("return w.finish()");
    w.close_block();
    w.blank();

    // reset method
    w.open_block("def reset(self)");
    for field in &delta_fields {
        let attr = format!("_prev_{}", field.name);
        w.line(&format!(
            "self.{attr} = {}",
            zero_literal(&field.resolved_type)
        ));
    }
    w.close_block();

    w.close_block();
    w.blank();

    // -----------------------------------------------------------------------
    // {Name}Decoder
    // -----------------------------------------------------------------------

    w.open_block(&format!("class {name}Decoder"));
    w.blank();

    // __init__
    w.open_block("def __init__(self)");
    for field in &delta_fields {
        let py_ty = py_type(&field.resolved_type, registry);
        let attr = format!("_prev_{}", field.name);
        w.line(&format!(
            "self.{attr}: {py_ty} = {}",
            zero_literal(&field.resolved_type)
        ));
    }
    w.close_block();
    w.blank();

    // decode method
    w.open_block(&format!("def decode(self, data: bytes) -> {name}"));
    w.line("r = _BitReader(data)");
    w.line(&format!("m = {name}.__new__({name})"));
    for field in &msg.fields {
        let fname = field.name.as_str();
        let pascal = to_pascal_case(fname);
        if is_delta(&field.encoding) {
            let attr = format!("_prev_{fname}");
            let inner_enc = strip_delta(&field.encoding);
            // Read delta
            w.line(&format!("delta_{fname} = None"));
            emit_read(
                w,
                &format!("delta_{fname}"),
                &field.resolved_type,
                &inner_enc,
                registry,
                "r",
            );
            // Reconstruct
            w.line(&format!("m.{pascal} = self.{attr} + delta_{fname}"));
            // Update prev
            w.line(&format!("self.{attr} = m.{pascal}"));
        } else {
            let target = format!("m.{pascal}");
            emit_read(
                w,
                &target,
                &field.resolved_type,
                &field.encoding,
                registry,
                "r",
            );
        }
    }
    w.line("r.flush_to_byte_boundary()");
    w.line("return m");
    w.close_block();
    w.blank();

    // reset method
    w.open_block("def reset(self)");
    for field in &delta_fields {
        let attr = format!("_prev_{}", field.name);
        w.line(&format!(
            "self.{attr} = {}",
            zero_literal(&field.resolved_type)
        ));
    }
    w.close_block();

    w.close_block();
    w.blank();
}
