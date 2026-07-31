use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::codegen::portable::PortableFunction;
use vexil_lang::ir::{
    CmpOp, ConfigDef, ConstraintOperand, Encoding, FieldConstraint, FieldEncoding, MessageDef,
    ResolvedType, TombstoneDef, TypeDef, TypeRegistry,
};

use crate::emit::CodeWriter;
use crate::types::{py_ident, py_type};

fn local_name(target: &str, suffix: &str) -> String {
    let mut stem = String::new();
    for ch in target.chars() {
        if ch.is_ascii_alphanumeric() {
            stem.push(ch);
        } else {
            stem.push_str(&format!("_{:x}_", ch as u32));
        }
    }
    format!("_vexil_{stem}_{suffix}")
}

// ---------------------------------------------------------------------------
// Constraint validation
// ---------------------------------------------------------------------------

fn generate_constraint_expr_py(constraint: &FieldConstraint, access: &str) -> String {
    match constraint {
        FieldConstraint::And(left, right) => {
            let left_expr = generate_constraint_expr_py(left, access);
            let right_expr = generate_constraint_expr_py(right, access);
            format!("({left_expr}) and ({right_expr})")
        }
        FieldConstraint::Or(left, right) => {
            let left_expr = generate_constraint_expr_py(left, access);
            let right_expr = generate_constraint_expr_py(right, access);
            format!("({left_expr}) or ({right_expr})")
        }
        FieldConstraint::Not(inner) => {
            let inner_expr = generate_constraint_expr_py(inner, access);
            format!("not ({inner_expr})")
        }
        FieldConstraint::Cmp { op, operand } => {
            let op_str = cmp_op_to_str_py(*op);
            let operand_str = operand_to_py(operand);
            format!("{access} {op_str} {operand_str}")
        }
        FieldConstraint::Range {
            low,
            high,
            exclusive_high,
        } => {
            let low_str = operand_to_py(low);
            let high_str = operand_to_py(high);
            if *exclusive_high {
                format!("{low_str} <= {access} < {high_str}")
            } else {
                format!("{low_str} <= {access} <= {high_str}")
            }
        }
        FieldConstraint::LenCmp { op, operand } => {
            let op_str = cmp_op_to_str_py(*op);
            let operand_str = operand_to_py(operand);
            format!("len({access}) {op_str} {operand_str}")
        }
        FieldConstraint::LenRange {
            low,
            high,
            exclusive_high,
        } => {
            let low_str = operand_to_py(low);
            let high_str = operand_to_py(high);
            if *exclusive_high {
                format!("{low_str} <= len({access}) < {high_str}")
            } else {
                format!("{low_str} <= len({access}) <= {high_str}")
            }
        }
    }
}

fn cmp_op_to_str_py(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::Ne => "!=",
        CmpOp::Lt => "<",
        CmpOp::Gt => ">",
        CmpOp::Le => "<=",
        CmpOp::Ge => ">=",
    }
}

fn operand_to_py(operand: &ConstraintOperand) -> String {
    match operand {
        ConstraintOperand::Int(i) => i.to_string(),
        ConstraintOperand::Float(f) => f.to_string(),
        ConstraintOperand::String(s) => format!("\"{s}\""),
        ConstraintOperand::Bool(b) => b.to_string(),
        ConstraintOperand::ConstRef(name) => name.to_string(),
    }
}

fn emit_constraint_validation_py(
    w: &mut CodeWriter,
    constraint: &FieldConstraint,
    access: &str,
    field_name: &str,
) {
    let condition = generate_constraint_expr_py(constraint, access);
    w.open_block(&format!("if not ({condition})"));
    w.line(&format!(
        "raise ValueError(f\"constraint violation for field '{field_name}': value {{{access}}} violates constraint\")"
    ));
    w.close_block();
}

// ---------------------------------------------------------------------------
// emit_write - write a value to BitWriter
// ---------------------------------------------------------------------------

pub fn emit_write(
    w: &mut CodeWriter,
    access: &str,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    writer: &str,
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
                w.line(&format!("{writer}.write_leb128_signed({access})"));
            } else {
                w.line(&format!("{writer}.write_leb128({access})"));
            }
            return;
        }
        Encoding::ZigZag => {
            w.line(&format!("{writer}.write_zigzag({access})"));
            return;
        }
        Encoding::Delta(inner) => {
            let base_enc = FieldEncoding {
                encoding: *inner.clone(),
                limit: enc.limit,
            };
            emit_write(w, access, ty, &base_enc, registry, writer);
            return;
        }
        Encoding::Default => {}
        _ => {}
    }

    emit_write_type(w, access, ty, registry, writer);
}

