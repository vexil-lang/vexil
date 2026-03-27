use vexil_lang::ir::{FieldEncoding, NewtypeDef, TypeRegistry};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::ts_type;

/// Emit a newtype: type alias + encode/decode functions that delegate to the inner type.
pub fn emit_newtype(w: &mut CodeWriter, nt: &NewtypeDef, registry: &TypeRegistry) {
    let name = nt.name.as_str();
    let inner_ts = ts_type(&nt.inner_type, registry);
    w.line(&format!("export type {name} = {inner_ts};"));
    w.blank();

    let default_enc = FieldEncoding::default_encoding();

    // Encode function
    w.open_block(&format!(
        "export function encode{name}(v: {name}, w: BitWriter): void"
    ));
    emit_write(w, "v", &nt.inner_type, &default_enc, registry, "w");
    w.close_block();
    w.blank();

    // Decode function
    w.open_block(&format!(
        "export function decode{name}(r: BitReader): {name}"
    ));
    emit_read(w, "value", &nt.inner_type, &default_enc, registry, "r");
    w.line("return value;");
    w.close_block();
    w.blank();
}
