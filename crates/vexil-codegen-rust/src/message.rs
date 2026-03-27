use std::collections::HashSet;

use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{
    Encoding, FieldEncoding, MessageDef, ResolvedType, TypeDef, TypeId, TypeRegistry,
};

use crate::annotations::{emit_field_annotations, emit_tombstones, emit_type_annotations};
use crate::emit::CodeWriter;
use crate::types::rust_type;

// ---------------------------------------------------------------------------
// Byte-alignment helper
// ---------------------------------------------------------------------------

/// Returns true if the type is byte-aligned (i.e., not sub-byte).
///
/// Returns false for: Bool, SubByte, exhaustive enum with wire_bits < 8.
pub fn is_byte_aligned(ty: &ResolvedType, registry: &TypeRegistry) -> bool {
    match ty {
        ResolvedType::Primitive(PrimitiveType::Bool) => false,
        ResolvedType::SubByte(_) => false,
        ResolvedType::Named(id) => {
            // Check if this is an exhaustive enum with small wire_bits
            if let Some(TypeDef::Enum(e)) = registry.get(*id) {
                e.wire_bits >= 8
            } else {
                true
            }
        }
        ResolvedType::Optional(inner) => is_byte_aligned(inner, registry),
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// Primitive type bits helper
// ---------------------------------------------------------------------------

fn primitive_bits(p: &PrimitiveType) -> u8 {
    match p {
        PrimitiveType::I8 | PrimitiveType::U8 => 8,
        PrimitiveType::I16 | PrimitiveType::U16 => 16,
        PrimitiveType::I32 | PrimitiveType::U32 | PrimitiveType::F32 => 32,
        PrimitiveType::I64 | PrimitiveType::U64 | PrimitiveType::F64 => 64,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Copy-type helper
// ---------------------------------------------------------------------------

/// Returns true if the type is `Copy` in Rust — primitives and sub-byte types.
/// Used to determine whether array/optional/result items need dereferencing
/// when accessed through a reference (`*item` vs `item`).
fn is_copy_type(ty: &ResolvedType) -> bool {
    matches!(ty, ResolvedType::Primitive(_) | ResolvedType::SubByte(_))
}

// ---------------------------------------------------------------------------
// emit_write
// ---------------------------------------------------------------------------

/// Emit code to write a field to `w: &mut BitWriter`.
///
/// `access` is the Rust expression for the value (e.g. `self.name` or `&self.data`).
/// For `Encoding::Delta`, this function is a no-op (the delta module handles it).
pub fn emit_write(
    w: &mut CodeWriter,
    access: &str,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    field_name: &str,
) {
    // Check non-default encoding first
    match &enc.encoding {
        Encoding::Varint => {
            if let Some(limit) = enc.limit {
                w.line(&format!(
                    "if ({access} as u64) > {limit}_u64 {{ return Err(vexil_runtime::EncodeError::LimitExceeded {{ field: \"{field_name}\", limit: {limit}_u64, actual: {access} as u64 }}); }}"
                ));
            }
            w.line(&format!("w.write_leb128({access} as u64);"));
            return;
        }
        Encoding::ZigZag => {
            if let Some(limit) = enc.limit {
                w.line(&format!(
                    "if ({access} as i64).unsigned_abs() > {limit}_u64 {{ return Err(vexil_runtime::EncodeError::LimitExceeded {{ field: \"{field_name}\", limit: {limit}_u64, actual: ({access} as i64).unsigned_abs() }}); }}"
                ));
            }
            let type_bits = match ty {
                ResolvedType::Primitive(p) => primitive_bits(p),
                _ => 64,
            };
            w.line(&format!("w.write_zigzag({access} as i64, {type_bits}_u8);"));
            return;
        }
        Encoding::Delta(inner) => {
            // For standard Pack, write the field using the inner (base) encoding.
            // The DeltaEncoder handles delta sequences separately.
            let base_enc = FieldEncoding {
                encoding: *inner.clone(),
                limit: enc.limit,
            };
            emit_write(w, access, ty, &base_enc, registry, field_name);
            return;
        }
        Encoding::Default => {} // fall through to type dispatch
        _ => {}                 // non_exhaustive guard
    }

    // Emit limit check for default encoding on collections/strings
    if let Some(limit) = enc.limit {
        match ty {
            ResolvedType::Array(_)
            | ResolvedType::Map(_, _)
            | ResolvedType::Semantic(SemanticType::String)
            | ResolvedType::Semantic(SemanticType::Bytes) => {
                w.line(&format!(
                    "if ({access}).len() as u64 > {limit}_u64 {{ return Err(vexil_runtime::EncodeError::LimitExceeded {{ field: \"{field_name}\", limit: {limit}_u64, actual: ({access}).len() as u64 }}); }}"
                ));
            }
            _ => {}
        }
    }

    emit_write_type(w, access, ty, registry, field_name);
}

#[allow(clippy::only_used_in_recursion)]
fn emit_write_type(
    w: &mut CodeWriter,
    access: &str,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    field_name: &str,
) {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::Bool => w.line(&format!("w.write_bool({access});")),
            PrimitiveType::U8 => w.line(&format!("w.write_u8({access});")),
            PrimitiveType::U16 => w.line(&format!("w.write_u16({access});")),
            PrimitiveType::U32 => w.line(&format!("w.write_u32({access});")),
            PrimitiveType::U64 => w.line(&format!("w.write_u64({access});")),
            PrimitiveType::I8 => w.line(&format!("w.write_i8({access});")),
            PrimitiveType::I16 => w.line(&format!("w.write_i16({access});")),
            PrimitiveType::I32 => w.line(&format!("w.write_i32({access});")),
            PrimitiveType::I64 => w.line(&format!("w.write_i64({access});")),
            PrimitiveType::F32 => w.line(&format!("w.write_f32({access});")),
            PrimitiveType::F64 => w.line(&format!("w.write_f64({access});")),
            PrimitiveType::Void => {} // 0 bits — nothing to write
        },
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            if s.signed {
                w.line(&format!("w.write_bits({access} as u8 as u64, {bits}_u8);"));
            } else {
                w.line(&format!("w.write_bits({access} as u64, {bits}_u8);"));
            }
        }
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => w.line(&format!("w.write_string(&{access});")),
            SemanticType::Bytes => w.line(&format!("w.write_bytes(&{access});")),
            SemanticType::Rgb => {
                w.line(&format!("w.write_u8({access}.0);"));
                w.line(&format!("w.write_u8({access}.1);"));
                w.line(&format!("w.write_u8({access}.2);"));
            }
            SemanticType::Uuid => w.line(&format!("w.write_raw_bytes(&{access});")),
            SemanticType::Timestamp => w.line(&format!("w.write_i64({access});")),
            SemanticType::Hash => w.line(&format!("w.write_raw_bytes(&{access});")),
        },
        ResolvedType::Named(_) => {
            w.line("w.enter_recursive()?;");
            w.line(&format!("{access}.pack(w)?;"));
            w.line("w.leave_recursive();");
        }
        ResolvedType::Optional(inner) => {
            // Presence bit
            w.line(&format!("w.write_bool({access}.is_some());"));
            // If inner is byte-aligned, flush before conditional
            if is_byte_aligned(inner, registry) {
                w.line("w.flush_to_byte_boundary();");
                // Hmm, actually flush is on writer side. We only flush after the presence
                // bit if the inner type requires byte alignment. The spec says flush the
                // bit-stream before writing the inner value. Let's keep the flush here
                // only when needed.
            }
            w.open_block(&format!("if let Some(ref inner_val) = {access}"));
            let inner_access = if is_copy_type(inner) {
                "*inner_val"
            } else {
                "inner_val"
            };
            emit_write_type(w, inner_access, inner, registry, field_name);
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            w.line(&format!("w.write_leb128({access}.len() as u64);"));
            w.open_block(&format!("for item in &{access}"));
            let item_access = if is_copy_type(inner) { "*item" } else { "item" };
            emit_write_type(w, item_access, inner, registry, field_name);
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            w.line(&format!("w.write_leb128({access}.len() as u64);"));
            w.open_block(&format!("for (map_k, map_v) in &{access}"));
            let k_access = if is_copy_type(k) { "*map_k" } else { "map_k" };
            let v_access = if is_copy_type(v) { "*map_v" } else { "map_v" };
            emit_write_type(w, k_access, k, registry, field_name);
            emit_write_type(w, v_access, v, registry, field_name);
            w.close_block();
        }
        ResolvedType::Result(ok, err) => {
            w.open_block(&format!("match &{access}"));
            w.open_block("Ok(ok_val) =>");
            w.line("w.write_bool(true);");
            let ok_access = if is_copy_type(ok) {
                "*ok_val"
            } else {
                "ok_val"
            };
            emit_write_type(w, ok_access, ok, registry, field_name);
            w.close_block();
            w.open_block("Err(err_val) =>");
            w.line("w.write_bool(false);");
            let err_access = if is_copy_type(err) {
                "*err_val"
            } else {
                "err_val"
            };
            emit_write_type(w, err_access, err, registry, field_name);
            w.close_block();
            w.close_block();
        }
        _ => {} // non_exhaustive guard
    }
}