fn emit_write_type(
    w: &mut CodeWriter,
    access: &str,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    writer: &str,
) {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::Bool => w.line(&format!("{writer}.write_bool({access})")),
            PrimitiveType::U8 => w.line(&format!("{writer}.write_u8({access})")),
            PrimitiveType::U16 => w.line(&format!("{writer}.write_u16({access})")),
            PrimitiveType::U32 => w.line(&format!("{writer}.write_u32({access})")),
            PrimitiveType::U64 => w.line(&format!("{writer}.write_u64({access})")),
            PrimitiveType::I8 => w.line(&format!("{writer}.write_i8({access})")),
            PrimitiveType::I16 => w.line(&format!("{writer}.write_i16({access})")),
            PrimitiveType::I32 => w.line(&format!("{writer}.write_i32({access})")),
            PrimitiveType::I64 => w.line(&format!("{writer}.write_i64({access})")),
            PrimitiveType::F32 => w.line(&format!("{writer}.write_f32({access})")),
            PrimitiveType::F64 => w.line(&format!("{writer}.write_f64({access})")),
            PrimitiveType::Fixed32 => w.line(&format!("{writer}.write_i32({access})")),
            PrimitiveType::Fixed64 => w.line(&format!("{writer}.write_i64({access})")),
            PrimitiveType::Void => {}
        },
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            w.line(&format!("{writer}.write_bits({access}, {bits})"));
        }
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => w.line(&format!("{writer}.write_string({access})")),
            SemanticType::Bytes => w.line(&format!("{writer}.write_bytes({access})")),
            SemanticType::Rgb => {
                w.line(&format!("{writer}.write_u8({access}[0])"));
                w.line(&format!("{writer}.write_u8({access}[1])"));
                w.line(&format!("{writer}.write_u8({access}[2])"));
            }
            SemanticType::Uuid => w.line(&format!("{writer}.write_raw_bytes({access}, 16)")),
            SemanticType::Timestamp => w.line(&format!("{writer}.write_i64({access})")),
            SemanticType::Hash => w.line(&format!("{writer}.write_raw_bytes({access}, 32)")),
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
                    w.line(&format!("{access}.encode_to({writer})"));
                }
                Some(TypeDef::Enum(_)) => {
                    w.line(&format!("{access}.encode_to({writer})"));
                }
                Some(TypeDef::Flags(_)) => {
                    w.line(&format!("{access}.encode_to({writer})"));
                }
                Some(TypeDef::Union(_)) => {
                    w.line(&format!("{access}.encode_to({writer})"));
                }
                Some(TypeDef::Newtype(_)) => {
                    w.line(&format!("{access}.encode_to({writer})"));
                }
                _ => {
                    w.line(&format!("# Unknown type: {type_name}"));
                }
            }
        }
        ResolvedType::Optional(inner) => {
            let value = local_name(access, "optional");
            w.line(&format!("{value} = {access}"));
            w.line(&format!("{writer}.write_bool({value} is not None)"));
            w.line(&format!("{writer}.flush_to_byte_boundary()"));
            w.open_block(&format!("if {value} is not None"));
            emit_write_type(w, &value, inner, registry, writer);
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            let item = local_name(access, "array_item");
            w.line(&format!("{writer}.write_leb128(len({access}))"));
            w.open_block(&format!("for {item} in {access}"));
            emit_write_type(w, &item, inner, registry, writer);
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            let map_k = local_name(access, "map_key");
            let map_v = local_name(access, "map_value");
            w.line(&format!("{writer}.write_leb128(len({access}))"));
            if matches!(k.as_ref(), ResolvedType::Semantic(SemanticType::String)) {
                // String keys have a canonical lexical order. Iterating a
                // caller-owned dict would otherwise make the wire depend on
                // insertion order.
                w.open_block(&format!("for {map_k} in sorted({access})"));
                w.line(&format!("{map_v} = {access}[{map_k}]"));
            } else {
                w.open_block(&format!("for {map_k}, {map_v} in {access}.items()"));
            }
            emit_write_type(w, &map_k, k, registry, writer);
            emit_write_type(w, &map_v, v, registry, writer);
            w.close_block();
        }
        ResolvedType::Set(inner) => {
            let item = local_name(access, "set_item");
            w.line(&format!("{writer}.write_leb128(len({access}))"));
            if matches!(inner.as_ref(), ResolvedType::Semantic(SemanticType::String)) {
                w.open_block(&format!("for {item} in sorted({access})"));
            } else {
                w.open_block(&format!("for {item} in {access}"));
            }
            emit_write_type(w, &item, inner, registry, writer);
            w.close_block();
        }
        ResolvedType::FixedArray(inner, _size) => {
            let item = local_name(access, "fixed_item");
            w.open_block(&format!("for {item} in {access}"));
            emit_write_type(w, &item, inner, registry, writer);
            w.close_block();
        }
        ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => {
            let item = local_name(access, "geometric_item");
            w.open_block(&format!("for {item} in {access}"));
            emit_write_type(w, &item, inner, registry, writer);
            w.close_block();
        }
        ResolvedType::Result(ok, err_ty) => {
            let value = local_name(access, "result");
            w.line(&format!("{value} = {access}"));
            w.open_block(&format!("if {value}[0] is True"));
            w.line(&format!("{writer}.write_bool(True)"));
            emit_write_type(w, &format!("{value}[1]"), ok, registry, writer);
            w.dedent();
            w.line("else:");
            w.indent();
            w.line(&format!("{writer}.write_bool(False)"));
            emit_write_type(w, &format!("{value}[1]"), err_ty, registry, writer);
            w.close_block();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// emit_read - read a value from BitReader
// ---------------------------------------------------------------------------

pub fn emit_read(
    w: &mut CodeWriter,
    target: &str,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    reader: &str,
) {
    match &enc.encoding {
        Encoding::Varint => {
            w.line(&format!("{target} = {reader}.read_leb128()"));
            return;
        }
        Encoding::ZigZag => {
            w.line(&format!("{target} = {reader}.read_zigzag()"));
            return;
        }
        Encoding::Delta(inner) => {
            let base_enc = FieldEncoding {
                encoding: *inner.clone(),
                limit: enc.limit,
            };
            emit_read(w, target, ty, &base_enc, registry, reader);
            return;
        }
        Encoding::Default => {}
        _ => {}
    }

    emit_read_type(w, target, ty, registry, reader);
}

fn emit_read_type(
    w: &mut CodeWriter,
    target: &str,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    reader: &str,
) {
    match ty {
        ResolvedType::Primitive(p) => {
            let read_fn = match p {
                PrimitiveType::Bool => "read_bool",
                PrimitiveType::U8 => "read_u8",
                PrimitiveType::U16 => "read_u16",
                PrimitiveType::U32 => "read_u32",
                PrimitiveType::U64 => "read_u64",
                PrimitiveType::I8 => "read_i8",
                PrimitiveType::I16 => "read_i16",
                PrimitiveType::I32 => "read_i32",
                PrimitiveType::I64 => "read_i64",
                PrimitiveType::F32 => "read_f32",
                PrimitiveType::F64 => "read_f64",
                PrimitiveType::Fixed32 => "read_i32",
                PrimitiveType::Fixed64 => "read_i64",
                PrimitiveType::Void => {
                    w.line(&format!("{target} = None"));
                    return;
                }
            };
            w.line(&format!("{target} = {reader}.{read_fn}()"));
        }
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            w.line(&format!("{target} = {reader}.read_bits({bits})"));
        }
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => {
                w.line(&format!("{target} = {reader}.read_string()"));
            }
            SemanticType::Bytes => {
                let length = local_name(target, "length");
                w.line(&format!("{length} = {reader}.read_leb128()"));
                w.line(&format!("{target} = {reader}.read_bytes({length})"));
            }
            SemanticType::Rgb => {
                let red = local_name(target, "red");
                let green = local_name(target, "green");
                let blue = local_name(target, "blue");
                w.line(&format!("{red} = {reader}.read_u8()"));
                w.line(&format!("{green} = {reader}.read_u8()"));
                w.line(&format!("{blue} = {reader}.read_u8()"));
                w.line(&format!("{target} = ({red}, {green}, {blue})"));
            }
            SemanticType::Uuid => {
                w.line(&format!("{target} = {reader}.read_bytes(16)"));
            }
            SemanticType::Timestamp => {
                w.line(&format!("{target} = {reader}.read_i64()"));
            }
            SemanticType::Hash => {
                w.line(&format!("{target} = {reader}.read_bytes(32)"));
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
                    w.line(&format!("{target} = {type_name}.decode_from({reader})"));
                }
                Some(TypeDef::Enum(_)) => {
                    w.line(&format!("{target} = {type_name}.decode_from({reader})"));
                }
                Some(TypeDef::Flags(_)) => {
                    w.line(&format!("{target} = {type_name}.decode_from({reader})"));
                }
                Some(TypeDef::Union(_)) => {
                    w.line(&format!("{target} = decode_{type_name}_from({reader})"));
                }
                Some(TypeDef::Newtype(_)) => {
                    w.line(&format!("{target} = {type_name}.decode_from({reader})"));
                }
                _ => {
                    w.line(&format!("# Unknown type: {type_name}"));
                }
            }
        }
        ResolvedType::Optional(inner) => {
            let present = local_name(target, "present");
            w.open_block("try");
            w.line(&format!("{present} = {reader}.read_bool()"));
            w.close_block();
            w.line("except DecodeError:");
            w.indent();
            w.line(&format!("{target} = None"));
            w.dedent();
            w.line("else:");
            w.indent();
            w.line(&format!("{reader}.flush_to_byte_boundary()"));
            w.open_block(&format!("if {present}"));
            emit_read_type(w, target, inner, registry, reader);
            w.dedent();
            w.line("else:");
            w.indent();
            w.line(&format!("{target} = None"));
            w.close_block();
            w.dedent();
        }
        ResolvedType::Array(inner) => {
            let length = local_name(target, "array_length");
            let items = local_name(target, "array_items");
            let item = local_name(target, "array_item");
            w.line(&format!("{length} = {reader}.read_leb128()"));
            let inner_py = py_type(inner, registry);
            w.line(&format!("{items}: list[{inner_py}] = []"));
            w.open_block(&format!("for _ in range({length})"));
            emit_read_type(w, &item, inner, registry, reader);
            w.line(&format!("{items}.append({item})"));
            w.close_block();
            w.line(&format!("{target} = {items}"));
        }
        ResolvedType::Set(inner) => {
            let length = local_name(target, "set_length");
            let items = local_name(target, "set_items");
            let item = local_name(target, "set_item");
            w.line(&format!("{length} = {reader}.read_leb128()"));
            let inner_py = py_type(inner, registry);
            w.line(&format!("{items}: set[{inner_py}] = set()"));
            w.open_block(&format!("for _ in range({length})"));
            emit_read_type(w, &item, inner, registry, reader);
            w.line(&format!("{items}.add({item})"));
            w.close_block();
            w.line(&format!("{target} = {items}"));
        }
        ResolvedType::FixedArray(inner, size) => {
            let inner_py = py_type(inner, registry);
            let items = local_name(target, "fixed_items");
            let item = local_name(target, "fixed_item");
            w.line(&format!("{items}: list[{inner_py}] = []"));
            w.open_block(&format!("for _ in range({size})"));
            emit_read_type(w, &item, inner, registry, reader);
            w.line(&format!("{items}.append({item})"));
            w.close_block();
            w.line(&format!("{target} = tuple({items})"));
        }
        ResolvedType::Map(k, v) => {
            let length = local_name(target, "map_length");
            let items = local_name(target, "map_items");
            let key = local_name(target, "map_key");
            let value = local_name(target, "map_value");
            w.line(&format!("{length} = {reader}.read_leb128()"));
            let k_py = py_type(k, registry);
            let v_py = py_type(v, registry);
            w.line(&format!("{items}: dict[{k_py}, {v_py}] = {{}}"));
            w.open_block(&format!("for _ in range({length})"));
            emit_read_type(w, &key, k, registry, reader);
            emit_read_type(w, &value, v, registry, reader);
            w.line(&format!("{items}[{key}] = {value}"));
            w.close_block();
            w.line(&format!("{target} = {items}"));
        }
        ResolvedType::Result(ok, err_ty) => {
            let is_ok = local_name(target, "result_is_ok");
            let ok_value = local_name(target, "result_ok");
            let err_value = local_name(target, "result_err");
            w.line(&format!("{is_ok} = {reader}.read_bool()"));
            w.open_block(&format!("if {is_ok}"));
            emit_read_type(w, &ok_value, ok, registry, reader);
            w.line(&format!("{target} = (True, {ok_value})"));
            w.dedent();
            w.line("else:");
            w.indent();
            emit_read_type(w, &err_value, err_ty, registry, reader);
            w.line(&format!("{target} = (False, {err_value})"));
            w.close_block();
        }
        ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => {
            let size = match ty {
                ResolvedType::Vec2(_) => 2,
                ResolvedType::Vec3(_) => 3,
                ResolvedType::Vec4(_) | ResolvedType::Quat(_) => 4,
                ResolvedType::Mat3(_) => 9,
                ResolvedType::Mat4(_) => 16,
                _ => unreachable!(),
            };
            let inner_py = py_type(inner, registry);
            let items = local_name(target, "geometric_items");
            let item = local_name(target, "geometric_item");
            w.line(&format!("{items}: list[{inner_py}] = []"));
            w.open_block(&format!("for _ in range({size})"));
            emit_read_type(w, &item, inner, registry, reader);
            w.line(&format!("{items}.append({item})"));
            w.close_block();
            w.line(&format!("{target} = tuple({items})"));
        }
        _ => {}
    }
}

