use std::collections::HashSet;

use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{
    CmpOp, ConstraintOperand, Encoding, FieldConstraint, FieldEncoding, MessageDef, ResolvedType,
    TombstoneDef, TypeDef, TypeId, TypeRegistry,
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
        ResolvedType::BitsInline(names) => names.len() >= 8,
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
        PrimitiveType::I32 | PrimitiveType::U32 | PrimitiveType::F32 | PrimitiveType::Fixed32 => 32,
        PrimitiveType::I64 | PrimitiveType::U64 | PrimitiveType::F64 | PrimitiveType::Fixed64 => 64,
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
// Constraint validation
// ---------------------------------------------------------------------------

/// Generate a Rust boolean expression for a field constraint.
/// `access` is the Rust expression for the field value (e.g., `self.field`).
/// Returns a string like `field >= 0 && field <= 100`.
fn generate_constraint_expr(constraint: &FieldConstraint, access: &str) -> String {
    match constraint {
        FieldConstraint::And(left, right) => {
            let left_expr = generate_constraint_expr(left, access);
            let right_expr = generate_constraint_expr(right, access);
            format!("({} && {})", left_expr, right_expr)
        }
        FieldConstraint::Or(left, right) => {
            let left_expr = generate_constraint_expr(left, access);
            let right_expr = generate_constraint_expr(right, access);
            format!("({} || {})", left_expr, right_expr)
        }
        FieldConstraint::Not(inner) => {
            let inner_expr = generate_constraint_expr(inner, access);
            format!("!({})", inner_expr)
        }
        FieldConstraint::Cmp { op, operand } => {
            let op_str = cmp_op_to_str(*op);
            let operand_str = operand_to_rust(operand);
            format!("{} {} {}", access, op_str, operand_str)
        }
        FieldConstraint::Range {
            low,
            high,
            exclusive_high,
        } => {
            let low_str = operand_to_rust(low);
            let high_str = operand_to_rust(high);
            if *exclusive_high {
                format!("{} >= {} && {} < {}", access, low_str, access, high_str)
            } else {
                format!("{} >= {} && {} <= {}", access, low_str, access, high_str)
            }
        }
        FieldConstraint::LenCmp { op, operand } => {
            let op_str = cmp_op_to_str(*op);
            let operand_str = operand_to_rust(operand);
            format!("({}).len() {} {}", access, op_str, operand_str)
        }
        FieldConstraint::LenRange {
            low,
            high,
            exclusive_high,
        } => {
            let low_str = operand_to_rust(low);
            let high_str = operand_to_rust(high);
            let len_access = format!("({}).len()", access);
            if *exclusive_high {
                format!(
                    "{} >= {} && {} < {}",
                    len_access, low_str, len_access, high_str
                )
            } else {
                format!(
                    "{} >= {} && {} <= {}",
                    len_access, low_str, len_access, high_str
                )
            }
        }
    }
}

fn cmp_op_to_str(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::Ne => "!=",
        CmpOp::Lt => "<",
        CmpOp::Gt => ">",
        CmpOp::Le => "<=",
        CmpOp::Ge => ">=",
    }
}

fn operand_to_rust(operand: &ConstraintOperand) -> String {
    match operand {
        ConstraintOperand::Int(i) => i.to_string(),
        ConstraintOperand::Float(f) => f.to_string(),
        ConstraintOperand::String(s) => format!("{:?}", s),
        ConstraintOperand::Bool(b) => b.to_string(),
        ConstraintOperand::ConstRef(name) => name.to_string(),
    }
}

/// Emit constraint validation code that returns an error if the constraint is violated.
/// For pack(): validates before encoding.
/// For unpack(): validates after decoding.
fn emit_constraint_validation(
    w: &mut CodeWriter,
    constraint: &FieldConstraint,
    access: &str,
    field_name: &str,
) {
    let condition = generate_constraint_expr(constraint, access);
    let negated_condition = format!("!({})", condition);
    w.open_block(&format!("if {}", negated_condition));
    w.line(&format!(
        "return Err(vexil_runtime::EncodeError::ConstraintViolation {{ field: \"{}\", message: format!(\"value violates constraint: expected `{}`\", {}) }});",
        field_name,
        condition.replace("\\\"", "\"").replace("\n", ""),
        access
    ));
    w.close_block();
}

