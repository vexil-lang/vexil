use vexil_lang::ir::{EnumDef, TypeRegistry};

use crate::emit::CodeWriter;

/// Emit a complete enum: string literal union + const object + encode/decode.
pub fn emit_enum(w: &mut CodeWriter, en: &EnumDef, _registry: &TypeRegistry) {
    let name = en.name.as_str();
    let wire_bits = en.wire_bits;
    let non_exhaustive = en.annotations.non_exhaustive;

    // Type: string literal union
    if non_exhaustive {
        // Non-exhaustive enums include an unknown string fallback
        let literals: Vec<String> = en
            .variants
            .iter()
            .map(|v| format!("'{}'", v.name))
            .collect();
        let mut all = literals;
        all.push("string".to_string());
        w.line(&format!("export type {name} = {};", all.join(" | ")));
    } else {
        let literals: Vec<String> = en
            .variants
            .iter()
            .map(|v| format!("'{}'", v.name))
            .collect();
        w.line(&format!("export type {name} = {};", literals.join(" | ")));
    }

    // Const object for convenient access
    w.open_block(&format!("export const {name} ="));
    for variant in &en.variants {
        w.line(&format!("{}: '{}' as const,", variant.name, variant.name));
    }
    w.dedent();
    w.line("} as const;");
    w.blank();

    // Encode function
    w.open_block(&format!(
        "export function encode{name}(v: {name}, w: BitWriter): void"
    ));
    // Build ordinal map
    w.line("let disc: number;");
    w.open_block("switch (v)");
    for variant in &en.variants {
        w.line(&format!(
            "case '{}': disc = {}; break;",
            variant.name, variant.ordinal
        ));
    }
    w.line(&format!(
        "default: throw new Error(`Unknown {name} variant: ${{v}}`);",
    ));
    w.close_block();
    w.line(&format!("w.writeBits(disc, {wire_bits});"));
    w.close_block();
    w.blank();

    // Decode function
    w.open_block(&format!(
        "export function decode{name}(r: BitReader): {name}"
    ));
    w.line(&format!("const disc = r.readBits({wire_bits});"));
    w.open_block("switch (disc)");
    for variant in &en.variants {
        w.line(&format!(
            "case {}: return '{}';",
            variant.ordinal, variant.name
        ));
    }
    if non_exhaustive {
        w.line("default: return `Unknown(${disc})`;");
    } else {
        w.line(&format!(
            "default: throw new Error(`Unknown {name} discriminant: ${{disc}}`);",
        ));
    }
    w.close_block();
    w.close_block();
    w.blank();
}
