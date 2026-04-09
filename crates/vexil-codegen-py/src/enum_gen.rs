use vexil_lang::ir::{EnumDef, TypeRegistry};

use crate::emit::CodeWriter;

/// Emit a complete Python enum class.
pub fn emit_enum(w: &mut CodeWriter, en: &EnumDef, _registry: &TypeRegistry) {
    let name = en.name.as_str();

    // Class definition — subclass of int
    w.open_block(&format!("class {name}(int)"));
    for variant in &en.variants {
        w.line(&format!(
            "{} = {}",
            to_upper_snake(&variant.name),
            variant.ordinal
        ));
    }
    w.blank();

    // encode method
    w.line("def encode(self) -> bytes:");
    w.indent();
    w.line(&format!(
        "return _BitWriter().write_bits(int(self), {}).finish()",
        en.wire_bits
    ));
    w.close_block();
    w.blank();

    // decode static method
    w.line("@staticmethod");
    w.open_block("def decode(data: bytes)");
    w.line("r = _BitReader(data)");
    w.line(&format!("v = r.read_bits({})", en.wire_bits));
    w.line(&format!("return {name}(v)"));
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