// ---------------------------------------------------------------------------
// emit_read
// ---------------------------------------------------------------------------

/// Emit code to read a field from `r: &mut BitReader<'_>`.
///
/// Binds the result to `var_name`.
pub fn emit_read(
    w: &mut CodeWriter,
    var_name: &str,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    field_name: &str,
) {
    match &enc.encoding {
        Encoding::Varint => {
            // max_bytes: 10 covers u64 LEB128
            w.line(&format!("let {var_name}_raw = r.read_leb128(10_u8)?;"));
            if let Some(limit) = enc.limit {
                w.line(&format!(
                    "if {var_name}_raw > {limit}_u64 {{ return Err(vexil_runtime::DecodeError::LimitExceeded {{ field: \"{field_name}\", limit: {limit}_u64, actual: {var_name}_raw }}); }}"
                ));
            }
            // Cast to the appropriate Rust type
            let rust_ty = read_cast_for_varint(ty);
            w.line(&format!(
                "let {var_name}: {rust_ty} = {var_name}_raw as {rust_ty};"
            ));
            return;
        }
        Encoding::ZigZag => {
            let type_bits = match ty {
                ResolvedType::Primitive(p) => primitive_bits(p),
                _ => 64,
            };
            // max_bytes: 10 for i64 zigzag
            w.line(&format!(
                "let {var_name}_raw = r.read_zigzag({type_bits}_u8, 10_u8)?;"
            ));
            if let Some(limit) = enc.limit {
                w.line(&format!(
                    "if {var_name}_raw.unsigned_abs() > {limit}_u64 {{ return Err(vexil_runtime::DecodeError::LimitExceeded {{ field: \"{field_name}\", limit: {limit}_u64, actual: {var_name}_raw.unsigned_abs() }}); }}"
                ));
            }
            let rust_ty = read_cast_for_zigzag(ty);
            w.line(&format!(
                "let {var_name}: {rust_ty} = {var_name}_raw as {rust_ty};"
            ));
            return;
        }
        Encoding::Delta(inner) => {
            // For standard Unpack, read the field using the inner (base) encoding.
            // The DeltaDecoder handles delta sequences separately.
            let base_enc = FieldEncoding {
                encoding: *inner.clone(),
                limit: enc.limit,
            };
            emit_read(w, var_name, ty, &base_enc, registry, field_name);
            return;
        }
        Encoding::Default => {}
        _ => {} // non_exhaustive guard
    }

    emit_read_type(w, var_name, ty, registry, field_name, enc.limit);
}

