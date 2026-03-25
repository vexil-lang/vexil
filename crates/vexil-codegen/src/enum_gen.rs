use vexil_lang::ir::{EnumDef, TypeRegistry};

use crate::annotations::{emit_tombstones, emit_type_annotations};
use crate::emit::CodeWriter;

/// Emit a complete enum type with `Pack` and `Unpack` implementations.
///
/// # Non-exhaustive enums
/// When `annotations.non_exhaustive` is true, the enum gains an `Unknown(u64)` catch-all
/// variant.  Because `Unknown` carries data, `#[repr(u64)]` cannot be used (Rust only
/// allows repr discriminants on fieldless enums).  In that case we emit a plain enum
/// and implement the Pack/Unpack trait by hand.
///
/// # Exhaustive enums
/// When the enum is exhaustive every variant is fieldless, so we use `#[repr(u64)]` to
/// let the compiler verify the discriminant assignments.
pub fn emit_enum(w: &mut CodeWriter, en: &EnumDef, _registry: &TypeRegistry) {
    let name = en.name.as_str();
    let non_exhaustive = en.annotations.non_exhaustive;
    let wire_bits = en.wire_bits;

    // ── Tombstone block ─────────────────────────────────────────────────────
    emit_tombstones(w, name, &en.tombstones);

    // ── Type-level annotations (doc, since, deprecated, non_exhaustive) ─────
    emit_type_annotations(w, &en.annotations);

    // ── Derive + repr ────────────────────────────────────────────────────────
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]");
    if !non_exhaustive {
        // repr(u64) is only valid for fieldless enums
        w.line("#[repr(u64)]");
    }

    // ── Enum body ────────────────────────────────────────────────────────────
    w.open_block(&format!("pub enum {name}"));
    for variant in &en.variants {
        let ordinal = variant.ordinal;
        // Field-level annotations for the variant
        for doc in &variant.annotations.doc {
            w.line(&format!("/// {doc}"));
        }
        if let Some(ref dep) = variant.annotations.deprecated {
            match &dep.since {
                Some(since) => w.line(&format!(
                    "#[deprecated(since = \"{since}\", note = \"{}\")]",
                    dep.reason
                )),
                None => w.line(&format!("#[deprecated(note = \"{}\")]", dep.reason)),
            }
        }
        if non_exhaustive {
            // No repr discriminant allowed when there are tuple variants
            w.line(&format!("{},", variant.name));
        } else {
            w.line(&format!("{} = {ordinal}_u64,", variant.name));
        }
    }
    if non_exhaustive {
        // Catch-all for unknown ordinals received from the wire
        w.line("Unknown(u64),");
    }
    w.close_block();
    w.blank();

    // ── Pack impl ────────────────────────────────────────────────────────────
    w.open_block(&format!("impl vexil_runtime::Pack for {name}"));
    w.open_block(
        "fn pack(&self, w: &mut vexil_runtime::BitWriter) -> Result<(), vexil_runtime::EncodeError>",
    );
    // Build the match arms inline then emit `let disc: u64 = match self { ... };`
    // We emit it as a block statement with a trailing semicolon on the closing brace.
    w.line("let disc: u64 = match self {");
    w.indent();
    for variant in &en.variants {
        let ordinal = variant.ordinal;
        w.line(&format!("Self::{} => {ordinal}_u64,", variant.name));
    }
    if non_exhaustive {
        w.line("Self::Unknown(v) => *v,");
    }
    w.dedent();
    w.line("};");
    w.line(&format!("w.write_bits(disc, {wire_bits}_u8);"));
    w.line("Ok(())");
    w.close_block();
    w.close_block();
    w.blank();

    // ── Unpack impl ──────────────────────────────────────────────────────────
    w.open_block(&format!("impl vexil_runtime::Unpack for {name}"));
    w.open_block(
        "fn unpack(r: &mut vexil_runtime::BitReader<'_>) -> Result<Self, vexil_runtime::DecodeError>",
    );
    w.line(&format!("let disc = r.read_bits({wire_bits}_u8)?;"));
    w.open_block("match disc");
    for variant in &en.variants {
        let ordinal = variant.ordinal;
        w.line(&format!("{ordinal}_u64 => Ok(Self::{}),", variant.name));
    }
    if non_exhaustive {
        w.line("other => Ok(Self::Unknown(other)),");
    } else {
        w.line(&format!(
            "_ => Err(vexil_runtime::DecodeError::UnknownEnumVariant {{ type_name: \"{name}\", value: disc }}),"
        ));
    }
    w.close_block();
    w.close_block();
    w.close_block();
    w.blank();
}
