use vexil_lang::ir::{TypeRegistry, UnionDef};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::py_type;

/// Emit a complete Python union: base class + variant classes + encode/decode.
pub fn emit_union(w: &mut CodeWriter, un: &UnionDef, registry: &TypeRegistry) {
    let name = un.name.as_str();

    // Base class
    w.open_block(&format!("class {name}"));
    w.blank();

    // encode method (dispatches to variant encode)
    w.open_block("def encode(self) -> bytes");
    w.line("return self._encode_variant()");
    w.close_block();
    w.blank();

    // _encode_variant — overridden by subclasses
    w.open_block("def _encode_variant(self) -> bytes");
    w.line("raise NotImplementedError");
    w.close_block();

    w.close_block();
    w.blank();

    // Emit individual variant classes
    for variant in &un.variants {
        let vname = variant.name.as_str();
        let class_name = format!("{name}{vname}");

        w.open_block(&format!("class {class_name}({name})"));

        // __init__
        if variant.fields.is_empty() {
            w.open_block("def __init__(self)");
            w.line("pass");
            w.close_block();
        } else {
            let params: Vec<String> = variant
                .fields
                .iter()
                .map(|f| {
                    let py_ty = py_type(&f.resolved_type, registry);
                    format!("{}: {py_ty}", f.name)
                })
                .collect();
            w.open_block(&format!("def __init__(self, {})", params.join(", ")));
            for field in &variant.fields {
                w.line(&format!("self.{} = {}", field.name, field.name));
            }
            w.close_block();
        }
        w.blank();

        // _encode_variant
        let ordinal = variant.ordinal;
        w.open_block("def _encode_variant(self) -> bytes");
        w.line("w = _BitWriter()");
        w.line(&format!("w.write_leb128({ordinal})"));

        if variant.fields.is_empty() {
            w.line("w.write_leb128(0)");
        } else {
            w.line("pw = _BitWriter()");
            for field in &variant.fields {
                let access = format!("self.{}", field.name);
                emit_write(
                    w,
                    &access,
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    "pw",
                );
            }
            w.line("pw.flush_to_byte_boundary()");
            w.line("payload = pw.finish()");
            w.line("w.write_leb128(len(payload))");
            w.line("w.write_raw_bytes(payload)");
        }
        w.line("return w.finish()");
        w.close_block();
        w.blank();

        w.close_block();
        w.blank();
    }

    // Module-level decode function
    w.open_block(&format!("def decode_{name}(data: bytes) -> {name}"));
    w.line("r = _BitReader(data)");
    w.line("r.flush_to_byte_boundary()");
    w.line("disc = r.read_leb128()");
    w.line("length = r.read_leb128()");

    for variant in &un.variants {
        let vname = variant.name.as_str();
        let class_name = format!("{name}{vname}");
        let ordinal = variant.ordinal;

        if ordinal == 0 {
            w.open_block(&format!("if disc == {ordinal}"));
        } else {
            w.open_block(&format!("elif disc == {ordinal}"));
        }

        if variant.fields.is_empty() {
            w.line(&format!("return {class_name}()"));
        } else {
            w.line("_payload = r.read_raw_bytes(length)");
            w.line("pr = _BitReader(_payload)");
            // Read each field into locals
            for field in &variant.fields {
                let py_ty = py_type(&field.resolved_type, registry);
                w.line(&format!(
                    "{}: {py_ty} = None  # type: ignore[assignment]",
                    field.name
                ));
                emit_read(
                    w,
                    &field.name,
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    "pr",
                );
            }
            let field_names: Vec<&str> = variant.fields.iter().map(|f| f.name.as_str()).collect();
            w.line(&format!("return {class_name}({})", field_names.join(", ")));
        }
        w.close_block();
    }

    // default case
    w.open_block("else");
    w.line(&format!(
        "raise ValueError(f\"unknown {name} discriminant: {{disc}}\")"
    ));
    w.close_block();

    w.close_block();
    w.blank();
}