fn emit_read_type(
    w: &mut CodeWriter,
    var_name: &str,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    field_name: &str,
    limit: Option<u64>,
) {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::Bool => w.line(&format!("let {var_name} = r.read_bool()?;")),
            PrimitiveType::U8 => w.line(&format!("let {var_name} = r.read_u8()?;")),
            PrimitiveType::U16 => w.line(&format!("let {var_name} = r.read_u16()?;")),
            PrimitiveType::U32 => w.line(&format!("let {var_name} = r.read_u32()?;")),
            PrimitiveType::U64 => w.line(&format!("let {var_name} = r.read_u64()?;")),
            PrimitiveType::I8 => w.line(&format!("let {var_name} = r.read_i8()?;")),
            PrimitiveType::I16 => w.line(&format!("let {var_name} = r.read_i16()?;")),
            PrimitiveType::I32 => w.line(&format!("let {var_name} = r.read_i32()?;")),
            PrimitiveType::I64 => w.line(&format!("let {var_name} = r.read_i64()?;")),
            PrimitiveType::F32 => w.line(&format!("let {var_name} = r.read_f32()?;")),
            PrimitiveType::F64 => w.line(&format!("let {var_name} = r.read_f64()?;")),
            PrimitiveType::Void => w.line(&format!("let {var_name} = ();")),
        },
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            if s.signed {
                w.line(&format!(
                    "let {var_name} = r.read_bits({bits}_u8)? as u8 as i8;"
                ));
            } else {
                w.line(&format!("let {var_name} = r.read_bits({bits}_u8)? as u8;"));
            }
        }
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => {
                w.line(&format!("let {var_name} = r.read_string()?;"));
                if let Some(lim) = limit {
                    w.line(&format!(
                        "if {var_name}.len() as u64 > {lim}_u64 {{ return Err(vexil_runtime::DecodeError::LimitExceeded {{ field: \"{field_name}\", limit: {lim}_u64, actual: {var_name}.len() as u64 }}); }}"
                    ));
                }
            }
            SemanticType::Bytes => {
                w.line(&format!("let {var_name} = r.read_bytes()?;"));
                if let Some(lim) = limit {
                    w.line(&format!(
                        "if {var_name}.len() as u64 > {lim}_u64 {{ return Err(vexil_runtime::DecodeError::LimitExceeded {{ field: \"{field_name}\", limit: {lim}_u64, actual: {var_name}.len() as u64 }}); }}"
                    ));
                }
            }
            SemanticType::Rgb => {
                w.line(&format!("let {var_name}_0 = r.read_u8()?;"));
                w.line(&format!("let {var_name}_1 = r.read_u8()?;"));
                w.line(&format!("let {var_name}_2 = r.read_u8()?;"));
                w.line(&format!(
                    "let {var_name} = ({var_name}_0, {var_name}_1, {var_name}_2);"
                ));
            }
            SemanticType::Uuid => {
                w.line(&format!(
                    "let {var_name}_bytes = r.read_raw_bytes(16_usize)?;"
                ));
                w.line(&format!(
                    "let {var_name}: [u8; 16] = {var_name}_bytes.try_into().map_err(|_| vexil_runtime::DecodeError::UnexpectedEof)?;"
                ));
            }
            SemanticType::Timestamp => {
                w.line(&format!("let {var_name} = r.read_i64()?;"));
            }
            SemanticType::Hash => {
                w.line(&format!(
                    "let {var_name}_bytes = r.read_raw_bytes(32_usize)?;"
                ));
                w.line(&format!(
                    "let {var_name}: [u8; 32] = {var_name}_bytes.try_into().map_err(|_| vexil_runtime::DecodeError::UnexpectedEof)?;"
                ));
            }
        },
        ResolvedType::Named(_) => {
            w.line("r.enter_recursive()?;");
            w.line(&format!(
                "let {var_name} = vexil_runtime::Unpack::unpack(r)?;"
            ));
            w.line("r.leave_recursive();");
        }
        ResolvedType::Optional(inner) => {
            w.line(&format!("let {var_name}_present = r.read_bool()?;"));
            if is_byte_aligned(inner, registry) {
                w.line("r.flush_to_byte_boundary();");
            }
            w.open_block(&format!("let {var_name} = if {var_name}_present"));
            emit_read_type(
                w,
                &format!("{var_name}_inner"),
                inner,
                registry,
                field_name,
                None,
            );
            w.line(&format!("Some({var_name}_inner)"));
            w.close_block();
            w.open_block("else");
            w.line("None");
            w.close_block();
            w.append(";");
            w.append("\n");
        }
        ResolvedType::Array(inner) => {
            w.line(&format!(
                "let {var_name}_len = r.read_leb128(10_u8)? as usize;"
            ));
            if let Some(lim) = limit {
                w.line(&format!(
                    "if {var_name}_len as u64 > {lim}_u64 {{ return Err(vexil_runtime::DecodeError::LimitExceeded {{ field: \"{field_name}\", limit: {lim}_u64, actual: {var_name}_len as u64 }}); }}"
                ));
            }
            w.line(&format!(
                "let mut {var_name} = Vec::with_capacity({var_name}_len);"
            ));
            w.open_block(&format!("for _ in 0..{var_name}_len"));
            emit_read_type(
                w,
                &format!("{var_name}_item"),
                inner,
                registry,
                field_name,
                None,
            );
            w.line(&format!("{var_name}.push({var_name}_item);"));
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            w.line(&format!(
                "let {var_name}_len = r.read_leb128(10_u8)? as usize;"
            ));
            if let Some(lim) = limit {
                w.line(&format!(
                    "if {var_name}_len as u64 > {lim}_u64 {{ return Err(vexil_runtime::DecodeError::LimitExceeded {{ field: \"{field_name}\", limit: {lim}_u64, actual: {var_name}_len as u64 }}); }}"
                ));
            }
            w.line(&format!(
                "let mut {var_name} = std::collections::BTreeMap::new();"
            ));
            w.open_block(&format!("for _ in 0..{var_name}_len"));
            emit_read_type(w, &format!("{var_name}_k"), k, registry, field_name, None);
            emit_read_type(w, &format!("{var_name}_v"), v, registry, field_name, None);
            w.line(&format!("{var_name}.insert({var_name}_k, {var_name}_v);"));
            w.close_block();
        }
        ResolvedType::Result(ok, err) => {
            w.line(&format!("let {var_name}_is_ok = r.read_bool()?;"));
            w.open_block(&format!("let {var_name} = if {var_name}_is_ok"));
            emit_read_type(w, &format!("{var_name}_ok"), ok, registry, field_name, None);
            w.line(&format!("Ok({var_name}_ok)"));
            w.close_block();
            w.open_block("else");
            emit_read_type(
                w,
                &format!("{var_name}_err"),
                err,
                registry,
                field_name,
                None,
            );
            w.line(&format!("Err({var_name}_err)"));
            w.close_block();
            w.append(";");
            w.append("\n");
        }
        _ => {} // non_exhaustive guard
    }
}

