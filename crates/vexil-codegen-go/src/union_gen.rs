use vexil_lang::ir::{TypeRegistry, UnionDef};

use crate::emit::CodeWriter;
use crate::message::{emit_read, emit_write};
use crate::types::{go_type, to_pascal_case};

/// Emit a complete union: interface + variant structs + Pack/Unpack standalone functions.
///
/// Wire format: discriminant (LEB128) + payload byte length (LEB128) + payload bytes.
pub fn emit_union(w: &mut CodeWriter, un: &UnionDef, registry: &TypeRegistry) {
    let name = un.name.as_str();

    // Interface type with marker method
    w.open_block(&format!("type {name} interface"));
    w.line(&format!("is{name}()"));
    w.close_block();
    w.blank();

    // Emit individual variant structs
    for variant in &un.variants {
        let vname = variant.name.as_str();
        let struct_name = format!("{name}{vname}");
        w.open_block(&format!("type {struct_name} struct"));
        for field in &variant.fields {
            let go_ty = go_type(&field.resolved_type, registry);
            let field_name = to_pascal_case(&field.name);
            w.line(&format!("{field_name} {go_ty}"));
        }
        w.line("Unknown []byte");
        w.close_block();
        w.blank();

        // Marker method
        w.open_block(&format!("func ({struct_name}) is{name}()"));
        w.close_block();
        w.blank();
    }

    // Pack function (standalone, not method on interface)
    w.open_block(&format!(
        "func Pack{name}(v {name}, w *vexil.BitWriter) error"
    ));
    w.open_block("switch t := v.(type)");

    for variant in &un.variants {
        let vname = variant.name.as_str();
        let struct_name = format!("{name}{vname}");
        let ordinal = variant.ordinal;

        w.open_block(&format!("case *{struct_name}:"));
        w.line(&format!("w.WriteLeb128({ordinal})"));

        if variant.fields.is_empty() {
            w.line("w.WriteLeb128(0)");
        } else {
            w.line("pw := vexil.NewBitWriter()");
            for field in &variant.fields {
                let field_name = to_pascal_case(&field.name);
                let access = format!("t.{field_name}");
                emit_write(
                    w,
                    &access,
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    "pw",
                    "return err",
                );
            }
            w.line("pw.FlushToByteBoundary()");
            w.line("payload := pw.Finish()");
            w.line("w.WriteLeb128(uint64(len(payload)))");
            w.line("w.WriteRawBytes(payload)");
        }
        w.close_block();
    }

    w.close_block(); // switch
    w.line("return nil");
    w.close_block(); // function
    w.blank();

    // Unpack function (standalone) — returns (T, error)
    let unpack_err_return = "return nil, err";
    w.open_block(&format!(
        "func Unpack{name}(r *vexil.BitReader) ({name}, error)"
    ));
    w.line("r.FlushToByteBoundary()");
    w.line("disc, err := r.ReadLeb128(10)");
    w.open_block("if err != nil");
    w.line(unpack_err_return);
    w.close_block();
    w.line("length, err := r.ReadLeb128(4)");
    w.open_block("if err != nil");
    w.line(unpack_err_return);
    w.close_block();
    w.open_block("switch disc");

    for variant in &un.variants {
        let vname = variant.name.as_str();
        let struct_name = format!("{name}{vname}");
        let ordinal = variant.ordinal;

        w.open_block(&format!("case {ordinal}:"));
        if variant.fields.is_empty() {
            w.open_block("");
            w.line("_, err := r.ReadRawBytes(int(length))");
            w.open_block("if err != nil");
            w.line(unpack_err_return);
            w.close_block();
            w.close_block();
            w.line(&format!("return &{struct_name}{{}}, nil"));
        } else {
            w.line("payloadBytes, err := r.ReadRawBytes(int(length))");
            w.open_block("if err != nil");
            w.line(unpack_err_return);
            w.close_block();
            w.line("pr := vexil.NewBitReader(payloadBytes)");
            w.line(&format!("result := &{struct_name}{{}}"));
            for field in &variant.fields {
                let field_name = to_pascal_case(&field.name);
                let target = format!("result.{field_name}");
                emit_read(
                    w,
                    &target,
                    &field.resolved_type,
                    &field.encoding,
                    registry,
                    "pr",
                    unpack_err_return,
                );
            }
            w.line("pr.FlushToByteBoundary()");
            w.line("result.Unknown = pr.ReadRemaining()");
            w.line("return result, nil");
        }
        w.close_block();
    }

    // Default case
    w.open_block("default:");
    w.open_block("");
    w.line("_, err := r.ReadRawBytes(int(length))");
    w.open_block("if err != nil");
    w.line(unpack_err_return);
    w.close_block();
    w.close_block();
    w.line("return nil, fmt.Errorf(\"unknown discriminant %d\", disc)");
    w.close_block();

    w.close_block(); // switch
    w.close_block(); // function
    w.blank();
}
