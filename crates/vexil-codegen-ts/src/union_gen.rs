use vexil_lang::ir::{TypeRegistry, UnionDef};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::ts_type;

/// Emit a complete union: discriminated union type + encode/decode.
///
/// Wire format: discriminant (LEB128) + payload byte length (LEB128) + payload bytes.
pub fn emit_union(w: &mut CodeWriter, un: &UnionDef, registry: &TypeRegistry) {
    let name = un.name.as_str();
    let non_exhaustive = un.annotations.non_exhaustive;

    // Emit individual variant interfaces
    for variant in &un.variants {
        let vname = variant.name.as_str();
        let iface_name = format!("{name}_{vname}");
        w.open_block(&format!("export interface {iface_name}"));
        w.line(&format!("tag: '{}';", vname));
        for field in &variant.fields {
            let field_ts = ts_type(&field.resolved_type, registry);
            w.line(&format!("{}: {};", field.name, field_ts));
        }
        w.close_block();
        w.blank();
    }

    // Non-exhaustive unknown variant
    if non_exhaustive {
        w.open_block(&format!("export interface {name}_Unknown"));
        w.line("tag: '__unknown';");
        w.line("discriminant: number;");
        w.line("data: Uint8Array;");
        w.close_block();
        w.blank();
    }

    // Union type
    let variant_types: Vec<String> = un
        .variants
        .iter()
        .map(|v| format!("{name}_{}", v.name))
        .collect();
    let mut all_types = variant_types;
    if non_exhaustive {
        all_types.push(format!("{name}_Unknown"));
    }
    w.line(&format!("export type {name} = {};", all_types.join(" | ")));
    w.blank();

    // Encode function
    w.open_block(&format!(
        "export function encode{name}(v: {name}, w: BitWriter): void"
    ));
    w.line("w.flushToByteBoundary();");
    w.open_block("switch (v.tag)");

    for variant in &un.variants {
        let vname = variant.name.as_str();
        let ordinal = variant.ordinal;

        w.open_block(&format!("case '{vname}':"));
        w.line(&format!("w.writeLeb128(BigInt({ordinal}));"));

        if variant.fields.is_empty() {
            w.line("w.writeLeb128(0n);");
        } else {
            w.line("const payloadW = new BitWriter();");
            for field in &variant.fields {
                let access = format!("v.{}", field.name);
                emit_write(
                    w,
                    &access,
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    "payloadW",
                );
            }
            w.line("payloadW.flushToByteBoundary();");
            w.line("const payload = payloadW.finish();");
            w.line("w.writeLeb128(BigInt(payload.length));");
            w.line("w.writeRawBytes(payload);");
        }
        w.line("break;");
        w.close_block();
    }

    if non_exhaustive {
        w.open_block("case '__unknown':");
        w.line("w.writeLeb128(BigInt(v.discriminant));");
        w.line("w.writeLeb128(BigInt(v.data.length));");
        w.line("w.writeRawBytes(v.data);");
        w.line("break;");
        w.close_block();
    }

    w.close_block(); // switch
    w.close_block(); // function
    w.blank();

    // Decode function
    w.open_block(&format!(
        "export function decode{name}(r: BitReader): {name}"
    ));
    w.line("r.flushToByteBoundary();");
    w.line("const disc = Number(r.readLeb128());");
    w.line("const len = Number(r.readLeb128());");
    w.open_block("switch (disc)");

    for variant in &un.variants {
        let vname = variant.name.as_str();
        let ordinal = variant.ordinal;

        w.open_block(&format!("case {ordinal}:"));
        if variant.fields.is_empty() {
            w.line("r.readRawBytes(len);");
            w.line(&format!("return {{ tag: '{vname}' as const }};"));
        } else {
            w.line("const payloadBytes = r.readRawBytes(len);");
            w.line("const pr = new BitReader(payloadBytes);");
            for field in &variant.fields {
                emit_read(
                    w,
                    field.name.as_str(),
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    "pr",
                );
            }
            w.line("pr.flushToByteBoundary();");
            let field_assigns: Vec<String> =
                variant.fields.iter().map(|f| f.name.to_string()).collect();
            w.line(&format!(
                "return {{ tag: '{vname}' as const, {} }};",
                field_assigns.join(", ")
            ));
        }
        w.close_block();
    }

    if non_exhaustive {
        w.open_block("default:");
        w.line("const data = r.readRawBytes(len);");
        w.line("return { tag: '__unknown' as const, discriminant: disc, data };");
        w.close_block();
    } else {
        w.open_block("default:");
        w.line(&format!(
            "throw new Error(`Unknown {name} discriminant: ${{disc}}`);"
        ));
        w.close_block();
    }

    w.close_block(); // switch
    w.close_block(); // function
    w.blank();
}
