use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{
    CmpOp, ConfigDef, ConstraintOperand, Encoding, FieldConstraint, FieldEncoding, MessageDef,
    ResolvedType, TypeDef, TypeRegistry,
};

use crate::emit::CodeWriter;
use crate::types::{go_collection_key_type, go_type, to_pascal_case};

// ---------------------------------------------------------------------------
// Byte-alignment helper
// ---------------------------------------------------------------------------

/// Returns true if the type is byte-aligned (i.e., not sub-byte).
pub fn is_byte_aligned(ty: &ResolvedType, registry: &TypeRegistry) -> bool {
    match ty {
        ResolvedType::Primitive(PrimitiveType::Bool) => false,
        ResolvedType::SubByte(_) => false,
        ResolvedType::BitsInline(names) => names.len() >= 8,
        ResolvedType::Named(id) => {
            if let Some(TypeDef::Enum(e)) = registry.get(*id) {
                e.wire_bits >= 8
            } else {
                true
            }
        }
        // An optional always starts with a one-bit presence flag. Nested
        // optional flags therefore remain contiguous until a present payload
        // itself requires byte alignment.
        ResolvedType::Optional(_) => false,
        _ => true,
    }
}

fn canonical_less_expr(
    ty: &ResolvedType,
    registry: &TypeRegistry,
    left: &str,
    right: &str,
) -> String {
    match ty {
        ResolvedType::Primitive(PrimitiveType::Bool) => format!("!{left} && {right}"),
        ResolvedType::Semantic(SemanticType::Uuid) => {
            format!("string({left}[:]) < string({right}[:])")
        }
        ResolvedType::Named(id) => match registry.get(*id) {
            Some(TypeDef::Newtype(newtype)) => {
                canonical_less_expr(&newtype.inner_type, registry, left, right)
            }
            _ => format!("{left} < {right}"),
        },
        _ => format!("{left} < {right}"),
    }
}

fn emit_sorted_collection_keys(
    w: &mut CodeWriter,
    access: &str,
    key_type: &ResolvedType,
    registry: &TypeRegistry,
    prefix: &str,
) -> String {
    let keys = format!(
        "{prefix}{}",
        access
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
    );
    let key_go = go_collection_key_type(key_type, registry);
    w.line(&format!("{keys} := make([]{key_go}, 0, len({access}))"));
    w.open_block(&format!("for key := range {access}"));
    w.line(&format!("{keys} = append({keys}, key)"));
    w.close_block();
    w.open_block(&format!("sort.Slice({keys}, func(i, j int) bool"));
    let less = canonical_less_expr(
        key_type,
        registry,
        &format!("{keys}[i]"),
        &format!("{keys}[j]"),
    );
    w.line(&format!("return {less}"));
    w.dedent();
    w.line("})");
    keys
}

// ---------------------------------------------------------------------------
// Primitive bits helper
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
// LEB128 max bytes helper
// ---------------------------------------------------------------------------

fn leb128_max_bytes_for_type(ty: &ResolvedType) -> u8 {
    match ty {
        ResolvedType::Primitive(PrimitiveType::U64 | PrimitiveType::I64) => 10,
        ResolvedType::Primitive(PrimitiveType::U32 | PrimitiveType::I32) => 5,
        ResolvedType::Primitive(PrimitiveType::U16 | PrimitiveType::I16) => 3,
        ResolvedType::Primitive(PrimitiveType::U8 | PrimitiveType::I8) => 2,
        _ => 10,
    }
}

// ---------------------------------------------------------------------------
// Constraint validation
// ---------------------------------------------------------------------------

