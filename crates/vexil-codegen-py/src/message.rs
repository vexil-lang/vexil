use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{
    CmpOp, ConfigDef, ConstraintOperand, Encoding, FieldConstraint, FieldEncoding, MessageDef,
    ResolvedType, TombstoneDef, TypeDef, TypeRegistry,
};

use crate::emit::CodeWriter;
use crate::types::py_type;

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
                    w.line(&format!("{writer}.write_message({access})"));
                }
                Some(TypeDef::Enum(_)) => {
                    w.line(&format!("{access}.encode_to({writer})"));
                }
                Some(TypeDef::Flags(_)) => {
                    w.line(&format!("{access}.encode_to({writer})"));
                }
                Some(TypeDef::Union(_)) => {
                    w.line(&format!("{writer}.extend({access}.encode())"));
                }
                Some(TypeDef::Newtype(_)) => {
                    w.line(&format!("{writer}.extend({access}.encode())"));
                }
                _ => {
                    w.line(&format!("# Unknown type: {type_name}"));
                }
            }
        }
        ResolvedType::Optional(inner) => {
            w.line(&format!("{writer}.write_bool({access} is not None)"));
            w.line(&format!("{writer}.flush_to_byte_boundary()"));
            w.open_block(&format!("if {access} is not None"));
            match inner.as_ref() {
                ResolvedType::Named(_) => {
                    emit_write_type(w, access, inner, registry, writer);
                }
                _ => {
                    emit_write_type(w, access, inner, registry, writer);
                }
            }
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            w.line(&format!("{writer}.write_leb128(len({access}))"));
            w.open_block("for item in {access}");
            emit_write_type(w, "item", inner, registry, writer);
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            w.line(&format!("{writer}.write_leb128(len({access}))"));
            w.open_block(&format!("for map_k, map_v in {access}.items()"));
            emit_write_type(w, "map_k", k, registry, writer);
            emit_write_type(w, "map_v", v, registry, writer);
            w.close_block();
        }
        ResolvedType::Set(inner) => {
            w.line(&format!("{writer}.write_leb128(len({access}))"));
            w.open_block(&format!("for item in {access}"));
            emit_write_type(w, "item", inner, registry, writer);
            w.close_block();
        }
        ResolvedType::FixedArray(inner, _size) => {
            w.open_block(&format!("for item in {access}"));
            emit_write_type(w, "item", inner, registry, writer);
            w.close_block();
        }
        ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => {
            w.open_block(&format!("for item in {access}"));
            emit_write_type(w, "item", inner, registry, writer);
            w.close_block();
        }
        ResolvedType::Result(ok, err_ty) => {
            w.open_block(&format!("if {access}[0]"));
            w.line(&format!("{writer}.write_bool(True)"));
            emit_write_type(w, &format!("{access}[1]"), ok, registry, writer);
            w.dedent();
            w.line("else:");
            w.indent();
            w.line(&format!("{writer}.write_bool(False)"));
            emit_write_type(w, &format!("{access}[1]"), err_ty, registry, writer);
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
                w.line(&format!("{target} = {reader}.read_bytes()"));
            }
            SemanticType::Rgb => {
                w.line(&format!("r = {reader}.read_u8()"));
                w.line(&format!("g = {reader}.read_u8()"));
                w.line(&format!("b = {reader}.read_u8()"));
                w.line(&format!("{target} = (r, g, b)"));
            }
            SemanticType::Uuid => {
                w.line(&format!("{target} = {reader}.read_raw_bytes(16)"));
            }
            SemanticType::Timestamp => {
                w.line(&format!("{target} = {reader}.read_i64()"));
            }
            SemanticType::Hash => {
                w.line(&format!("{target} = {reader}.read_raw_bytes(32)"));
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
                    w.line(&format!("_payload = {reader}.read_bytes()"));
                    w.line(&format!("{target} = decode_{type_name}(_payload)"));
                }
                Some(TypeDef::Newtype(_)) => {
                    w.line(&format!("_payload = {reader}.read_bytes()"));
                    w.line(&format!("{target} = {type_name}.decode(_payload)"));
                }
                _ => {
                    w.line(&format!("# Unknown type: {type_name}"));
                }
            }
        }
        ResolvedType::Optional(inner) => {
            w.line(&format!("present = {reader}.read_bool()"));
            w.line(&format!("{reader}.flush_to_byte_boundary()"));
            w.open_block("if present");
            let inner_py = py_type(inner, registry);
            w.line(&format!(
                "{target}: {inner_py} = None  # type: ignore[assignment]"
            ));
            emit_read_type(w, target, inner, registry, reader);
            w.dedent();
            w.line("else:");
            w.indent();
            w.line(&format!("{target} = None"));
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            w.line(&format!("arr_len = {reader}.read_leb128()"));
            let inner_py = py_type(inner, registry);
            w.line(&format!("{target}: list[{inner_py}] = []"));
            w.open_block("for _ in range(arr_len)");
            w.line(&format!(
                "_item: {inner_py} = None  # type: ignore[assignment]"
            ));
            emit_read_type(w, "_item", inner, registry, reader);
            w.line(&format!("{target}.append(_item)"));
            w.close_block();
        }
        ResolvedType::Set(inner) => {
            w.line(&format!("set_len = {reader}.read_leb128()"));
            let inner_py = py_type(inner, registry);
            w.line(&format!("{target}: set[{inner_py}] = set()"));
            w.open_block("for _ in range(set_len)");
            w.line(&format!(
                "_item: {inner_py} = None  # type: ignore[assignment]"
            ));
            emit_read_type(w, "_item", inner, registry, reader);
            w.line(&format!("{target}.add(_item)"));
            w.close_block();
        }
        ResolvedType::FixedArray(inner, size) => {
            let inner_py = py_type(inner, registry);
            w.line(&format!("{target}: tuple[{inner_py}, ...] = ("));
            w.indent();
            for i in 0..*size {
                let _ = i;
                w.line(&format!("{reader}.read_value(),"));
            }
            w.dedent();
            w.line(")");
        }
        ResolvedType::Map(k, v) => {
            w.line(&format!("map_len = {reader}.read_leb128()"));
            let k_py = py_type(k, registry);
            let v_py = py_type(v, registry);
            w.line(&format!("{target}: dict[{k_py}, {v_py}] = {{}}"));
            w.open_block("for _ in range(map_len)");
            w.line(&format!("_k: {k_py} = None  # type: ignore[assignment]"));
            w.line(&format!("_v: {v_py} = None  # type: ignore[assignment]"));
            emit_read_type(w, "_k", k, registry, reader);
            emit_read_type(w, "_v", v, registry, reader);
            w.line(&format!("{target}[_k] = _v"));
            w.close_block();
        }
        ResolvedType::Result(ok, err_ty) => {
            w.line(&format!("is_ok = {reader}.read_bool()"));
            let ok_py = py_type(ok, registry);
            let err_py = py_type(err_ty, registry);
            w.open_block("if is_ok");
            w.line(&format!(
                "_ok_val: {ok_py} = None  # type: ignore[assignment]"
            ));
            emit_read_type(w, "_ok_val", ok, registry, reader);
            w.line(&format!("{target} = (True, _ok_val)"));
            w.dedent();
            w.line("else:");
            w.indent();
            w.line(&format!(
                "_err_val: {err_py} = None  # type: ignore[assignment]"
            ));
            emit_read_type(w, "_err_val", err_ty, registry, reader);
            w.line(&format!("{target} = (False, _err_val)"));
            w.close_block();
        }
        _ => {}
    }
}

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
                SemanticType::Bytes => format!("_ = {reader}.read_bytes()"),
                SemanticType::Rgb => {
                    w.line(&format!("_ = {reader}.read_u8()"));
                    w.line(&format!("_ = {reader}.read_u8()"));
                    w.line(&format!("_ = {reader}.read_u8()"));
                    return;
                }
                SemanticType::Uuid => format!("_ = {reader}.read_raw_bytes(16)"),
                SemanticType::Timestamp => format!("_ = {reader}.read_i64()"),
                SemanticType::Hash => format!("_ = {reader}.read_raw_bytes(32)"),
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
            w.line(&format!("_tmp = {type_name}.__new__({type_name})"));
            w.line(&format!("_tmp.decode_from({reader})"));
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

pub fn emit_message(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry) {
    let name = msg.name.as_str();

    // Dataclass definition with methods
    w.line("@dataclass");
    w.open_block(&format!("class {name}"));
    for field in &msg.fields {
        let py_ty = py_type(&field.resolved_type, registry);
        let field_name = &field.name;
        w.line(&format!("{field_name}: {py_ty}"));
    }
    w.line("unknown: bytes = b\"\"");
    w.blank();

    // encode method
    w.open_block("def encode(self) -> bytes");
    w.line("w = _BitWriter()");
    for field in &msg.fields {
        let field_name = &field.name;
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
    w.line("return w.finish()");
    w.close_block();
    w.blank();

    // decode static method
    w.line("@staticmethod");
    w.open_block("def decode(data: bytes)");
    w.line("r = _BitReader(data)");
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
                let field_name = &field.name;
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
        let field_name = &field.name;
        w.line(&format!("{field_name}: {py_ty}"));
    }
    w.close_block();
    w.blank();
}