fn read_cast_for_varint(ty: &ResolvedType) -> &'static str {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::U8 => "u8",
            PrimitiveType::U16 => "u16",
            PrimitiveType::U32 => "u32",
            PrimitiveType::U64 => "u64",
            _ => "u64",
        },
        _ => "u64",
    }
}

fn read_cast_for_zigzag(ty: &ResolvedType) -> &'static str {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::I8 => "i8",
            PrimitiveType::I16 => "i16",
            PrimitiveType::I32 => "i32",
            PrimitiveType::I64 => "i64",
            _ => "i64",
        },
        _ => "i64",
    }
}

// ---------------------------------------------------------------------------
// emit_message
// ---------------------------------------------------------------------------

/// Emit a complete message struct with Pack and Unpack implementations.
pub fn emit_message(
    w: &mut CodeWriter,
    msg: &MessageDef,
    registry: &TypeRegistry,
    needs_box: &HashSet<(TypeId, usize)>,
    type_id: TypeId,
) {
    let name = msg.name.as_str();

    // Tombstone comments
    emit_tombstones(w, name, &msg.tombstones);

    // Type annotations
    emit_type_annotations(w, &msg.annotations);
    w.line("#[derive(Debug, Clone, PartialEq)]");

    // Struct definition
    w.open_block(&format!("pub struct {name}"));
    for (fi, field) in msg.fields.iter().enumerate() {
        emit_field_annotations(w, &field.annotations);
        let field_rust_type = rust_type(
            &field.resolved_type,
            registry,
            needs_box,
            Some((type_id, fi)),
        );
        w.line(&format!("pub {}: {},", field.name, field_rust_type));
    }
    w.close_block();
    w.blank();

    // Pack impl
    w.open_block(&format!("impl vexil_runtime::Pack for {name}"));
    w.open_block("fn pack(&self, w: &mut vexil_runtime::BitWriter) -> Result<(), vexil_runtime::EncodeError>");
    for field in &msg.fields {
        let access = format!("self.{}", field.name);
        emit_write(
            w,
            &access,
            &field.resolved_type,
            &field.encoding,
            registry,
            field.name.as_str(),
        );
    }
    w.line("w.flush_to_byte_boundary();");
    w.line("Ok(())");
    w.close_block();
    w.close_block();
    w.blank();

    // Unpack impl
    w.open_block(&format!("impl vexil_runtime::Unpack for {name}"));
    w.open_block("fn unpack(r: &mut vexil_runtime::BitReader<'_>) -> Result<Self, vexil_runtime::DecodeError>");
    for field in &msg.fields {
        let var_name = field.name.as_str();
        emit_read(
            w,
            var_name,
            &field.resolved_type,
            &field.encoding,
            registry,
            var_name,
        );
    }
    w.line("r.flush_to_byte_boundary();");
    w.open_block("Ok(Self");
    for field in &msg.fields {
        w.line(&format!("{},", field.name));
    }
    w.dedent();
    w.line("})");

    w.close_block();
    w.close_block();
    w.blank();
}
