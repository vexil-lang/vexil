use vexil_lang::ir::{FlagsDef, TypeRegistry};

use crate::annotations::{emit_tombstones, emit_type_annotations};
use crate::emit::CodeWriter;

/// Convert a `SmolStr` name to UPPER_SNAKE_CASE.
fn to_upper_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut prev_lower = false;
    for ch in name.chars() {
        if ch.is_uppercase() && prev_lower {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
        prev_lower = ch.is_lowercase();
    }
    out
}

/// Emit a complete flags newtype struct with bit constants, utility methods,
/// bitwise operator impls, and `Pack`/`Unpack` implementations.
pub fn emit_flags(w: &mut CodeWriter, flags: &FlagsDef, _registry: &TypeRegistry) {
    let name = flags.name.as_str();

    // ── Tombstone block ─────────────────────────────────────────────────────
    emit_tombstones(w, name, &flags.tombstones);

    // ── Type-level annotations (doc, since, deprecated, non_exhaustive) ─────
    emit_type_annotations(w, &flags.annotations);

    // ── Derive ───────────────────────────────────────────────────────────────
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]");

    // ── Struct definition ────────────────────────────────────────────────────
    w.line(&format!("pub struct {name}(pub u64);"));
    w.blank();

    // ── impl block ───────────────────────────────────────────────────────────
    w.open_block(&format!("impl {name}"));

    // Bit constants
    for bit_def in &flags.bits {
        let const_name = to_upper_snake(bit_def.name.as_str());
        let bit = bit_def.bit;
        // Field-level annotations
        for doc in &bit_def.annotations.doc {
            w.line(&format!("/// {doc}"));
        }
        if let Some(ref dep) = bit_def.annotations.deprecated {
            match &dep.since {
                Some(since) => w.line(&format!(
                    "#[deprecated(since = \"{since}\", note = \"{}\")]",
                    dep.reason
                )),
                None => w.line(&format!("#[deprecated(note = \"{}\")]", dep.reason)),
            }
        }
        w.line(&format!(
            "pub const {const_name}: Self = Self(1_u64 << {bit}_u32);"
        ));
    }

    w.blank();

    // Utility methods
    w.line("pub const fn contains(self, other: Self) -> bool { self.0 & other.0 == other.0 }");
    w.line("pub const fn is_empty(self) -> bool { self.0 == 0 }");
    w.line("pub const fn empty() -> Self { Self(0) }");

    w.close_block();
    w.blank();

    // ── BitOr ────────────────────────────────────────────────────────────────
    w.open_block(&format!("impl std::ops::BitOr for {name}"));
    w.line("type Output = Self;");
    w.open_block("fn bitor(self, rhs: Self) -> Self");
    w.line("Self(self.0 | rhs.0)");
    w.close_block();
    w.close_block();
    w.blank();

    // ── BitAnd ───────────────────────────────────────────────────────────────
    w.open_block(&format!("impl std::ops::BitAnd for {name}"));
    w.line("type Output = Self;");
    w.open_block("fn bitand(self, rhs: Self) -> Self");
    w.line("Self(self.0 & rhs.0)");
    w.close_block();
    w.close_block();
    w.blank();

    // ── Not ──────────────────────────────────────────────────────────────────
    w.open_block(&format!("impl std::ops::Not for {name}"));
    w.line("type Output = Self;");
    w.open_block("fn not(self) -> Self");
    w.line("Self(!self.0)");
    w.close_block();
    w.close_block();
    w.blank();

    // ── Pack impl ────────────────────────────────────────────────────────────
    w.open_block(&format!("impl vexil_runtime::Pack for {name}"));
    w.open_block(
        "fn pack(&self, w: &mut vexil_runtime::BitWriter) -> Result<(), vexil_runtime::EncodeError>",
    );
    match flags.wire_bytes {
        1 => w.line("w.write_u8(self.0 as u8);"),
        2 => w.line("w.write_u16(self.0 as u16);"),
        4 => w.line("w.write_u32(self.0 as u32);"),
        _ => w.line("w.write_u64(self.0);"),
    }
    w.line("Ok(())");
    w.close_block();
    w.close_block();
    w.blank();

    // ── Unpack impl ──────────────────────────────────────────────────────────
    w.open_block(&format!("impl vexil_runtime::Unpack for {name}"));
    w.open_block(
        "fn unpack(r: &mut vexil_runtime::BitReader<'_>) -> Result<Self, vexil_runtime::DecodeError>",
    );
    match flags.wire_bytes {
        1 => w.line("let raw = r.read_u8()? as u64;"),
        2 => w.line("let raw = r.read_u16()? as u64;"),
        4 => w.line("let raw = r.read_u32()? as u64;"),
        _ => w.line("let raw = r.read_u64()?;"),
    }
    w.line("Ok(Self(raw))");
    w.close_block();
    w.close_block();
    w.blank();
}
