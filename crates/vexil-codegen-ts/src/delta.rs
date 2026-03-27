use vexil_lang::ast::PrimitiveType;
use vexil_lang::ir::{Encoding, FieldEncoding, MessageDef, ResolvedType, TypeRegistry};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::ts_type;

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

/// Returns true if the type is a 64-bit integer (bigint in TypeScript).
fn is_bigint(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(PrimitiveType::U64) | ResolvedType::Primitive(PrimitiveType::I64)
    )
}

/// Returns true if the type is an unsigned integer (u8, u16, u32, or sub-byte unsigned).
fn is_unsigned(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(PrimitiveType::U8 | PrimitiveType::U16 | PrimitiveType::U32)
    ) || matches!(ty, ResolvedType::SubByte(s) if !s.signed)
}

/// Returns the TypeScript zero literal for delta prev-state initialisation.
fn zero_literal(ty: &ResolvedType) -> &'static str {
    match ty {
        ResolvedType::Primitive(PrimitiveType::F32 | PrimitiveType::F64) => "0.0",
        ResolvedType::Primitive(PrimitiveType::U64 | PrimitiveType::I64) => "0n",
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

/// Convert a snake_case field name to camelCase for use in TypeScript property names.
fn to_camel(name: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for ch in name.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Emit the delta subtraction expression for encoding.
fn emit_encode_delta(w: &mut CodeWriter, field_name: &str, ty: &ResolvedType) {
    let camel = to_camel(field_name);
    if is_bigint(ty) {
        let bits = if matches!(ty, ResolvedType::Primitive(PrimitiveType::U64)) {
            64
        } else {
            // i64: plain subtraction (bigint wraps naturally when decoded)
            w.line(&format!(
                "const delta_{field_name} = v.{field_name} - this.prev{camel};"
            ));
            return;
        };
        w.line(&format!(
            "const delta_{field_name} = BigInt.asUintN({bits}, v.{field_name} - this.prev{camel});"
        ));
    } else if is_float(ty) {
        w.line(&format!(
            "const delta_{field_name} = v.{field_name} - this.prev{camel};"
        ));
    } else if is_unsigned(ty) {
        // Unsigned wrapping subtraction
        let mask_expr = match ty {
            ResolvedType::Primitive(PrimitiveType::U8) => {
                format!("(v.{field_name} - this.prev{camel}) & 0xFF")
            }
            ResolvedType::Primitive(PrimitiveType::U16) => {
                format!("(v.{field_name} - this.prev{camel}) & 0xFFFF")
            }
            ResolvedType::Primitive(PrimitiveType::U32) => {
                format!("(v.{field_name} - this.prev{camel}) >>> 0")
            }
            ResolvedType::SubByte(s) => {
                let mask = (1u32 << s.bits) - 1;
                format!("(v.{field_name} - this.prev{camel}) & 0x{mask:X}")
            }
            _ => format!("(v.{field_name} - this.prev{camel}) >>> 0"),
        };
        w.line(&format!("const delta_{field_name} = {mask_expr};"));
    } else {
        // Signed integers (i8, i16, i32)
        let suffix = match ty {
            ResolvedType::Primitive(PrimitiveType::I32) => " | 0",
            _ => "",
        };
        w.line(&format!(
            "const delta_{field_name} = (v.{field_name} - this.prev{camel}){suffix};"
        ));
    }
}

/// Emit the delta addition expression for decoding.
fn emit_decode_reconstruct(w: &mut CodeWriter, field_name: &str, ty: &ResolvedType) {
    let camel = to_camel(field_name);
    if is_bigint(ty) {
        if matches!(ty, ResolvedType::Primitive(PrimitiveType::U64)) {
            w.line(&format!(
                "const {field_name} = BigInt.asUintN(64, this.prev{camel} + delta_{field_name});"
            ));
        } else {
            // i64
            w.line(&format!(
                "const {field_name} = this.prev{camel} + delta_{field_name};"
            ));
        }
    } else if is_float(ty) {
        w.line(&format!(
            "const {field_name} = this.prev{camel} + delta_{field_name};"
        ));
    } else if is_unsigned(ty) {
        let mask_expr = match ty {
            ResolvedType::Primitive(PrimitiveType::U8) => {
                format!("(this.prev{camel} + delta_{field_name}) & 0xFF")
            }
            ResolvedType::Primitive(PrimitiveType::U16) => {
                format!("(this.prev{camel} + delta_{field_name}) & 0xFFFF")
            }
            ResolvedType::Primitive(PrimitiveType::U32) => {
                format!("(this.prev{camel} + delta_{field_name}) >>> 0")
            }
            ResolvedType::SubByte(s) => {
                let mask = (1u32 << s.bits) - 1;
                format!("(this.prev{camel} + delta_{field_name}) & 0x{mask:X}")
            }
            _ => format!("(this.prev{camel} + delta_{field_name}) >>> 0"),
        };
        w.line(&format!("const {field_name} = {mask_expr};"));
    } else {
        // Signed integers (i8, i16, i32)
        let suffix = match ty {
            ResolvedType::Primitive(PrimitiveType::I32) => " | 0",
            _ => "",
        };
        w.line(&format!(
            "const {field_name} = (this.prev{camel} + delta_{field_name}){suffix};"
        ));
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Emit `{Name}Encoder` and `{Name}Decoder` classes for a message that
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

    // -----------------------------------------------------------------------
    // {Name}Encoder
    // -----------------------------------------------------------------------

    w.open_block(&format!("export class {name}Encoder"));

    // Private prev fields
    for field in &delta_fields {
        let camel = to_camel(&field.name);
        let ts = ts_type(&field.resolved_type, registry);
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("private prev{camel}: {ts} = {zero};"));
    }
    w.blank();

    // encode()
    w.open_block(&format!("encode(v: {name}, w: BitWriter): void"));
    for field in &msg.fields {
        let fname = field.name.as_str();
        if is_delta(&field.encoding) {
            let camel = to_camel(fname);
            emit_encode_delta(w, fname, &field.resolved_type);
            // Write the delta value using the INNER encoding (Delta stripped).
            let inner_enc = strip_delta(&field.encoding);
            emit_write(
                w,
                &format!("delta_{fname}"),
                &field.resolved_type,
                &inner_enc,
                registry,
                "w",
            );
            w.line(&format!("this.prev{camel} = v.{fname};"));
        } else {
            let access = format!("v.{fname}");
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
    w.line("w.flushToByteBoundary();");
    w.open_block("if (v._unknown.length > 0)");
    w.line("w.writeRawBytes(v._unknown);");
    w.close_block();
    w.close_block();
    w.blank();

    // reset()
    w.open_block("reset(): void");
    for field in &delta_fields {
        let camel = to_camel(&field.name);
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("this.prev{camel} = {zero};"));
    }
    w.close_block();

    w.close_block(); // class {Name}Encoder
    w.blank();

    // -----------------------------------------------------------------------
    // {Name}Decoder
    // -----------------------------------------------------------------------

    w.open_block(&format!("export class {name}Decoder"));

    // Private prev fields
    for field in &delta_fields {
        let camel = to_camel(&field.name);
        let ts = ts_type(&field.resolved_type, registry);
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("private prev{camel}: {ts} = {zero};"));
    }
    w.blank();

    // decode()
    w.open_block(&format!("decode(r: BitReader): {name}"));
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
                "r",
            );
            // Reconstruct the original value.
            emit_decode_reconstruct(w, fname, &field.resolved_type);
            let camel = to_camel(fname);
            w.line(&format!("this.prev{camel} = {fname};"));
        } else {
            emit_read(
                w,
                fname,
                &field.resolved_type,
                &field.encoding,
                registry,
                "r",
            );
        }
    }
    w.line("r.flushToByteBoundary();");
    w.line("const _unknown = r.readRemaining();");
    let field_names: Vec<&str> = msg.fields.iter().map(|f| f.name.as_str()).collect();
    let mut all_names = field_names;
    all_names.push("_unknown");
    w.line(&format!("return {{ {} }};", all_names.join(", ")));
    w.close_block();
    w.blank();

    // reset()
    w.open_block("reset(): void");
    for field in &delta_fields {
        let camel = to_camel(&field.name);
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("this.prev{camel} = {zero};"));
    }
    w.close_block();

    w.close_block(); // class {Name}Decoder
    w.blank();
}
