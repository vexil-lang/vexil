use std::collections::HashSet;

use vexil_lang::ir::{TypeId, TypeRegistry, UnionDef};

use crate::annotations::{emit_field_annotations, emit_tombstones, emit_type_annotations};
use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::rust_type;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns true if a type needs explicit dereference when accessed through a
/// by-ref pattern binding (e.g. union variant destructuring in `match &self`).
/// Primitives and sub-byte types are Copy and need `*field` to get the value;
/// reference types (String, Vec, etc.) work via auto-ref coercion.
fn needs_deref(ty: &vexil_lang::ir::ResolvedType) -> bool {
    use vexil_lang::ir::ResolvedType;
    matches!(ty, ResolvedType::Primitive(_) | ResolvedType::SubByte(_))
}

/// Collect field-encoding code from `emit_write` or `emit_read` into a
/// scratch `CodeWriter`, then redirect the reader/writer variable name by
/// string substitution before emitting into the target writer.
///
/// `emit_write` always emits `w.`-prefixed calls; `emit_read` emits `r.`-prefixed calls.
/// We redirect them to `payload_w.` / `pr.` for the union payload sub-buffer.
fn emit_write_to_payload(
    w: &mut CodeWriter,
    field_name: &str,
    ty: &vexil_lang::ir::ResolvedType,
    enc: &vexil_lang::ir::FieldEncoding,
    registry: &TypeRegistry,
) {
    // Union variant fields are destructured by-ref (match on &self).
    // Primitives/sub-byte need deref; String/Bytes/Array/Map/Named work via auto-ref.
    let access = if needs_deref(ty) {
        format!("*{field_name}")
    } else {
        field_name.to_string()
    };
    let mut scratch = CodeWriter::new();
    emit_write(&mut scratch, &access, ty, enc, registry, field_name);
    let code = scratch.finish();
    // Replace `w.` → `payload_w.` for method calls, and `(w)` → `(&mut payload_w)` for
    // argument passing (e.g. Named types emit `.pack(w)?;`)
    //
    // Union variant fields are already borrowed (`&T`) from the match destructuring,
    // so `emit_write`'s `&access` patterns produce double-references (`&&Vec<T>`).
    // Fix by stripping the extra `&` for iteration/match contexts.
    let redirected = code
        .replace("w.", "payload_w.")
        .replace("(w)", "(&mut payload_w)")
        .replace(&format!("in &{access}"), &format!("in {access}"))
        .replace(&format!("match &{access}"), &format!("match {access}"));
    for line in redirected.lines() {
        if !line.trim().is_empty() {
            w.line(line.trim());
        }
    }
}

fn emit_read_from_payload(
    w: &mut CodeWriter,
    var_name: &str,
    ty: &vexil_lang::ir::ResolvedType,
    enc: &vexil_lang::ir::FieldEncoding,
    registry: &TypeRegistry,
) {
    let mut scratch = CodeWriter::new();
    emit_read(&mut scratch, var_name, ty, enc, registry, var_name);
    let code = scratch.finish();
    // Replace `r.` → `pr.` for method calls, and `(r)` → `(&mut pr)` for
    // argument passing (e.g. Named types emit `Unpack::unpack(r)?;`)
    let redirected = code.replace("r.", "pr.").replace("(r)", "(&mut pr)");
    for line in redirected.lines() {
        if !line.trim().is_empty() {
            w.line(line.trim());
        }
    }
}

// ---------------------------------------------------------------------------
// emit_union
// ---------------------------------------------------------------------------

