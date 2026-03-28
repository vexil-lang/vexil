use vexil_lang::ast::PrimitiveType;
use vexil_lang::ir::{Encoding, FieldEncoding, MessageDef, ResolvedType, TypeRegistry};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::{go_type, to_pascal_case};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns true if a field uses delta encoding.
fn is_delta(enc: &FieldEncoding) -> bool {
    matches!(enc.encoding, Encoding::Delta(_))
}

/// Returns the Go zero literal for delta prev-state initialisation.
fn zero_literal(ty: &ResolvedType) -> &'static str {
    match ty {
        ResolvedType::Primitive(PrimitiveType::F32 | PrimitiveType::F64) => "0.0",
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

    w.open_block(&format!("type {name}Encoder struct"));
    for field in &delta_fields {
        let go_ty = go_type(&field.resolved_type, registry);
        let camel = to_camel_lower(&field.name);
        w.line(&format!("prev{camel} {go_ty}"));
    }
    w.close_block();
    w.blank();

    // Constructor
    w.open_block(&format!("func New{name}Encoder() *{name}Encoder"));
    w.line(&format!("return &{name}Encoder{{}}"));
    w.close_block();
    w.blank();

    // Pack method
    w.open_block(&format!(
        "func (e *{name}Encoder) Pack(val *{name}, w *vexil.BitWriter) error"
    ));
    for field in &msg.fields {
        let fname = field.name.as_str();
        let pascal = to_pascal_case(fname);
        if is_delta(&field.encoding) {
            let camel = to_camel_lower(fname);
            let inner_enc = strip_delta(&field.encoding);
            let go_ty = go_type(&field.resolved_type, registry);
            // Compute delta
            w.line(&format!(
                "delta{pascal} := {go_ty}(val.{pascal} - e.prev{camel})"
            ));
            // Write the delta
            let delta_var = format!("delta{pascal}");
            emit_write(
                w,
                &delta_var,
                &field.resolved_type,
                &inner_enc,
                registry,
                "w",
                "return err",
            );
            // Update prev
            w.line(&format!("e.prev{camel} = val.{pascal}"));
        } else {
            let access = format!("val.{pascal}");
            emit_write(
                w,
                &access,
                &field.resolved_type,
                &field.encoding,
                registry,
                "w",
                "return err",
            );
        }
    }
    w.line("w.FlushToByteBoundary()");
    w.open_block("if len(val.Unknown) > 0");
    w.line("w.WriteRawBytes(val.Unknown)");
    w.close_block();
    w.line("return nil");
    w.close_block();
    w.blank();

    // Reset method
    w.open_block(&format!("func (e *{name}Encoder) Reset()"));
    for field in &delta_fields {
        let camel = to_camel_lower(&field.name);
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("e.prev{camel} = {zero}"));
    }
    w.close_block();
    w.blank();

    // -----------------------------------------------------------------------
    // {Name}Decoder
    // -----------------------------------------------------------------------

    w.open_block(&format!("type {name}Decoder struct"));
    for field in &delta_fields {
        let go_ty = go_type(&field.resolved_type, registry);
        let camel = to_camel_lower(&field.name);
        w.line(&format!("prev{camel} {go_ty}"));
    }
    w.close_block();
    w.blank();

    // Constructor
    w.open_block(&format!("func New{name}Decoder() *{name}Decoder"));
    w.line(&format!("return &{name}Decoder{{}}"));
    w.close_block();
    w.blank();

    // Unpack method — returns (*T, error)
    let err_ret = "return nil, err";
    w.open_block(&format!(
        "func (d *{name}Decoder) Unpack(r *vexil.BitReader) (*{name}, error)"
    ));
    w.line(&format!("m := &{name}{{}}"));
    for field in &msg.fields {
        let fname = field.name.as_str();
        let pascal = to_pascal_case(fname);
        if is_delta(&field.encoding) {
            let camel = to_camel_lower(fname);
            let inner_enc = strip_delta(&field.encoding);
            let go_ty = go_type(&field.resolved_type, registry);
            // Read the delta
            let delta_var = format!("delta{pascal}");
            w.line(&format!("var {delta_var} {go_ty}"));
            emit_read(
                w,
                &delta_var,
                &field.resolved_type,
                &inner_enc,
                registry,
                "r",
                err_ret,
            );
            // Reconstruct
            w.line(&format!(
                "m.{pascal} = {go_ty}(d.prev{camel} + {delta_var})"
            ));
            // Update prev
            w.line(&format!("d.prev{camel} = m.{pascal}"));
        } else {
            let target = format!("m.{pascal}");
            emit_read(
                w,
                &target,
                &field.resolved_type,
                &field.encoding,
                registry,
                "r",
                err_ret,
            );
        }
    }
    w.line("r.FlushToByteBoundary()");
    w.line("m.Unknown = r.ReadRemaining()");
    w.line("return m, nil");
    w.close_block();
    w.blank();

    // Reset method
    w.open_block(&format!("func (d *{name}Decoder) Reset()"));
    for field in &delta_fields {
        let camel = to_camel_lower(&field.name);
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("d.prev{camel} = {zero}"));
    }
    w.close_block();
    w.blank();
}

/// Convert snake_case to PascalCase for use as Go unexported prev field suffix.
/// Go convention: prev fields use unexported (lowercase first letter).
fn to_camel_lower(name: &str) -> String {
    // For the prev fields, we want PascalCase since they're part of prevTimestamp, etc.
    // The full name is "prev{PascalCase}" which makes it unexported since "prev" starts lowercase.
    to_pascal_case(name)
}
