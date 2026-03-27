use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{
    ConfigDef, Encoding, FieldEncoding, MessageDef, ResolvedType, TypeDef, TypeRegistry,
};

use crate::emit::CodeWriter;
use crate::types::ts_type;

// ---------------------------------------------------------------------------
// Byte-alignment helper
// ---------------------------------------------------------------------------

/// Returns true if the type is byte-aligned (i.e., not sub-byte).
pub fn is_byte_aligned(ty: &ResolvedType, registry: &TypeRegistry) -> bool {
    match ty {
        ResolvedType::Primitive(PrimitiveType::Bool) => false,
        ResolvedType::SubByte(_) => false,
        ResolvedType::Named(id) => {
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
// Primitive bits helper
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
// emit_write
// ---------------------------------------------------------------------------

/// Emit code to write a value to a BitWriter.
///
/// `access` is the TypeScript expression for the value.
/// `writer` is the variable name of the BitWriter (e.g. "w" or "payloadW").
pub fn emit_write(
    w: &mut CodeWriter,
    access: &str,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    writer: &str,
) {
    // Check non-default encoding first
    match &enc.encoding {
        Encoding::Varint => {
            w.line(&format!("{writer}.writeLeb128(BigInt({access}));"));
            return;
        }
        Encoding::ZigZag => {
            let type_bits = match ty {
                ResolvedType::Primitive(p) => primitive_bits(p),
                _ => 64,
            };
            w.line(&format!(
                "{writer}.writeZigZag(BigInt({access}), {type_bits});",
            ));
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
        Encoding::Default => {} // fall through to type dispatch
        _ => {}                 // non_exhaustive guard
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
            PrimitiveType::Bool => w.line(&format!("{writer}.writeBool({access});")),
            PrimitiveType::U8 => w.line(&format!("{writer}.writeU8({access});")),
            PrimitiveType::U16 => w.line(&format!("{writer}.writeU16({access});")),
            PrimitiveType::U32 => w.line(&format!("{writer}.writeU32({access});")),
            PrimitiveType::U64 => w.line(&format!("{writer}.writeU64({access});")),
            PrimitiveType::I8 => w.line(&format!("{writer}.writeI8({access});")),
            PrimitiveType::I16 => w.line(&format!("{writer}.writeI16({access});")),
            PrimitiveType::I32 => w.line(&format!("{writer}.writeI32({access});")),
            PrimitiveType::I64 => w.line(&format!("{writer}.writeI64({access});")),
            PrimitiveType::F32 => w.line(&format!("{writer}.writeF32({access});")),
            PrimitiveType::F64 => w.line(&format!("{writer}.writeF64({access});")),
            PrimitiveType::Void => {} // 0 bits — nothing to write
        },
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            w.line(&format!("{writer}.writeBits({access}, {bits});"));
        }
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => w.line(&format!("{writer}.writeString({access});")),
            SemanticType::Bytes => w.line(&format!("{writer}.writeBytes({access});")),
            SemanticType::Rgb => {
                w.line(&format!("{writer}.writeU8({access}[0]);"));
                w.line(&format!("{writer}.writeU8({access}[1]);"));
                w.line(&format!("{writer}.writeU8({access}[2]);"));
            }
            SemanticType::Uuid => w.line(&format!("{writer}.writeRawBytes({access});")),
            SemanticType::Timestamp => w.line(&format!("{writer}.writeI64({access});")),
            SemanticType::Hash => w.line(&format!("{writer}.writeRawBytes({access});")),
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
            w.line(&format!("{writer}.enterNested();"));
            w.line(&format!("encode{type_name}({access}, {writer});"));
            w.line(&format!("{writer}.leaveNested();"));
        }
        ResolvedType::Optional(inner) => {
            // Presence bit
            w.line(&format!("{writer}.writeBool({access} !== null);"));
            if is_byte_aligned(inner, registry) {
                w.line(&format!("{writer}.flushToByteBoundary();"));
            }
            w.open_block(&format!("if ({access} !== null)"));
            emit_write_type(w, access, inner, registry, writer);
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            w.line(&format!("{writer}.writeLeb128(BigInt({access}.length));"));
            w.open_block(&format!("for (const item of {access})"));
            emit_write_type(w, "item", inner, registry, writer);
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            w.line(&format!("{writer}.writeLeb128(BigInt({access}.size));"));
            w.open_block(&format!("for (const [mapK, mapV] of {access})"));
            emit_write_type(w, "mapK", k, registry, writer);
            emit_write_type(w, "mapV", v, registry, writer);
            w.close_block();
        }
        ResolvedType::Result(ok, err) => {
            w.open_block(&format!("if ('ok' in {access})"));
            w.line(&format!("{writer}.writeBool(true);"));
            emit_write_type(w, &format!("{access}.ok"), ok, registry, writer);
            w.dedent();
            w.line("} else {");
            w.indent();
            w.line(&format!("{writer}.writeBool(false);"));
            emit_write_type(w, &format!("{access}.err"), err, registry, writer);
            w.close_block();
        }
        _ => {} // non_exhaustive guard
    }
}

// ---------------------------------------------------------------------------
// emit_read
// ---------------------------------------------------------------------------

/// Emit code to read a value from a BitReader and bind to `var_name`.
///
/// `reader` is the variable name of the BitReader (e.g. "r" or "pr").
pub fn emit_read(
    w: &mut CodeWriter,
    var_name: &str,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    reader: &str,
) {
    match &enc.encoding {
        Encoding::Varint => {
            let cast = varint_cast(ty);
            w.line(&format!("const {var_name} = {cast}{reader}.readLeb128());"));
            return;
        }
        Encoding::ZigZag => {
            let type_bits = match ty {
                ResolvedType::Primitive(p) => primitive_bits(p),
                _ => 64,
            };
            let cast = zigzag_cast(ty);
            w.line(&format!(
                "const {var_name} = {cast}{reader}.readZigZag({type_bits}));",
            ));
            return;
        }
        Encoding::Delta(inner) => {
            let base_enc = FieldEncoding {
                encoding: *inner.clone(),
                limit: enc.limit,
            };
            emit_read(w, var_name, ty, &base_enc, registry, reader);
            return;
        }
        Encoding::Default => {}
        _ => {} // non_exhaustive guard
    }

    emit_read_type(w, var_name, ty, registry, reader);
}

fn varint_cast(ty: &ResolvedType) -> &'static str {
    match ty {
        ResolvedType::Primitive(PrimitiveType::U64 | PrimitiveType::I64) => "(",
        _ => "Number(",
    }
}

