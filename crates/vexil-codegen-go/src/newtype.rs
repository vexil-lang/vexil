use vexil_lang::ir::{FieldEncoding, NewtypeDef, TypeRegistry};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::go_type;

/// Emit a newtype: type definition + Pack/Unpack standalone functions.
pub fn emit_newtype(w: &mut CodeWriter, nt: &NewtypeDef, registry: &TypeRegistry) {
    let name = nt.name.as_str();
    let inner_go = go_type(&nt.inner_type, registry);
    w.line(&format!("type {name} {inner_go}"));
    w.blank();

    let default_enc = FieldEncoding::default_encoding();

    // Pack function — standalone because newtypes are value types
    w.open_block(&format!(
        "func Pack{name}(v {name}, w *vexil.BitWriter) error"
    ));
    let cast = format!("{inner_go}(v)");
    emit_write(
        w,
        &cast,
        &nt.inner_type,
        &default_enc,
        registry,
        "w",
        "return err",
    );
    w.line("return nil");
    w.close_block();
    w.blank();

    // Unpack function — returns (value, error)
    w.open_block(&format!(
        "func Unpack{name}(r *vexil.BitReader) ({name}, error)"
    ));
    w.line(&format!("var inner {inner_go}"));
    emit_read(
        w,
        "inner",
        &nt.inner_type,
        &default_enc,
        registry,
        "r",
        &format!("return {name}(inner), err"),
    );
    w.line(&format!("return {name}(inner), nil"));
    w.close_block();
    w.blank();
}
