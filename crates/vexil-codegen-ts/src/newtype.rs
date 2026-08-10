use vexil_lang::ir::{FieldEncoding, NewtypeDef, TypeRegistry};

use crate::emit::CodeWriter;
use crate::message::{emit_read_with_symbols, emit_write_with_symbols};
use crate::types::{ts_type_with_symbols, TypeScriptSymbols};

/// Emit a newtype: type alias + encode/decode functions that delegate to the inner type.
pub fn emit_newtype(w: &mut CodeWriter, nt: &NewtypeDef, registry: &TypeRegistry) {
    emit_newtype_with_symbols(w, nt, registry, &TypeScriptSymbols::default());
}

pub(crate) fn emit_newtype_with_symbols(
    w: &mut CodeWriter,
    nt: &NewtypeDef,
    registry: &TypeRegistry,
    symbols: &TypeScriptSymbols,
) {
    let name = nt.name.as_str();
    let inner_ts = ts_type_with_symbols(&nt.inner_type, registry, symbols);
    w.line(&format!("export type {name} = {inner_ts};"));
    w.blank();

    let default_enc = FieldEncoding::default_encoding();

    // Encode function
    w.open_block(&format!(
        "export function encode{name}(v: {name}, w: BitWriter): void"
    ));
    emit_write_with_symbols(w, "v", &nt.inner_type, &default_enc, registry, "w", symbols);
    w.close_block();
    w.blank();

    // Decode function
    w.open_block(&format!(
        "export function decode{name}(r: BitReader): {name}"
    ));
    emit_read_with_symbols(
        w,
        "value",
        &nt.inner_type,
        &default_enc,
        registry,
        "r",
        symbols,
    );
    w.line("return value;");
    w.close_block();
    w.blank();
}