fn zigzag_cast(ty: &ResolvedType) -> &'static str {
    match ty {
        ResolvedType::Primitive(PrimitiveType::I64) => "(",
        _ => "Number(",
    }
}

fn emit_read_type(
    w: &mut CodeWriter,
    var_name: &str,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    reader: &str,
) {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::Bool => {
                w.line(&format!("const {var_name} = {reader}.readBool();"));
            }
            PrimitiveType::U8 => {
                w.line(&format!("const {var_name} = {reader}.readU8();"));
            }
            PrimitiveType::U16 => {
                w.line(&format!("const {var_name} = {reader}.readU16();"));
            }
            PrimitiveType::U32 => {
                w.line(&format!("const {var_name} = {reader}.readU32();"));
            }
            PrimitiveType::U64 => {
                w.line(&format!("const {var_name} = {reader}.readU64();"));
            }
            PrimitiveType::I8 => {
                w.line(&format!("const {var_name} = {reader}.readI8();"));
            }
            PrimitiveType::I16 => {
                w.line(&format!("const {var_name} = {reader}.readI16();"));
            }
            PrimitiveType::I32 => {
                w.line(&format!("const {var_name} = {reader}.readI32();"));
            }
            PrimitiveType::I64 => {
                w.line(&format!("const {var_name} = {reader}.readI64();"));
            }
            PrimitiveType::F32 => {
                w.line(&format!("const {var_name} = {reader}.readF32();"));
            }
            PrimitiveType::F64 => {
                w.line(&format!("const {var_name} = {reader}.readF64();"));
            }
            PrimitiveType::Void => {
                w.line(&format!("const {var_name} = undefined;"));
            }
        },
        ResolvedType::SubByte(s) => {
            let bits = s.bits;
            if s.signed {
                // Sign-extend sub-byte value
                let shift = 8 - bits;
                w.line(&format!(
                    "const {var_name} = ({reader}.readBits({bits}) << {shift}) >> {shift};",
                ));
            } else {
                w.line(&format!("const {var_name} = {reader}.readBits({bits});",));
            }
        }
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => {
                w.line(&format!("const {var_name} = {reader}.readString();"));
            }
            SemanticType::Bytes => {
                w.line(&format!("const {var_name} = {reader}.readBytes();"));
            }
            SemanticType::Rgb => {
                w.line(&format!("const {var_name}_0 = {reader}.readU8();"));
                w.line(&format!("const {var_name}_1 = {reader}.readU8();"));
                w.line(&format!("const {var_name}_2 = {reader}.readU8();"));
                w.line(&format!(
                    "const {var_name}: [number, number, number] = [{var_name}_0, {var_name}_1, {var_name}_2];"
                ));
            }
            SemanticType::Uuid => {
                w.line(&format!("const {var_name} = {reader}.readRawBytes(16);"));
            }
            SemanticType::Timestamp => {
                w.line(&format!("const {var_name} = {reader}.readI64();"));
            }
            SemanticType::Hash => {
                w.line(&format!("const {var_name} = {reader}.readRawBytes(32);"));
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
            w.line(&format!("{reader}.enterNested();"));
            w.line(&format!("const {var_name} = decode{type_name}({reader});"));
            w.line(&format!("{reader}.leaveNested();"));
        }
        ResolvedType::Optional(inner) => {
            w.line(&format!("const {var_name}_present = {reader}.readBool();"));
            if is_byte_aligned(inner, registry) {
                w.line(&format!("{reader}.flushToByteBoundary();"));
            }
            let inner_ts = ts_type(inner, registry);
            w.line(&format!("let {var_name}: {inner_ts} | null;",));
            w.open_block(&format!("if ({var_name}_present)"));
            emit_read_type(w, &format!("{var_name}_inner"), inner, registry, reader);
            w.line(&format!("{var_name} = {var_name}_inner;"));
            w.dedent();
            w.line("} else {");
            w.indent();
            w.line(&format!("{var_name} = null;"));
            w.close_block();
        }
        ResolvedType::Array(inner) => {
            w.line(&format!(
                "const {var_name}_len = Number({reader}.readLeb128());"
            ));
            let inner_ts = ts_type(inner, registry);
            w.line(&format!("const {var_name}: {inner_ts}[] = [];"));
            w.open_block(&format!("for (let i = 0; i < {var_name}_len; i++)"));
            emit_read_type(w, &format!("{var_name}_item"), inner, registry, reader);
            w.line(&format!("{var_name}.push({var_name}_item);"));
            w.close_block();
        }
        ResolvedType::Map(k, v) => {
            w.line(&format!(
                "const {var_name}_len = Number({reader}.readLeb128());"
            ));
            let k_ts = ts_type(k, registry);
            let v_ts = ts_type(v, registry);
            w.line(&format!("const {var_name} = new Map<{k_ts}, {v_ts}>();"));
            w.open_block(&format!("for (let i = 0; i < {var_name}_len; i++)"));
            emit_read_type(w, &format!("{var_name}_k"), k, registry, reader);
            emit_read_type(w, &format!("{var_name}_v"), v, registry, reader);
            w.line(&format!("{var_name}.set({var_name}_k, {var_name}_v);"));
            w.close_block();
        }
        ResolvedType::Result(ok, err) => {
            let ok_ts = ts_type(ok, registry);
            let err_ts = ts_type(err, registry);
            w.line(&format!("const {var_name}_isOk = {reader}.readBool();"));
            w.line(&format!(
                "let {var_name}: {{ ok: {ok_ts} }} | {{ err: {err_ts} }};"
            ));
            w.open_block(&format!("if ({var_name}_isOk)"));
            emit_read_type(w, &format!("{var_name}_ok"), ok, registry, reader);
            w.line(&format!("{var_name} = {{ ok: {var_name}_ok }};"));
            w.dedent();
            w.line("} else {");
            w.indent();
            emit_read_type(w, &format!("{var_name}_err"), err, registry, reader);
            w.line(&format!("{var_name} = {{ err: {var_name}_err }};"));
            w.close_block();
        }
        _ => {} // non_exhaustive guard
    }
}

