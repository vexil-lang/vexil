use vexil_lang::ir::{FlagsDef, TypeRegistry};

use crate::emit::CodeWriter;

/// Emit a flags type: number type alias + named constants + encode/decode.
pub fn emit_flags(w: &mut CodeWriter, flags: &FlagsDef, _registry: &TypeRegistry) {
    let name = flags.name.as_str();

    // Type alias
    w.line(&format!("export type {name} = number;"));

    // Const object with bit values
    w.open_block(&format!("export const {name} ="));
    for bit_def in &flags.bits {
        let bit = bit_def.bit;
        w.line(&format!("{}: {},", bit_def.name, 1u64 << bit));
    }
    w.dedent();
    w.line("} as const;");
    w.blank();

    // Encode function
    w.open_block(&format!(
        "export function encode{name}(v: {name}, w: BitWriter): void"
    ));
    match flags.wire_bytes {
        1 => w.line("w.writeU8(v);"),
        2 => w.line("w.writeU16(v);"),
        4 => w.line("w.writeU32(v);"),
        _ => w.line("w.writeU64(BigInt(v));"),
    }
    w.close_block();
    w.blank();

    // Decode function
    w.open_block(&format!(
        "export function decode{name}(r: BitReader): {name}"
    ));
    match flags.wire_bytes {
        1 => w.line("return r.readU8();"),
        2 => w.line("return r.readU16();"),
        4 => w.line("return r.readU32();"),
        _ => w.line("return Number(r.readU64());"),
    }
    w.close_block();
    w.blank();
}
