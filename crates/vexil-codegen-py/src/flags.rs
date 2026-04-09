use vexil_lang::ir::{FlagsDef, TypeRegistry};

use crate::emit::CodeWriter;

/// Emit a complete Python flags class.
pub fn emit_flags(w: &mut CodeWriter, flags: &FlagsDef, _registry: &TypeRegistry) {
    let name = flags.name.as_str();
    let wire_bytes = flags.wire_bytes;

    // Class definition — subclass of int
    w.open_block(&format!("class {name}(int)"));
    for bit_def in &flags.bits {
        let value = 1u64 << bit_def.bit;
        w.line(&format!("{} = {}", to_upper_snake(&bit_def.name), value));
    }
    w.blank();

    // encode method
    w.line("def encode(self) -> bytes:");
    w.indent();
    w.line("w = _BitWriter()");
    match wire_bytes {
        1 => w.line("w.write_u8(int(self))"),
        2 => w.line("w.write_u16(int(self))"),
        4 => w.line("w.write_u32(int(self))"),
        _ => w.line("w.write_u64(int(self))"),
    }
    w.line("return w.finish()");
    w.close_block();
    w.blank();

    // decode static method
    w.line("@staticmethod");
    w.open_block("def decode(data: bytes)");
    w.line("r = _BitReader(data)");
    match wire_bytes {
        1 => w.line("v = r.read_u8()"),
        2 => w.line("v = r.read_u16()"),
        4 => w.line("v = r.read_u32()"),
        _ => w.line("v = r.read_u64()"),
    }
    w.line(&format!("return {name}(v)"));
    w.close_block();
    w.blank();

    // has_flag helper
    w.open_block("def has(self, flag: int)");
    w.line("return bool(int(self) & flag)");
    w.close_block();
    w.blank();

    // __repr__ for readability
    w.line("def __repr__(self) -> str:");
    w.indent();
    w.line(&format!("return f\"{name}({{int(self)}})\""));
    w.close_block();

    w.close_block();
    w.blank();
}

/// Convert PascalCase name to UPPER_SNAKE for Python constants.
fn to_upper_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.extend(ch.to_uppercase());
    }
    result
}
