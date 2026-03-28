use vexil_lang::ir::{FlagsDef, TypeRegistry};

use crate::emit::CodeWriter;

/// Emit a flags type: type declaration + const block + Pack/Unpack methods.
pub fn emit_flags(w: &mut CodeWriter, flags: &FlagsDef, _registry: &TypeRegistry) {
    let name = flags.name.as_str();
    let wire_bytes = flags.wire_bytes;

    // Type declaration based on wire width
    let backing = match wire_bytes {
        1 => "uint8",
        2 => "uint16",
        4 => "uint32",
        _ => "uint64",
    };
    w.line(&format!("type {name} {backing}"));
    w.blank();

    // Constants
    w.line("const (");
    w.indent();
    for bit_def in &flags.bits {
        let bit = bit_def.bit;
        let value = 1u64 << bit;
        w.line(&format!("{name}{} {name} = {value}", bit_def.name));
    }
    w.dedent();
    w.line(")");
    w.blank();

    // Pack method (value receiver)
    w.open_block(&format!("func (f {name}) Pack(w *vexil.BitWriter) error"));
    match wire_bytes {
        1 => w.line("w.WriteU8(uint8(f))"),
        2 => w.line("w.WriteU16(uint16(f))"),
        4 => w.line("w.WriteU32(uint32(f))"),
        _ => w.line("w.WriteU64(uint64(f))"),
    }
    w.line("return nil");
    w.close_block();
    w.blank();

    // Unpack method (pointer receiver)
    w.open_block(&format!(
        "func (f *{name}) Unpack(r *vexil.BitReader) error"
    ));
    match wire_bytes {
        1 => {
            w.line("v, err := r.ReadU8()");
            w.open_block("if err != nil");
            w.line("return err");
            w.close_block();
            w.line(&format!("*f = {name}(v)"));
        }
        2 => {
            w.line("v, err := r.ReadU16()");
            w.open_block("if err != nil");
            w.line("return err");
            w.close_block();
            w.line(&format!("*f = {name}(v)"));
        }
        4 => {
            w.line("v, err := r.ReadU32()");
            w.open_block("if err != nil");
            w.line("return err");
            w.close_block();
            w.line(&format!("*f = {name}(v)"));
        }
        _ => {
            w.line("v, err := r.ReadU64()");
            w.open_block("if err != nil");
            w.line("return err");
            w.close_block();
            w.line(&format!("*f = {name}(v)"));
        }
    }
    w.line("return nil");
    w.close_block();
    w.blank();
}