/// Emit a typed local container initializer or an unannotated attribute
/// assignment. Python only permits annotations on simple names, and message
/// fields already carry their dataclass type annotations.
// ---------------------------------------------------------------------------
// emit_tombstone_read - read and discard (for backwards compatibility)
// ---------------------------------------------------------------------------

fn emit_tombstone_read(
    w: &mut CodeWriter,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    reader: &str,
) {
    match ty {
        ResolvedType::Primitive(p) => {
            let read_fn = match p {
                PrimitiveType::Bool => "read_bool",
                PrimitiveType::U8 => "read_u8",
                PrimitiveType::U16 => "read_u16",
                PrimitiveType::U32 => "read_u32",
                PrimitiveType::U64 => "read_u64",
                PrimitiveType::I8 => "read_i8",
                PrimitiveType::I16 => "read_i16",
                PrimitiveType::I32 => "read_i32",
                PrimitiveType::I64 => "read_i64",
                PrimitiveType::F32 => "read_f32",
                PrimitiveType::F64 => "read_f64",
                PrimitiveType::Fixed32 => "read_i32",
                PrimitiveType::Fixed64 => "read_i64",
                PrimitiveType::Void => return,
            };
            w.line(&format!("_ = {reader}.{read_fn}()"));
        }
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            w.line(&format!("_ = {reader}.read_bits({bits})"));
        }
        ResolvedType::Semantic(s) => {
            let read_expr = match s {
                SemanticType::String => format!("_ = {reader}.read_string()"),
                SemanticType::Bytes => {
                    format!("_ = {reader}.read_bytes({reader}.read_leb128())")
                }
                SemanticType::Rgb => {
                    w.line(&format!("_ = {reader}.read_u8()"));
                    w.line(&format!("_ = {reader}.read_u8()"));
                    w.line(&format!("_ = {reader}.read_u8()"));
                    return;
                }
                SemanticType::Uuid => format!("_ = {reader}.read_bytes(16)"),
                SemanticType::Timestamp => format!("_ = {reader}.read_i64()"),
                SemanticType::Hash => format!("_ = {reader}.read_bytes(32)"),
            };
            w.line(&read_expr);
        }
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
            if matches!(registry.get(*id), Some(TypeDef::Union(_))) {
                w.line(&format!("_ = decode_{type_name}_from({reader})"));
            } else {
                w.line(&format!("_ = {type_name}.decode_from({reader})"));
            }
        }
        ResolvedType::Optional(inner) => {
            w.line(&format!("_present = {reader}.read_bool()"));
            w.line(&format!("{reader}.flush_to_byte_boundary()"));
            w.open_block("if _present");
            emit_tombstone_read(w, inner, registry, reader);
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            w.line(&format!("_len = {reader}.read_leb128()"));
            w.open_block("for _ in range(_len)");
            emit_tombstone_read(w, inner, registry, reader);
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            w.line(&format!("_len = {reader}.read_leb128()"));
            w.open_block("for _ in range(_len)");
            emit_tombstone_read(w, k, registry, reader);
            emit_tombstone_read(w, v, registry, reader);
            w.close_block();
        }
        ResolvedType::Result(ok, err_ty) => {
            w.line(&format!("_is_ok = {reader}.read_bool()"));
            w.open_block("if _is_ok");
            emit_tombstone_read(w, ok, registry, reader);
            w.dedent();
            w.line("else:");
            w.indent();
            emit_tombstone_read(w, err_ty, registry, reader);
            w.close_block();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// emit_message - main message struct + encode/decode
// ---------------------------------------------------------------------------

pub fn emit_message(
    w: &mut CodeWriter,
    msg: &MessageDef,
    registry: &TypeRegistry,
    functions: &[PortableFunction],
) {
    let name = msg.name.as_str();

    // Dataclass definition with methods
    w.line("@dataclass");
    w.open_block(&format!("class {name}"));
    for field in &msg.fields {
        let py_ty = py_type(&field.resolved_type, registry);
        let field_name = py_ident(field.name.as_str());
        w.line(&format!("{field_name}: {py_ty}"));
    }
    w.line("unknown: bytes = b\"\"");
    for function in functions {
        w.blank();
        crate::fn_body::emit_function(w, function, registry);
    }
    w.blank();

    // encode method
    w.open_block("def encode(self) -> bytes");
    w.line("w = _BitWriter()");
    w.line("self.encode_to(w)");
    w.line("return w.finish()");
    w.close_block();
    w.blank();

    w.open_block("def encode_to(self, w: _BitWriter) -> None");
    for field in &msg.fields {
        let field_name = py_ident(field.name.as_str());
        let access = format!("self.{field_name}");
        // Validate constraint before encoding
        if let Some(constraint) = &field.constraint {
            emit_constraint_validation_py(w, constraint, &access, field.name.as_str());
        }
        emit_write(
            w,
            &access,
            &field.resolved_type,
            &field.encoding,
            registry,
            "w",
        );
    }
    w.line("w.flush_to_byte_boundary()");
    w.open_block("if self.unknown");
    w.line("w.write_raw_bytes(self.unknown, len(self.unknown))");
    w.close_block();
    w.close_block();
    w.blank();

    // decode static method
    w.line("@staticmethod");
    w.open_block(&format!("def decode(data: bytes) -> {name}"));
    w.line("r = _BitReader(data)");
    w.line(&format!("return {name}.decode_from(r)"));
    w.close_block();
    w.blank();

    w.line("@staticmethod");
    w.open_block(&format!("def decode_from(r: _BitReader) -> {name}"));
    w.line(&format!("m = {name}.__new__({name})"));

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

    for (_ord, action) in actions.iter() {
        match action {
            DecodeAction::Field(field) => {
                let field_name = py_ident(field.name.as_str());
                let target = format!("m.{field_name}");
                emit_read(
                    w,
                    &target,
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    "r",
                );
                // Validate constraint after decoding
                if let Some(constraint) = &field.constraint {
                    emit_constraint_validation_py(w, constraint, &target, field.name.as_str());
                }
            }
            DecodeAction::Tombstone(tombstone) => {
                if let Some(ref ty) = tombstone.original_type {
                    w.line(&format!("# discard @removed ordinal {}", tombstone.ordinal));
                    emit_tombstone_read(w, ty, registry, "r");
                }
            }
        }
    }
    w.line("r.flush_to_byte_boundary()");
    w.line("m.unknown = b\"\"");
    w.line("return m");
    w.close_block();
    w.blank();
    w.close_block();
    w.blank();
}

// ---------------------------------------------------------------------------
// emit_config - config type (struct only, no codec)
// ---------------------------------------------------------------------------

pub fn emit_config(w: &mut CodeWriter, cfg: &ConfigDef, registry: &TypeRegistry) {
    let name = cfg.name.as_str();

    w.line("@dataclass");
    w.open_block(&format!("class {name}"));
    for field in &cfg.fields {
        let py_ty = py_type(&field.resolved_type, registry);
        let field_name = py_ident(field.name.as_str());
        w.line(&format!("{field_name}: {py_ty}"));
    }
    w.close_block();
    w.blank();
}
