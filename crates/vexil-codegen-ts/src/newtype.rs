use vexil_lang::ir::{NewtypeDef, TypeRegistry};

use crate::emit::CodeWriter;
use crate::types::ts_type;

/// Emit a newtype: type alias only (no codec — encode/decode uses inner type).
pub fn emit_newtype(w: &mut CodeWriter, nt: &NewtypeDef, registry: &TypeRegistry) {
    let name = nt.name.as_str();
    let inner_ts = ts_type(&nt.inner_type, registry);
    w.line(&format!("export type {name} = {inner_ts};"));
    w.blank();
}