/// Emit constraint validation code for unpack that returns a DecodeError if the constraint is violated.
fn emit_constraint_validation_unpack(
    w: &mut CodeWriter,
    constraint: &FieldConstraint,
    access: &str,
    field_name: &str,
) {
    let condition = generate_constraint_expr(constraint, access);
    let negated_condition = format!("!({})", condition);
    w.open_block(&format!("if {}", negated_condition));
    w.line(&format!(
        "return Err(vexil_runtime::DecodeError::InvalidValue {{ field: \"{}\", message: format!(\"value violates constraint: expected `{}`\", {}) }});",
        field_name,
        condition.replace("\\\"", "\"").replace("\n", ""),
        access
    ));
    w.close_block();
}

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
            | ResolvedType::Set(_)
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
            PrimitiveType::Fixed32 => w.line(&format!("w.write_i32({access});")),
            PrimitiveType::Fixed64 => w.line(&format!("w.write_i64({access});")),
            PrimitiveType::Void => {} // 0 bits — nothing to write
        },
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            w.line(&format!("w.write_bits({access} as u64, {bits}_u8);"));
        }
        ResolvedType::BitsInline(names) => {
            let bits = names.len();
            w.line(&format!("w.write_bits({access} as u64, {bits}_u8);"));
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
        ResolvedType::Set(inner) => {
            w.line(&format!("w.write_leb128({access}.len() as u64);"));
            w.open_block(&format!("for item in {access}"));
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
            PrimitiveType::Fixed32 => w.line(&format!("let {var_name} = r.read_i32()?;")),
            PrimitiveType::Fixed64 => w.line(&format!("let {var_name} = r.read_i64()?;")),
            PrimitiveType::Void => w.line(&format!("let {var_name} = ();")),
        },
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            let uint = crate::types::containing_int_type(bits);
            if s.signed {
                let int = uint.replace('u', "i");
                w.line(&format!(
                    "let {var_name} = r.read_bits({bits}_u8)? as {uint} as {int};"
                ));
            } else {
                w.line(&format!(
                    "let {var_name} = r.read_bits({bits}_u8)? as {uint};"
                ));
            }
        }
        ResolvedType::BitsInline(names) => {
            let bits = names.len() as u8;
            let uint = crate::types::containing_int_type(bits);
            w.line(&format!(
                "let {var_name} = r.read_bits({bits}_u8)? as {uint};"
            ));
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
        ResolvedType::Set(inner) => {
            w.line(&format!(
                "let {var_name}_len = r.read_leb128(10_u8)? as usize;"
            ));
            if let Some(lim) = limit {
                w.line(&format!(
                    "if {var_name}_len as u64 > {lim}_u64 {{ return Err(vexil_runtime::DecodeError::LimitExceeded {{ field: \"{field_name}\", limit: {lim}_u64, actual: {var_name}_len as u64 }}); }}"
                ));
            }
            w.line(&format!(
                "let mut {var_name} = std::collections::BTreeSet::new();"
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
            w.line(&format!("{var_name}.insert({var_name}_item);"));
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

// ---------------------------------------------------------------------------
// emit_tombstone_read — read-and-discard for typed tombstones
// ---------------------------------------------------------------------------

/// Emit code to read and discard bytes for a typed tombstone during decode.
///
/// This advances the `BitReader` cursor past the tombstone's wire data without
/// binding the value to any live field.
fn emit_tombstone_read(w: &mut CodeWriter, ty: &ResolvedType, registry: &TypeRegistry, idx: usize) {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::Bool => w.line("let _ = r.read_bool()?;"),
            PrimitiveType::U8 => w.line("let _ = r.read_u8()?;"),
            PrimitiveType::U16 => w.line("let _ = r.read_u16()?;"),
            PrimitiveType::U32 => w.line("let _ = r.read_u32()?;"),
            PrimitiveType::U64 => w.line("let _ = r.read_u64()?;"),
            PrimitiveType::I8 => w.line("let _ = r.read_i8()?;"),
            PrimitiveType::I16 => w.line("let _ = r.read_i16()?;"),
            PrimitiveType::I32 => w.line("let _ = r.read_i32()?;"),
            PrimitiveType::I64 => w.line("let _ = r.read_i64()?;"),
            PrimitiveType::F32 => w.line("let _ = r.read_f32()?;"),
            PrimitiveType::F64 => w.line("let _ = r.read_f64()?;"),
            PrimitiveType::Fixed32 => w.line("let _ = r.read_i32()?;"),
            PrimitiveType::Fixed64 => w.line("let _ = r.read_i64()?;"),
            PrimitiveType::Void => {} // 0 bits — nothing to read
        },
        ResolvedType::SubByte(s) => {
            w.line(&format!("let _ = r.read_bits({}_u8)?;", s.bits));
        }
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => w.line("let _ = r.read_string()?;"),
            SemanticType::Bytes => w.line("let _ = r.read_bytes()?;"),
            SemanticType::Rgb => {
                w.line("let _ = r.read_u8()?;");
                w.line("let _ = r.read_u8()?;");
                w.line("let _ = r.read_u8()?;");
            }
            SemanticType::Uuid => {
                w.line("let _ = r.read_raw_bytes(16_usize)?;");
            }
            SemanticType::Timestamp => w.line("let _ = r.read_i64()?;"),
            SemanticType::Hash => {
                w.line("let _ = r.read_raw_bytes(32_usize)?;");
            }
        },
        ResolvedType::Named(_) => {
            let var = format!("_tombstone_{idx}");
            w.line("r.enter_recursive()?;");
            w.line(&format!(
                "let {var}: () = {{ let _ = vexil_runtime::Unpack::unpack(r)?; }};"
            ));
            w.line("r.leave_recursive();");
        }
        ResolvedType::Optional(inner) => {
            w.line("let _tombstone_present = r.read_bool()?;");
            if is_byte_aligned(inner, registry) {
                w.line("r.flush_to_byte_boundary();");
            }
            w.open_block("if _tombstone_present");
            emit_tombstone_read(w, inner, registry, idx);
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            let len_var = format!("_tombstone_{idx}_len");
            w.line(&format!("let {len_var} = r.read_leb128(10_u8)? as usize;"));
            w.open_block(&format!("for _ in 0..{len_var}"));
            emit_tombstone_read(w, inner, registry, idx);
            w.close_block();
        }
        ResolvedType::Set(inner) => {
            let len_var = format!("_tombstone_{idx}_len");
            w.line(&format!("let {len_var} = r.read_leb128(10_u8)? as usize;"));
            w.open_block(&format!("for _ in 0..{len_var}"));
            emit_tombstone_read(w, inner, registry, idx);
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            let len_var = format!("_tombstone_{idx}_len");
            w.line(&format!("let {len_var} = r.read_leb128(10_u8)? as usize;"));
            w.open_block(&format!("for _ in 0..{len_var}"));
            emit_tombstone_read(w, k, registry, idx);
            emit_tombstone_read(w, v, registry, idx);
            w.close_block();
        }
        ResolvedType::Result(ok, err) => {
            w.line("let _tombstone_is_ok = r.read_bool()?;");
            w.open_block("if _tombstone_is_ok");
            emit_tombstone_read(w, ok, registry, idx);
            w.dedent();
            w.line("} else {");
            w.indent();
            emit_tombstone_read(w, err, registry, idx);
            w.close_block();
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
    w.line("pub _unknown: Vec<u8>,");
    w.close_block();
    w.blank();

    // Pack impl
    w.open_block(&format!("impl vexil_runtime::Pack for {name}"));
    w.open_block("fn pack(&self, w: &mut vexil_runtime::BitWriter) -> Result<(), vexil_runtime::EncodeError>");
    for field in &msg.fields {
        let access = format!("self.{}", field.name);
        // Validate constraint before encoding
        if let Some(constraint) = &field.constraint {
            emit_constraint_validation(w, constraint, &access, field.name.as_str());
        }
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
    w.open_block("if !self._unknown.is_empty()");
    w.line("w.write_raw_bytes(&self._unknown);");
    w.close_block();
    w.line("Ok(())");
    w.close_block();
    w.close_block();
    w.blank();

    // Unpack impl
    w.open_block(&format!("impl vexil_runtime::Unpack for {name}"));
    w.open_block("fn unpack(r: &mut vexil_runtime::BitReader<'_>) -> Result<Self, vexil_runtime::DecodeError>");

    // Build a sorted sequence of decode actions: live fields + typed tombstones
    // ordered by ordinal so tombstone bytes are read-and-discarded at the correct position.
    enum DecodeAction<'a> {
        Field(&'a vexil_lang::ir::FieldDef),
        Tombstone(&'a TombstoneDef),
    }
    let mut actions: Vec<(u32, DecodeAction<'_>)> = Vec::new();
    for field in &msg.fields {
        actions.push((field.ordinal, DecodeAction::Field(field)));
    }
    for tombstone in &msg.tombstones {
        if tombstone.original_type.is_some() {
            actions.push((tombstone.ordinal, DecodeAction::Tombstone(tombstone)));
        }
    }
    actions.sort_by_key(|(ord, _)| *ord);

    for (idx, (_ord, action)) in actions.iter().enumerate() {
        match action {
            DecodeAction::Field(field) => {
                let var_name = field.name.as_str();
                emit_read(
                    w,
                    var_name,
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    var_name,
                );
                // Validate constraint after decoding
                if let Some(constraint) = &field.constraint {
                    emit_constraint_validation_unpack(w, constraint, var_name, var_name);
                }
            }
            DecodeAction::Tombstone(tombstone) => {
                if let Some(ref ty) = tombstone.original_type {
                    w.line(&format!(
                        "// discard @removed ordinal {}",
                        tombstone.ordinal
                    ));
                    emit_tombstone_read(w, ty, registry, idx);
                }
            }
        }
    }
    w.line("r.flush_to_byte_boundary();");
    w.line("let _unknown = Vec::new();");
    w.open_block("Ok(Self");
    for field in &msg.fields {
        w.line(&format!("{},", field.name));
    }
    w.line("_unknown,");
    w.dedent();
    w.line("})");

    w.close_block();
    w.close_block();
    w.blank();
}
