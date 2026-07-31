use vexil_lang::ir::{TypeRegistry, UnionDef};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::{py_ident, py_type};

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
                    format!("{}: {py_ty}", py_ident(f.name.as_str()))
                })
                .collect();
            w.open_block(&format!("def __init__(self, {})", params.join(", ")));
            for field in &variant.fields {
                let ident = py_ident(field.name.as_str());
                w.line(&format!("self.{ident} = {ident}"));
            }
            w.close_block();
        }
        w.blank();

        // _encode_variant
        let ordinal = variant.ordinal;
        w.open_block("def _encode_variant(self) -> bytes");
        w.line("_vexil_writer = _BitWriter()");
        w.line(&format!("_vexil_writer.write_leb128({ordinal})"));

        if variant.fields.is_empty() {
            w.line("_vexil_writer.write_leb128(0)");
        } else {
            w.line("_vexil_payload_writer = _BitWriter()");
            for field in &variant.fields {
                let access = format!("self.{}", py_ident(field.name.as_str()));
                emit_write(
                    w,
                    &access,
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    "_vexil_payload_writer",
                );
            }
            w.line("_vexil_payload_writer.flush_to_byte_boundary()");
            w.line("_vexil_payload = _vexil_payload_writer.finish()");
            w.line("_vexil_writer.write_leb128(len(_vexil_payload))");
            w.line("_vexil_writer.write_raw_bytes(_vexil_payload, len(_vexil_payload))");
        }
        w.line("return _vexil_writer.finish()");
        w.close_block();
        w.blank();

        w.close_block();
        w.blank();
    }

    // Reader-level decode lets messages consume one union value without
    // guessing its size. The byte-array convenience wrapper follows below.
    w.open_block(&format!(
        "def decode_{name}_from(_vexil_reader: _BitReader) -> {name}"
    ));
    w.line("_vexil_reader.flush_to_byte_boundary()");
    w.line("_vexil_discriminant = _vexil_reader.read_leb128()");
    w.line("_vexil_length = _vexil_reader.read_leb128()");

    for variant in &un.variants {
        let vname = variant.name.as_str();
        let class_name = format!("{name}{vname}");
        let ordinal = variant.ordinal;

        if ordinal == 0 {
            w.open_block(&format!("if _vexil_discriminant == {ordinal}"));
        } else {
            w.open_block(&format!("elif _vexil_discriminant == {ordinal}"));
        }

        if variant.fields.is_empty() {
            w.line(&format!("return {class_name}()"));
        } else {
            w.line("_vexil_payload = _vexil_reader.read_bytes(_vexil_length)");
            w.line("_vexil_payload_reader = _BitReader(_vexil_payload)");
            // Read each field into locals
            for field in &variant.fields {
                let py_ty = py_type(&field.resolved_type, registry);
                let ident = format!("_vexil_field_{}", field.ordinal);
                w.line(&format!(
                    "{ident}: {py_ty} = None  # type: ignore[assignment]"
                ));
                emit_read(
                    w,
                    &ident,
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    "_vexil_payload_reader",
                );
            }
            let field_names: Vec<String> = variant
                .fields
                .iter()
                .map(|field| format!("_vexil_field_{}", field.ordinal))
                .collect();
            w.line(&format!("return {class_name}({})", field_names.join(", ")));
        }
        w.close_block();
    }

    // default case
    w.open_block("else");
    w.line(&format!(
        "raise ValueError(f\"unknown {name} discriminant: {{_vexil_discriminant}}\")"
    ));
    w.close_block();

    w.close_block();
    w.blank();

    w.open_block(&format!("def decode_{name}(data: bytes) -> {name}"));
    w.line("_vexil_reader = _BitReader(data)");
    w.line(&format!("return decode_{name}_from(_vexil_reader)"));
    w.close_block();
    w.blank();
}