/// Generate a Go boolean expression for a field constraint.
/// `access` is the Go expression for the field value (e.g., `m.Field`).
fn generate_constraint_expr_go(constraint: &FieldConstraint, access: &str) -> String {
    match constraint {
        FieldConstraint::And(left, right) => {
            let left_expr = generate_constraint_expr_go(left, access);
            let right_expr = generate_constraint_expr_go(right, access);
            format!("({} && {})", left_expr, right_expr)
        }
        FieldConstraint::Or(left, right) => {
            let left_expr = generate_constraint_expr_go(left, access);
            let right_expr = generate_constraint_expr_go(right, access);
            format!("({} || {})", left_expr, right_expr)
        }
        FieldConstraint::Not(inner) => {
            let inner_expr = generate_constraint_expr_go(inner, access);
            format!("!({})", inner_expr)
        }
        FieldConstraint::Cmp { op, operand } => {
            let op_str = cmp_op_to_str_go(*op);
            let operand_str = operand_to_go(operand);
            format!("{} {} {}", access, op_str, operand_str)
        }
        FieldConstraint::Range {
            low,
            high,
            exclusive_high,
        } => {
            let low_str = operand_to_go(low);
            let high_str = operand_to_go(high);
            if *exclusive_high {
                format!("{} >= {} && {} < {}", access, low_str, access, high_str)
            } else {
                format!("{} >= {} && {} <= {}", access, low_str, access, high_str)
            }
        }
        FieldConstraint::LenCmp { op, operand } => {
            let op_str = cmp_op_to_str_go(*op);
            let operand_str = operand_to_go(operand);
            format!("len({}) {} {}", access, op_str, operand_str)
        }
        FieldConstraint::LenRange {
            low,
            high,
            exclusive_high,
        } => {
            let low_str = operand_to_go(low);
            let high_str = operand_to_go(high);
            let len_access = format!("len({})", access);
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

fn cmp_op_to_str_go(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::Ne => "!=",
        CmpOp::Lt => "<",
        CmpOp::Gt => ">",
        CmpOp::Le => "<=",
        CmpOp::Ge => ">=",
    }
}

fn operand_to_go(operand: &ConstraintOperand) -> String {
    match operand {
        ConstraintOperand::Int(i) => i.to_string(),
        ConstraintOperand::Float(f) => f.to_string(),
        ConstraintOperand::String(s) => format!("{:?}", s),
        ConstraintOperand::Bool(b) => b.to_string(),
        ConstraintOperand::ConstRef(name) => name.to_string(),
    }
}

/// Emit constraint validation code that returns an error if the constraint is violated.
fn emit_constraint_validation_go(
    w: &mut CodeWriter,
    constraint: &FieldConstraint,
    access: &str,
    field_name: &str,
    _err_return: &str,
) {
    let condition = generate_constraint_expr_go(constraint, access);
    let negated_condition = format!("!({})", condition);
    w.open_block(&format!("if {}", negated_condition));
    w.line(&format!(
        "return fmt.Errorf(`constraint violation for field \"{}\": value %v violates constraint: expected [{}]`, {})",
        field_name,
        condition.replace('\n', ""),
        access
    ));
    w.close_block();
}

fn constraint_value_access_go(ty: &ResolvedType, access: &str) -> (Vec<String>, String) {
    let mut guards = Vec::new();
    let mut value = access.to_string();
    let mut current = ty;
    while let ResolvedType::Optional(inner) = current {
        guards.push(format!("{value} != nil"));
        value = format!("(*{value})");
        current = inner;
    }
    (guards, value)
}

fn emit_field_constraint_validation_go(
    w: &mut CodeWriter,
    constraint: &FieldConstraint,
    ty: &ResolvedType,
    access: &str,
    field_name: &str,
    err_return: &str,
) {
    let (guards, value) = constraint_value_access_go(ty, access);
    if !guards.is_empty() {
        w.open_block(&format!("if {}", guards.join(" && ")));
    }
    emit_constraint_validation_go(w, constraint, &value, field_name, err_return);
    if !guards.is_empty() {
        w.close_block();
    }
}

/// Emit code to write a value to a BitWriter.
///
/// `access` is the Go expression for the value.
/// `writer` is the variable name of the BitWriter (e.g. "w" or "pw").
/// `err_return` is the Go error return statement (e.g. "return err" or "return nil, err").
pub fn emit_write(
    w: &mut CodeWriter,
    access: &str,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    writer: &str,
    err_return: &str,
) {
    match &enc.encoding {
        Encoding::Varint => {
            let is_signed = matches!(
                ty,
                ResolvedType::Primitive(
                    PrimitiveType::I8
                        | PrimitiveType::I16
                        | PrimitiveType::I32
                        | PrimitiveType::I64
                )
            );
            if is_signed {
                let bits = match ty {
                    ResolvedType::Primitive(p) => primitive_bits(p),
                    _ => 64,
                };
                w.line(&format!(
                    "{writer}.WriteLeb128(uint64({access}) & ((1 << {bits}) - 1))"
                ));
            } else {
                w.line(&format!("{writer}.WriteLeb128(uint64({access}))"));
            }
            return;
        }
        Encoding::ZigZag => {
            let type_bits = match ty {
                ResolvedType::Primitive(p) => primitive_bits(p),
                _ => 64,
            };
            w.line(&format!(
                "{writer}.WriteZigZag(int64({access}), {type_bits})"
            ));
            return;
        }
        Encoding::Delta(inner) => {
            let base_enc = FieldEncoding {
                encoding: *inner.clone(),
                limit: enc.limit,
            };
            emit_write(w, access, ty, &base_enc, registry, writer, err_return);
            return;
        }
        Encoding::Default => {}
        _ => {}
    }

    emit_write_type(w, access, ty, registry, writer, err_return);
}

fn emit_write_type(
    w: &mut CodeWriter,
    access: &str,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    writer: &str,
    err_return: &str,
) {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::Bool => w.line(&format!("{writer}.WriteBool({access})")),
            PrimitiveType::U8 => w.line(&format!("{writer}.WriteU8({access})")),
            PrimitiveType::U16 => w.line(&format!("{writer}.WriteU16({access})")),
            PrimitiveType::U32 => w.line(&format!("{writer}.WriteU32({access})")),
            PrimitiveType::U64 => w.line(&format!("{writer}.WriteU64({access})")),
            PrimitiveType::I8 => w.line(&format!("{writer}.WriteI8({access})")),
            PrimitiveType::I16 => w.line(&format!("{writer}.WriteI16({access})")),
            PrimitiveType::I32 => w.line(&format!("{writer}.WriteI32({access})")),
            PrimitiveType::I64 => w.line(&format!("{writer}.WriteI64({access})")),
            PrimitiveType::F32 => w.line(&format!("{writer}.WriteF32({access})")),
            PrimitiveType::F64 => w.line(&format!("{writer}.WriteF64({access})")),
            PrimitiveType::Fixed32 => w.line(&format!("{writer}.WriteI32({access})")),
            PrimitiveType::Fixed64 => w.line(&format!("{writer}.WriteI64({access})")),
            PrimitiveType::Void => {}
        },
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            w.line(&format!("{writer}.WriteBits(uint64({access}), {bits})"));
        }
        ResolvedType::BitsInline(names) => {
            let bits = names.len();
            w.line(&format!("{writer}.WriteBits(uint64({access}), {bits})"));
        }
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => w.line(&format!("{writer}.WriteString({access})")),
            SemanticType::Bytes => w.line(&format!("{writer}.WriteBytes({access})")),
            SemanticType::Rgb => {
                w.line(&format!("{writer}.WriteU8({access}[0])"));
                w.line(&format!("{writer}.WriteU8({access}[1])"));
                w.line(&format!("{writer}.WriteU8({access}[2])"));
            }
            SemanticType::Uuid => w.line(&format!("{writer}.WriteRawBytes({access}[:])")),
            SemanticType::Timestamp => w.line(&format!("{writer}.WriteI64({access})")),
            SemanticType::Hash => w.line(&format!("{writer}.WriteRawBytes({access}[:])")),
        },
        ResolvedType::Named(id) => {
            let type_name = match registry.get(*id) {
                Some(def) => match def {
                    TypeDef::Message(m) => m.name.to_string(),
                    TypeDef::Enum(e) => e.name.to_string(),
                    TypeDef::Flags(f) => f.name.to_string(),
                    TypeDef::Union(u) => u.name.to_string(),
                    TypeDef::Newtype(n) => n.name.to_string(),
                    _ => "Unknown".to_string(),
                },
                None => "Unknown".to_string(),
            };
            match registry.get(*id) {
                Some(TypeDef::Message(_)) => {
                    w.line(&format!(
                        "if err := {writer}.EnterRecursive(); err != nil {{"
                    ));
                    w.indent();
                    w.line(err_return);
                    w.close_block();
                    w.line(&format!("if err := {access}.Pack({writer}); err != nil {{"));
                    w.indent();
                    w.line(err_return);
                    w.close_block();
                    w.line(&format!("{writer}.LeaveRecursive()"));
                }
                Some(TypeDef::Enum(_)) => {
                    w.line(&format!("{access}.Pack({writer})"));
                }
                Some(TypeDef::Flags(_)) => {
                    w.line(&format!("{access}.Pack({writer})"));
                }
                Some(TypeDef::Union(_)) => {
                    w.line(&format!(
                        "if err := Pack{type_name}({access}, {writer}); err != nil {{"
                    ));
                    w.indent();
                    w.line(err_return);
                    w.close_block();
                }
                Some(TypeDef::Newtype(_)) => {
                    w.line(&format!("Pack{type_name}({access}, {writer})"));
                }
                _ => {
                    w.line(&format!("// Unknown type: {type_name}"));
                }
            }
        }
        ResolvedType::Optional(inner) => {
            w.line(&format!("{writer}.WriteBool({access} != nil)"));
            w.open_block(&format!("if {access} != nil"));
            if is_byte_aligned(inner, registry) {
                w.line(&format!("{writer}.FlushToByteBoundary()"));
            }
            // For Named types (messages, etc.), the pointer is already the right type
            // for method calls. For primitives, we need to dereference.
            match inner.as_ref() {
                ResolvedType::Named(_) => {
                    emit_write_type(w, access, inner, registry, writer, err_return);
                }
                _ => {
                    let deref = format!("(*{access})");
                    emit_write_type(w, &deref, inner, registry, writer, err_return);
                }
            }
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            w.line(&format!("{writer}.WriteLeb128(uint64(len({access})))"));
            w.open_block(&format!("for _, item := range {access}"));
            emit_write_type(w, "item", inner, registry, writer, err_return);
            w.close_block();
        }
        ResolvedType::Set(inner) => {
            w.line(&format!("{writer}.WriteLeb128(uint64(len({access})))"));
            let keys = emit_sorted_collection_keys(w, access, inner, registry, "setKeys");
            w.open_block(&format!("for _, item := range {keys}"));
            let item_access =
                if matches!(inner.as_ref(), ResolvedType::Semantic(SemanticType::Bytes)) {
                    "[]byte(item)"
                } else {
                    "item"
                };
            emit_write_type(w, item_access, inner, registry, writer, err_return);
            w.close_block();
        }
        ResolvedType::FixedArray(inner, _) => {
            w.open_block(&format!("for _, item := range {access}"));
            emit_write_type(w, "item", inner, registry, writer, err_return);
            w.close_block();
        }
        ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => {
            w.open_block(&format!("for _, item := range {access}"));
            emit_write_type(w, "item", inner, registry, writer, err_return);
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            w.line(&format!("{writer}.WriteLeb128(uint64(len({access})))"));
            let keys = emit_sorted_collection_keys(w, access, k, registry, "mapKeys");
            w.open_block(&format!("for _, mapK := range {keys}"));
            w.line(&format!("mapV := {access}[mapK]"));
            let key_access = if matches!(k.as_ref(), ResolvedType::Semantic(SemanticType::Bytes)) {
                "[]byte(mapK)"
            } else {
                "mapK"
            };
            emit_write_type(w, key_access, k, registry, writer, err_return);
            emit_write_type(w, "mapV", v, registry, writer, err_return);
            w.close_block();
        }
        ResolvedType::Result(ok, err_ty) => {
            w.open_block(&format!("if {access}.Ok != nil"));
            w.line(&format!("{writer}.WriteBool(true)"));
            emit_write_type(
                w,
                &format!("*{access}.Ok"),
                ok,
                registry,
                writer,
                err_return,
            );
            w.dedent();
            w.line("} else {");
            w.indent();
            w.line(&format!("{writer}.WriteBool(false)"));
            emit_write_type(
                w,
                &format!("*{access}.Err"),
                err_ty,
                registry,
                writer,
                err_return,
            );
            w.close_block();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// emit_read
// ---------------------------------------------------------------------------

/// Emit code to read a value from a BitReader and assign to `target`.
///
/// `target` is the Go lvalue to assign to (e.g. "m.DeviceID" or a local var).
/// `reader` is the variable name of the BitReader (e.g. "r" or "pr").
/// `err_return` is the Go error return statement (e.g. "return err" or "return nil, err").
pub fn emit_read(
    w: &mut CodeWriter,
    target: &str,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    reader: &str,
    err_return: &str,
) {
    match &enc.encoding {
        Encoding::Varint => {
            let max_bytes = leb128_max_bytes_for_type(ty);
            let go_ty = go_type(ty, registry);
            w.open_block("");
            w.line(&format!("raw, err := {reader}.ReadLeb128({max_bytes})"));
            w.open_block("if err != nil");
            w.line(err_return);
            w.close_block();
            w.line(&format!("{target} = {go_ty}(raw)"));
            w.close_block();
            return;
        }
        Encoding::ZigZag => {
            let type_bits = match ty {
                ResolvedType::Primitive(p) => primitive_bits(p),
                _ => 64,
            };
            let max_bytes = leb128_max_bytes_for_type(ty);
            let go_ty = go_type(ty, registry);
            w.open_block("");
            w.line(&format!(
                "raw, err := {reader}.ReadZigZag({type_bits}, {max_bytes})"
            ));
            w.open_block("if err != nil");
            w.line(err_return);
            w.close_block();
            w.line(&format!("{target} = {go_ty}(raw)"));
            w.close_block();
            return;
        }
        Encoding::Delta(inner) => {
            let base_enc = FieldEncoding {
                encoding: *inner.clone(),
                limit: enc.limit,
            };
            emit_read(w, target, ty, &base_enc, registry, reader, err_return);
            return;
        }
        Encoding::Default => {}
        _ => {}
    }

    emit_read_type(w, target, ty, registry, reader, err_return);
}

fn emit_read_type(
    w: &mut CodeWriter,
    target: &str,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    reader: &str,
    err_return: &str,
) {
    match ty {
        ResolvedType::Primitive(p) => {
            let read_fn = match p {
                PrimitiveType::Bool => "ReadBool",
                PrimitiveType::U8 => "ReadU8",
                PrimitiveType::U16 => "ReadU16",
                PrimitiveType::U32 => "ReadU32",
                PrimitiveType::U64 => "ReadU64",
                PrimitiveType::I8 => "ReadI8",
                PrimitiveType::I16 => "ReadI16",
                PrimitiveType::I32 => "ReadI32",
                PrimitiveType::I64 => "ReadI64",
                PrimitiveType::F32 => "ReadF32",
                PrimitiveType::F64 => "ReadF64",
                PrimitiveType::Fixed32 => "ReadI32",
                PrimitiveType::Fixed64 => "ReadI64",
                PrimitiveType::Void => {
                    return;
                }
            };
            w.open_block("");
            w.line(&format!("v, err := {reader}.{read_fn}()"));
            w.open_block("if err != nil");
            w.line(err_return);
            w.close_block();
            w.line(&format!("{target} = v"));
            w.close_block();
        }
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            w.open_block("");
            w.line(&format!("v, err := {reader}.ReadBits({bits})"));
            w.open_block("if err != nil");
            w.line(err_return);
            w.close_block();
            if s.signed {
                if bits <= 56 {
                    let shift = 64 - bits;
                    w.line(&format!(
                        "{target} := int8((uint64(v) << {shift}) >> {shift})"
                    ));
                } else {
                    w.line(&format!("{target} := int8(v & ((1 << {bits}) - 1))"));
                }
            } else {
                w.line(&format!("{target} = uint8(v)"));
            }
            w.close_block();
        }
        ResolvedType::BitsInline(names) => {
            let bits = names.len();
            let go_ty = go_type(ty, registry);
            w.open_block("");
            w.line(&format!("v, err := {reader}.ReadBits({bits})"));
            w.open_block("if err != nil");
            w.line(err_return);
            w.close_block();
            w.line(&format!("{target} = {go_ty}(v)"));
            w.close_block();
        }
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => {
                w.open_block("");
                w.line(&format!("v, err := {reader}.ReadString()"));
                w.open_block("if err != nil");
                w.line(err_return);
                w.close_block();
                w.line(&format!("{target} = v"));
                w.close_block();
            }
            SemanticType::Bytes => {
                w.open_block("");
                w.line(&format!("v, err := {reader}.ReadBytes()"));
                w.open_block("if err != nil");
                w.line(err_return);
                w.close_block();
                w.line(&format!("{target} = v"));
                w.close_block();
            }
            SemanticType::Rgb => {
                w.open_block("");
                w.line(&format!("r0, err := {reader}.ReadU8()"));
                w.open_block("if err != nil");
                w.line(err_return);
                w.close_block();
                w.line(&format!("r1, err := {reader}.ReadU8()"));
                w.open_block("if err != nil");
                w.line(err_return);
                w.close_block();
                w.line(&format!("r2, err := {reader}.ReadU8()"));
                w.open_block("if err != nil");
                w.line(err_return);
                w.close_block();
                w.line(&format!("{target} = [3]uint8{{r0, r1, r2}}"));
                w.close_block();
            }
            SemanticType::Uuid => {
                w.open_block("");
                w.line(&format!("v, err := {reader}.ReadRawBytes(16)"));
                w.open_block("if err != nil");
                w.line(err_return);
                w.close_block();
                w.line(&format!("copy({target}[:], v)"));
                w.close_block();
            }
            SemanticType::Timestamp => {
                w.open_block("");
                w.line(&format!("v, err := {reader}.ReadI64()"));
                w.open_block("if err != nil");
                w.line(err_return);
                w.close_block();
                w.line(&format!("{target} = v"));
                w.close_block();
            }
            SemanticType::Hash => {
                w.open_block("");
                w.line(&format!("v, err := {reader}.ReadRawBytes(32)"));
                w.open_block("if err != nil");
                w.line(err_return);
                w.close_block();
                w.line(&format!("copy({target}[:], v)"));
                w.close_block();
            }
        },
        ResolvedType::Named(id) => {
            let type_name = match registry.get(*id) {
                Some(def) => match def {
                    TypeDef::Message(m) => m.name.to_string(),
                    TypeDef::Enum(e) => e.name.to_string(),
                    TypeDef::Flags(f) => f.name.to_string(),
                    TypeDef::Union(u) => u.name.to_string(),
                    TypeDef::Newtype(n) => n.name.to_string(),
                    _ => "Unknown".to_string(),
                },
                None => "Unknown".to_string(),
            };
            match registry.get(*id) {
                Some(TypeDef::Message(_)) => {
                    w.open_block("");
                    w.line(&format!(
                        "if err := {reader}.EnterRecursive(); err != nil {{"
                    ));
                    w.indent();
                    w.line(err_return);
                    w.close_block();
                    w.line(&format!(
                        "if err := {target}.Unpack({reader}); err != nil {{"
                    ));
                    w.indent();
                    w.line(err_return);
                    w.close_block();
                    w.line(&format!("{reader}.LeaveRecursive()"));
                    w.close_block();
                }
                Some(TypeDef::Enum(_)) => {
                    w.line(&format!(
                        "if err := {target}.Unpack({reader}); err != nil {{"
                    ));
                    w.indent();
                    w.line(err_return);
                    w.close_block();
                }
                Some(TypeDef::Flags(_)) => {
                    w.line(&format!(
                        "if err := {target}.Unpack({reader}); err != nil {{"
                    ));
                    w.indent();
                    w.line(err_return);
                    w.close_block();
                }
                Some(TypeDef::Union(_)) => {
                    w.open_block("");
                    w.line(&format!("v, err := Unpack{type_name}({reader})"));
                    w.open_block("if err != nil");
                    w.line(err_return);
                    w.close_block();
                    w.line(&format!("{target} = v"));
                    w.close_block();
                }
                Some(TypeDef::Newtype(_)) => {
                    w.open_block("");
                    w.line(&format!("v, err := Unpack{type_name}({reader})"));
                    w.open_block("if err != nil");
                    w.line(err_return);
                    w.close_block();
                    w.line(&format!("{target} = v"));
                    w.close_block();
                }
                _ => {
                    w.line(&format!("// Unknown type: {type_name}"));
                }
            }
        }
        ResolvedType::Optional(inner) => {
            w.open_block("");
            w.line(&format!("present, err := {reader}.ReadBool()"));
            w.open_block("if err != nil && err != vexil.ErrUnexpectedEOF");
            w.line(err_return);
            w.close_block();
            w.open_block("if err == nil");
            w.open_block("if present");
            if is_byte_aligned(inner, registry) {
                w.line(&format!("{reader}.FlushToByteBoundary()"));
            }
            let inner_go = go_collection_key_type(inner, registry);
            let opt_value = if target == "optVal" || target.ends_with("OptVal") {
                format!("nested{}", to_pascal_case(target))
            } else {
                "optVal".to_string()
            };
            w.line(&format!("var {opt_value} {inner_go}"));
            emit_read_type(w, &opt_value, inner, registry, reader, err_return);
            w.line(&format!("{target} = &{opt_value}"));
            w.close_block();
            w.close_block();
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            w.open_block("");
            w.line(&format!("arrLen, err := {reader}.ReadLeb128(4)"));
            w.open_block("if err != nil");
            w.line(err_return);
            w.close_block();
            let inner_go = go_type(inner, registry);
            w.line(&format!("{target} = make([]{inner_go}, arrLen)"));
            w.open_block("for i := uint64(0); i < arrLen; i++");
            emit_read_type(
                w,
                &format!("{target}[i]"),
                inner,
                registry,
                reader,
                err_return,
            );
            w.close_block();
            w.close_block();
        }
        ResolvedType::Set(inner) => {
            w.open_block("");
            w.line(&format!("setLen, err := {reader}.ReadLeb128(4)"));
            w.open_block("if err != nil");
            w.line(err_return);
            w.close_block();
            let inner_go = go_type(inner, registry);
            w.line(&format!(
                "{target} = make(map[{inner_go}]struct{{}}, setLen)"
            ));
            w.open_block("for i := uint64(0); i < setLen; i++");
            if matches!(inner.as_ref(), ResolvedType::Semantic(SemanticType::Bytes)) {
                w.line("var itemBytes []byte");
                emit_read_type(w, "itemBytes", inner, registry, reader, err_return);
                w.line("item := string(itemBytes)");
            } else {
                w.line(&format!("var item {inner_go}"));
                emit_read_type(w, "item", inner, registry, reader, err_return);
            }
            w.line(&format!("{target}[item] = struct{{}}{{}}"));
            w.close_block();
            w.close_block();
        }
        ResolvedType::FixedArray(inner, size) => {
            let n = *size;
            let inner_go = go_type(inner, registry);
            w.line(&format!("{target} = [{n}]{inner_go}{{}}"));
            w.open_block(&format!("for i := 0; i < {n}; i++"));
            emit_read_type(
                w,
                &format!("{target}[i]"),
                inner,
                registry,
                reader,
                err_return,
            );
            w.close_block();
        }
        ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => {
            let n = match ty {
                ResolvedType::Vec2(_) => 2,
                ResolvedType::Vec3(_) => 3,
                ResolvedType::Vec4(_) | ResolvedType::Quat(_) => 4,
                ResolvedType::Mat3(_) => 9,
                ResolvedType::Mat4(_) => 16,
                _ => unreachable!(),
            };
            let inner_go = go_type(inner, registry);
            w.line(&format!("{target} = [{n}]{inner_go}{{}}"));
            w.open_block(&format!("for i := 0; i < {n}; i++"));
            emit_read_type(
                w,
                &format!("{target}[i]"),
                inner,
                registry,
                reader,
                err_return,
            );
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            w.open_block("");
            w.line(&format!("mapLen, err := {reader}.ReadLeb128(4)"));
            w.open_block("if err != nil");
            w.line(err_return);
            w.close_block();
            let k_go = go_collection_key_type(k, registry);
            let v_go = go_type(v, registry);
            w.line(&format!("{target} = make(map[{k_go}]{v_go}, mapLen)"));
            w.open_block("for i := uint64(0); i < mapLen; i++");
            if matches!(k.as_ref(), ResolvedType::Semantic(SemanticType::Bytes)) {
                w.line("var mapKeyBytes []byte");
            } else {
                w.line(&format!("var mapKey {k_go}"));
            }
            w.line(&format!("var mapVal {v_go}"));
            if matches!(k.as_ref(), ResolvedType::Semantic(SemanticType::Bytes)) {
                emit_read_type(w, "mapKeyBytes", k, registry, reader, err_return);
                w.line("mapKey := string(mapKeyBytes)");
            } else {
                emit_read_type(w, "mapKey", k, registry, reader, err_return);
            }
            emit_read_type(w, "mapVal", v, registry, reader, err_return);
            w.line(&format!("{target}[mapKey] = mapVal"));
            w.close_block();
            w.close_block();
        }
        ResolvedType::Result(ok, err_ty) => {
            w.open_block("");
            w.line(&format!("isOk, err := {reader}.ReadBool()"));
            w.open_block("if err != nil");
            w.line(err_return);
            w.close_block();
            w.open_block("if isOk");
            emit_read_type(w, &format!("{target}_ok"), ok, registry, reader, err_return);
            w.line(&format!("{target}.Ok = &{target}_ok"));
            w.dedent();
            w.line("} else {");
            w.indent();
            emit_read_type(
                w,
                &format!("{target}_err"),
                err_ty,
                registry,
                reader,
                err_return,
            );
            w.line(&format!("{target}.Err = &{target}_err"));
            w.close_block();
            w.close_block();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// emit_message
// ---------------------------------------------------------------------------

/// Emit a complete message: struct + Pack method + Unpack method.
pub fn emit_message(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry) {
    let name = msg.name.as_str();

    // Struct definition
    w.open_block(&format!("type {name} struct"));
    for field in &msg.fields {
        let go_ty = go_type(&field.resolved_type, registry);
        let field_name = to_pascal_case(&field.name);
        w.line(&format!("{field_name} {go_ty}"));
    }
    w.line("Unknown []byte");
    w.close_block();
    w.blank();

    let err_ret = "return err";

    // Pack method
    w.open_block(&format!("func (m *{name}) Pack(w *vexil.BitWriter) error"));
    for field in &msg.fields {
        let field_name = to_pascal_case(&field.name);
        let access = format!("m.{field_name}");
        // Validate constraint before encoding
        if let Some(constraint) = &field.constraint {
            emit_field_constraint_validation_go(
                w,
                constraint,
                &field.resolved_type,
                &access,
                field.name.as_str(),
                err_ret,
            );
        }
        emit_write(
            w,
            &access,
            &field.resolved_type,
            &field.encoding,
            registry,
            "w",
            err_ret,
        );
    }
    w.line("w.FlushToByteBoundary()");
    w.open_block("if len(m.Unknown) > 0");
    w.line("w.WriteRawBytes(m.Unknown)");
    w.close_block();
    w.line("return nil");
    w.close_block();
    w.blank();

    // Unpack method
    w.open_block(&format!(
        "func (m *{name}) Unpack(r *vexil.BitReader) error"
    ));

    let mut fields: Vec<_> = msg.fields.iter().collect();
    fields.sort_by_key(|field| field.ordinal);
    for field in fields {
        let field_name_pascal = to_pascal_case(&field.name);
        let target = format!("m.{field_name_pascal}");
        emit_read(
            w,
            &target,
            &field.resolved_type,
            &field.encoding,
            registry,
            "r",
            err_ret,
        );
        // Validate constraint after decoding
        if let Some(constraint) = &field.constraint {
            emit_field_constraint_validation_go(
                w,
                constraint,
                &field.resolved_type,
                &target,
                field.name.as_str(),
                err_ret,
            );
        }
    }
    w.line("r.FlushToByteBoundary()");
    w.line("m.Unknown = nil");
    w.line("return nil");
    w.close_block();
    w.blank();
}

// ---------------------------------------------------------------------------
// emit_config
// ---------------------------------------------------------------------------

/// Emit a config type: struct only (no codec).
pub fn emit_config(w: &mut CodeWriter, cfg: &ConfigDef, registry: &TypeRegistry) {
    let name = cfg.name.as_str();

    w.open_block(&format!("type {name} struct"));
    for field in &cfg.fields {
        let go_ty = go_type(&field.resolved_type, registry);
        let field_name = to_pascal_case(&field.name);
        w.line(&format!("{field_name} {go_ty}"));
    }
    w.close_block();
    w.blank();
}