// ---------------------------------------------------------------------------
// emit_message
// ---------------------------------------------------------------------------

/// Emit a complete message: interface + encode function + decode function.
pub fn emit_message(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry) {
    let name = msg.name.as_str();

    // Interface
    w.open_block(&format!("export interface {name}"));
    for field in &msg.fields {
        let field_ts = ts_type(&field.resolved_type, registry);
        w.line(&format!("{}: {};", field.name, field_ts));
    }
    w.close_block();
    w.blank();

    // Encode function
    w.open_block(&format!(
        "export function encode{name}(v: {name}, w: BitWriter): void"
    ));
    for field in &msg.fields {
        let access = format!("v.{}", field.name);
        emit_write(
            w,
            &access,
            &field.resolved_type,
            &field.encoding,
            registry,
            "w",
        );
    }
    w.line("w.flushToByteBoundary();");
    w.close_block();
    w.blank();

    // Decode function
    w.open_block(&format!(
        "export function decode{name}(r: BitReader): {name}"
    ));
    for field in &msg.fields {
        emit_read(
            w,
            field.name.as_str(),
            &field.resolved_type,
            &field.encoding,
            registry,
            "r",
        );
    }
    w.line("r.flushToByteBoundary();");
    let field_names: Vec<&str> = msg.fields.iter().map(|f| f.name.as_str()).collect();
    w.line(&format!("return {{ {} }};", field_names.join(", ")));
    w.close_block();
    w.blank();
}

// ---------------------------------------------------------------------------
// emit_config
// ---------------------------------------------------------------------------

/// Emit a config type: interface only (no codec).
pub fn emit_config(w: &mut CodeWriter, cfg: &ConfigDef, registry: &TypeRegistry) {
    let name = cfg.name.as_str();

    w.open_block(&format!("export interface {name}"));
    for field in &cfg.fields {
        let field_ts = ts_type(&field.resolved_type, registry);
        w.line(&format!("{}: {};", field.name, field_ts));
    }
    w.close_block();
    w.blank();
}
