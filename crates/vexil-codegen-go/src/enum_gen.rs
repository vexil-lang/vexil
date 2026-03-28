use vexil_lang::ir::{EnumDef, TypeRegistry};

use crate::emit::CodeWriter;

/// Emit a complete enum: type declaration + const block + Pack/Unpack methods.
pub fn emit_enum(w: &mut CodeWriter, en: &EnumDef, _registry: &TypeRegistry) {
    let name = en.name.as_str();
    let wire_bits = en.wire_bits;

    // Type declaration
    w.line(&format!("type {name} int"));
    w.blank();

    // Constants
    w.line("const (");
    w.indent();
    for variant in &en.variants {
        w.line(&format!(
            "{name}{} {name} = {}",
            variant.name, variant.ordinal
        ));
    }
    w.dedent();
    w.line(")");
    w.blank();

    // Pack method (value receiver — enum is a small int)
    w.open_block(&format!("func (s {name}) Pack(w *vexil.BitWriter) error"));
    w.line(&format!("w.WriteBits(uint64(s), {wire_bits})"));
    w.line("return nil");
    w.close_block();
    w.blank();

    // Unpack method (pointer receiver — modifies the value)
    w.open_block(&format!(
        "func (s *{name}) Unpack(r *vexil.BitReader) error"
    ));
    w.line(&format!("v, err := r.ReadBits({wire_bits})"));
    w.open_block("if err != nil");
    w.line("return err");
    w.close_block();
    w.line(&format!("*s = {name}(v)"));
    w.line("return nil");
    w.close_block();
    w.blank();
}
