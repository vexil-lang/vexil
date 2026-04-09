use vexil_lang::ir::{FieldEncoding, NewtypeDef, TypeRegistry};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::py_type;

/// Emit a complete Python newtype class.
pub fn emit_newtype(w: &mut CodeWriter, nt: &NewtypeDef, registry: &TypeRegistry) {
    let name = nt.name.as_str();
    let inner_py = py_type(&nt.inner_type, registry);

    w.open_block(&format!("class {name}"));
    w.blank();

    // __init__
    w.open_block("def __init__(self, value)");
    w.line(&format!("self.value: {inner_py} = value"));
    w.close_block();
    w.blank();

    // encode method
    let default_enc = FieldEncoding::default_encoding();
    w.open_block("def encode(self) -> bytes");
    w.line("w = _BitWriter()");
    emit_write(w, "self.value", &nt.inner_type, &default_enc, registry, "w");
    w.line("return w.finish()");
    w.close_block();
    w.blank();

    // decode static method
    w.line("@staticmethod");
    w.open_block(&format!("def decode(data: bytes) -> {name}"));
    w.line("r = _BitReader(data)");
    w.line(&format!(
        "inner: {inner_py} = None  # type: ignore[assignment]"
    ));
    emit_read(w, "inner", &nt.inner_type, &default_enc, registry, "r");
    w.line(&format!("return {name}(inner)"));
    w.close_block();
    w.blank();

    // __repr__
    w.line("def __repr__(self) -> str:");
    w.indent();
    w.line(&format!("return f\"{name}({{self.value!r}})\""));
    w.close_block();
    w.blank();

    // __eq__
    w.open_block("def __eq__(self, other)");
    w.line(&format!(
        "return isinstance(other, {name}) and self.value == other.value"
    ));
    w.close_block();

    w.close_block();
    w.blank();
}