/// Emit a complete union enum with `Pack` and `Unpack` implementations.
///
/// Wire format (§4.4): discriminant (LEB128) + payload byte length (LEB128) + payload bytes.
///
/// Each variant is emitted as a struct variant.  Empty variants still write
/// discriminant + 0-length payload on the wire.
///
/// When `annotations.non_exhaustive` is true an extra `Unknown { discriminant: u64, data: Vec<u8> }`
/// catch-all variant is appended.
pub fn emit_union(
    w: &mut CodeWriter,
    un: &UnionDef,
    registry: &TypeRegistry,
    needs_box: &HashSet<(TypeId, usize)>,
    type_id: TypeId,
) {
    let name = un.name.as_str();
    let non_exhaustive = un.annotations.non_exhaustive;

    // ── Tombstone block ─────────────────────────────────────────────────────
    emit_tombstones(w, name, &un.tombstones);

    // ── Type-level annotations (doc, since, deprecated, non_exhaustive) ─────
    emit_type_annotations(w, &un.annotations);
    w.line("#[derive(Debug, Clone, PartialEq)]");

    // ── Enum body ────────────────────────────────────────────────────────────
    w.open_block(&format!("pub enum {name}"));
    for variant in &un.variants {
        emit_tombstones(
            w,
            &format!("{}_{}", name, variant.name),
            &variant.tombstones,
        );
        emit_field_annotations(w, &variant.annotations);

        let fields_str: String = variant
            .fields
            .iter()
            .enumerate()
            .map(|(fi, field)| {
                let field_rust_type = rust_type(
                    &field.resolved_type,
                    registry,
                    needs_box,
                    Some((type_id, fi)),
                );
                format!("{}: {}", field.name, field_rust_type)
            })
            .collect::<Vec<_>>()
            .join(", ");

        if fields_str.is_empty() {
            w.line(&format!("{} {{}},", variant.name));
        } else {
            w.line(&format!("{} {{ {} }},", variant.name, fields_str));
        }
    }
    if non_exhaustive {
        w.line("Unknown { discriminant: u64, data: Vec<u8> },");
    }
    w.close_block();
    w.blank();

    // ── Pack impl ────────────────────────────────────────────────────────────
    w.open_block(&format!("impl vexil_runtime::Pack for {name}"));
    w.open_block(
        "fn pack(&self, w: &mut vexil_runtime::BitWriter) -> Result<(), vexil_runtime::EncodeError>",
    );
    w.open_block("match self");

    for variant in &un.variants {
        let ordinal = variant.ordinal;
        let vname = variant.name.as_str();

        if variant.fields.is_empty() {
            // Empty variant: write discriminant + 0-length payload
            w.open_block(&format!("Self::{vname} {{}} =>"));
            w.line(&format!("w.write_leb128({ordinal}_u64);"));
            w.line("w.write_leb128(0_u64);");
            w.close_block();
        } else {
            let bindings = variant
                .fields
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            w.open_block(&format!("Self::{vname} {{ {bindings} }} =>"));
            w.line(&format!("w.write_leb128({ordinal}_u64);"));
            w.line("let mut payload_w = vexil_runtime::BitWriter::new();");
            for field in &variant.fields {
                emit_write_to_payload(
                    w,
                    field.name.as_str(),
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                );
            }
            w.line("payload_w.flush_to_byte_boundary();");
            w.line("let payload = payload_w.finish();");
            w.line("w.write_leb128(payload.len() as u64);");
            w.line("w.write_raw_bytes(&payload);");
            w.close_block();
        }
    }

    if non_exhaustive {
        w.open_block("Self::Unknown { discriminant, data } =>");
        w.line("w.write_leb128(*discriminant);");
        w.line("w.write_leb128(data.len() as u64);");
        w.line("w.write_raw_bytes(data);");
        w.close_block();
    }

    w.close_block(); // end match
    w.line("Ok(())");
    w.close_block(); // end fn
    w.close_block(); // end impl
    w.blank();

    // ── Unpack impl ──────────────────────────────────────────────────────────
    w.open_block(&format!("impl vexil_runtime::Unpack for {name}"));
    w.open_block(
        "fn unpack(r: &mut vexil_runtime::BitReader<'_>) -> Result<Self, vexil_runtime::DecodeError>",
    );
    w.line("r.flush_to_byte_boundary();");
    w.line("let disc = r.read_leb128(10_u8)?;");
    w.line("let len = r.read_leb128(10_u8)? as usize;");
    w.open_block("match disc");

    for variant in &un.variants {
        let ordinal = variant.ordinal;
        let vname = variant.name.as_str();

        if variant.fields.is_empty() {
            w.open_block(&format!("{ordinal}_u64 =>"));
            w.line("let _skip = r.read_raw_bytes(len)?;");
            w.line(&format!("Ok(Self::{vname} {{}})"));
            w.close_block();
        } else {
            w.open_block(&format!("{ordinal}_u64 =>"));
            w.line("let payload = r.read_raw_bytes(len)?;");
            w.line("let mut pr = vexil_runtime::BitReader::new(&payload);");
            for field in &variant.fields {
                emit_read_from_payload(
                    w,
                    field.name.as_str(),
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                );
            }
            w.line("pr.flush_to_byte_boundary();");
            let field_names = variant
                .fields
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            w.line(&format!("Ok(Self::{vname} {{ {field_names} }})"));
            w.close_block();
        }
    }

    if non_exhaustive {
        w.open_block("other =>");
        w.line("let data = r.read_raw_bytes(len)?;");
        w.line("Ok(Self::Unknown { discriminant: other, data })");
        w.close_block();
    } else {
        w.open_block("_ =>");
        w.line("let _skip = r.read_raw_bytes(len)?;");
        w.line(&format!(
            "Err(vexil_runtime::DecodeError::UnknownUnionVariant {{ type_name: \"{name}\", discriminant: disc }})"
        ));
        w.close_block();
    }

    w.close_block(); // end match
    w.close_block(); // end fn
    w.close_block(); // end impl
    w.blank();
}
