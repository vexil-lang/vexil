use crate::ast::{DefaultValue, EnumBacking, PrimitiveType, SemanticType, SubByteType};
use crate::ir::{
    CompiledSchema, DeprecatedInfo, Encoding, FieldEncoding, ResolvedAnnotations, ResolvedType,
    TombstoneDef, TypeDef, TypeId, TypeRegistry,
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the canonical form of a single-file schema per spec §7.
/// Returns a deterministic UTF-8 string — single-space-delimited, no newlines.
pub fn canonical_form(compiled: &CompiledSchema) -> String {
    let mut out = String::new();

    // namespace
    out.push_str("namespace ");
    out.push_str(&compiled.namespace.join("."));

    // schema-level annotations
    if compiled.annotations != ResolvedAnnotations::default() {
        out.push(' ');
        let mut ann_buf = String::new();
        emit_annotations(&mut ann_buf, &compiled.annotations);
        out.push_str(ann_buf.trim_end());
    }

    // declarations in source order
    for &type_id in &compiled.declarations {
        out.push(' ');
        emit_type_def(&mut out, type_id, &compiled.registry);
    }

    out
}

/// Compute the BLAKE3 hash of the canonical form.
pub fn schema_hash(compiled: &CompiledSchema) -> [u8; 32] {
    let form = canonical_form(compiled);
    *blake3::hash(form.as_bytes()).as_bytes()
}

// ---------------------------------------------------------------------------
// Type string helpers (module-private)
// ---------------------------------------------------------------------------

fn type_str(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    #[allow(unreachable_patterns)]
    match ty {
        ResolvedType::Primitive(p) => primitive_str(p).to_owned(),
        ResolvedType::SubByte(s) => sub_byte_str(s),
        ResolvedType::Semantic(s) => semantic_str(s).to_owned(),
        ResolvedType::Named(id) => {
            if let Some(def) = registry.get(*id) {
                type_def_name(def).to_owned()
            } else {
                debug_assert!(false, "unresolved TypeId {:?} in canonical form", id);
                "<unresolved>".to_owned()
            }
        }
        ResolvedType::Optional(inner) => format!("optional<{}>", type_str(inner, registry)),
        ResolvedType::Array(inner) => format!("array<{}>", type_str(inner, registry)),
        ResolvedType::Map(k, v) => {
            format!("map<{}, {}>", type_str(k, registry), type_str(v, registry))
        }
        ResolvedType::Result(ok, err) => {
            format!(
                "result<{}, {}>",
                type_str(ok, registry),
                type_str(err, registry)
            )
        }
        _ => {
            debug_assert!(false, "unknown ResolvedType variant in canonical form");
            "<unknown>".to_owned()
        }
    }
}

fn primitive_str(p: &PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::Bool => "bool",
        PrimitiveType::U8 => "u8",
        PrimitiveType::U16 => "u16",
        PrimitiveType::U32 => "u32",
        PrimitiveType::U64 => "u64",
        PrimitiveType::I8 => "i8",
        PrimitiveType::I16 => "i16",
        PrimitiveType::I32 => "i32",
        PrimitiveType::I64 => "i64",
        PrimitiveType::F32 => "f32",
        PrimitiveType::F64 => "f64",
        PrimitiveType::Void => "void",
    }
}

fn sub_byte_str(s: &SubByteType) -> String {
    if s.signed {
        format!("i{}", s.bits)
    } else {
        format!("u{}", s.bits)
    }
}

fn semantic_str(s: &SemanticType) -> &'static str {
    match s {
        SemanticType::String => "string",
        SemanticType::Bytes => "bytes",
        SemanticType::Rgb => "rgb",
        SemanticType::Uuid => "uuid",
        SemanticType::Timestamp => "timestamp",
        SemanticType::Hash => "hash",
    }
}

fn type_def_name(def: &TypeDef) -> &str {
    #[allow(unreachable_patterns)]
    match def {
        TypeDef::Message(d) => d.name.as_str(),
        TypeDef::Enum(d) => d.name.as_str(),
        TypeDef::Flags(d) => d.name.as_str(),
        TypeDef::Union(d) => d.name.as_str(),
        TypeDef::Newtype(d) => d.name.as_str(),
        TypeDef::Config(d) => d.name.as_str(),
        _ => {
            debug_assert!(false, "unknown TypeDef variant in canonical form");
            "<unknown>"
        }
    }
}

// ---------------------------------------------------------------------------
// Annotation emission (module-private)
// ---------------------------------------------------------------------------

/// Emit annotations in sorted lexicographic order, each followed by a space.
/// Order: deprecated, doc, non_exhaustive, revision, since, version
fn emit_annotations(out: &mut String, ann: &ResolvedAnnotations) {
    // deprecated
    if let Some(dep) = &ann.deprecated {
        emit_deprecated(out, dep);
    }
    // doc (multiple, in source order)
    for d in &ann.doc {
        out.push_str("@doc(\"");
        out.push_str(d.as_str());
        out.push_str("\") ");
    }
    // non_exhaustive
    if ann.non_exhaustive {
        out.push_str("@non_exhaustive ");
    }
    // revision
    if let Some(rev) = ann.revision {
        out.push_str(&format!("@revision({rev}) "));
    }
    // since
    if let Some(since) = &ann.since {
        out.push_str(&format!("@since(\"{}\") ", since.as_str()));
    }
    // version
    if let Some(ver) = &ann.version {
        out.push_str(&format!("@version(\"{}\") ", ver.as_str()));
    }
}

fn emit_deprecated(out: &mut String, dep: &DeprecatedInfo) {
    if let Some(since) = &dep.since {
        out.push_str(&format!(
            "@deprecated(reason: \"{}\", since: \"{}\") ",
            dep.reason.as_str(),
            since.as_str()
        ));
    } else {
        out.push_str(&format!(
            "@deprecated(reason: \"{}\") ",
            dep.reason.as_str()
        ));
    }
}

// ---------------------------------------------------------------------------
// Encoding emission (module-private)
// ---------------------------------------------------------------------------

fn emit_encoding(out: &mut String, enc: &FieldEncoding) {
    emit_encoding_inner(out, &enc.encoding);
    if let Some(limit) = enc.limit {
        out.push_str(&format!("@limit({limit}) "));
    }
}

fn emit_encoding_inner(out: &mut String, enc: &Encoding) {
    #[allow(unreachable_patterns)]
    match enc {
        Encoding::Default => {}
        Encoding::Varint => out.push_str("@varint "),
        Encoding::ZigZag => out.push_str("@zigzag "),
        Encoding::Delta(inner) => {
            out.push_str("@delta ");
            emit_encoding_inner(out, inner);
        }
        _ => {
            debug_assert!(false, "unknown Encoding variant in canonical form");
        }
    }
}

// ---------------------------------------------------------------------------
// Tombstone emission (module-private)
// ---------------------------------------------------------------------------

fn emit_tombstones(out: &mut String, tombstones: &[TombstoneDef]) {
    // Sort by ordinal for determinism
    let mut sorted: Vec<&TombstoneDef> = tombstones.iter().collect();
    sorted.sort_by_key(|t| t.ordinal);
    for t in sorted {
        if let Some(since) = &t.since {
            out.push_str(&format!(
                "@removed({}, \"{}\", since: \"{}\") ",
                t.ordinal,
                t.reason.as_str(),
                since.as_str()
            ));
        } else {
            out.push_str(&format!(
                "@removed({}, \"{}\") ",
                t.ordinal,
                t.reason.as_str()
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Declaration emission (module-private)
// ---------------------------------------------------------------------------

fn emit_type_def(out: &mut String, type_id: TypeId, registry: &TypeRegistry) {
    let Some(def) = registry.get(type_id) else {
        debug_assert!(false, "unresolved TypeId {:?} in canonical form", type_id);
        return;
    };
    #[allow(unreachable_patterns)]
    match def {
        TypeDef::Message(msg) => {
            let mut ann_buf = String::new();
            emit_annotations(&mut ann_buf, &msg.annotations);
            if !ann_buf.is_empty() {
                out.push_str(ann_buf.trim_end());
                out.push(' ');
            }
            out.push_str("message ");
            out.push_str(msg.name.as_str());
            out.push_str(" {");
            let mut body = String::new();
            let mut fields = msg.fields.clone();
            fields.sort_by_key(|f| f.ordinal);
            for field in &fields {
                let mut f_ann = String::new();
                emit_annotations(&mut f_ann, &field.annotations);
                let type_s = type_str(&field.resolved_type, registry);
                let mut enc_buf = String::new();
                emit_encoding(&mut enc_buf, &field.encoding);
                let mut field_str =
                    format!("{} @{} : {}", field.name.as_str(), field.ordinal, type_s);
                if !enc_buf.is_empty() {
                    field_str.push(' ');
                    field_str.push_str(enc_buf.trim_end());
                }
                if !f_ann.is_empty() {
                    field_str = format!("{} {}", f_ann.trim_end(), field_str);
                }
                body.push_str(&field_str);
                body.push(' ');
            }
            emit_tombstones(&mut body, &msg.tombstones);
            let body_trimmed = body.trim_end();
            if body_trimmed.is_empty() {
                out.push('}');
            } else {
                out.push(' ');
                out.push_str(body_trimmed);
                out.push_str(" }");
            }
        }
        TypeDef::Enum(enm) => {
            let mut ann_buf = String::new();
            emit_annotations(&mut ann_buf, &enm.annotations);
            if !ann_buf.is_empty() {
                out.push_str(ann_buf.trim_end());
                out.push(' ');
            }
            out.push_str("enum ");
            out.push_str(enm.name.as_str());
            if let Some(backing) = &enm.backing {
                let backing_str = match backing {
                    EnumBacking::U8 => "u8",
                    EnumBacking::U16 => "u16",
                    EnumBacking::U32 => "u32",
                    EnumBacking::U64 => "u64",
                };
                out.push_str(&format!(" : {}", backing_str));
            }
            out.push_str(" {");
            let mut body = String::new();
            let mut variants = enm.variants.clone();
            variants.sort_by_key(|v| v.ordinal);
            for v in &variants {
                let mut v_ann = String::new();
                emit_annotations(&mut v_ann, &v.annotations);
                if !v_ann.is_empty() {
                    body.push_str(v_ann.trim_end());
                    body.push(' ');
                }
                body.push_str(&format!("{} = {} ", v.name.as_str(), v.ordinal));
            }
            emit_tombstones(&mut body, &enm.tombstones);
            let body_trimmed = body.trim_end();
            if body_trimmed.is_empty() {
                out.push('}');
            } else {
                out.push(' ');
                out.push_str(body_trimmed);
                out.push_str(" }");
            }
        }
        TypeDef::Flags(flags) => {
            let mut ann_buf = String::new();
            emit_annotations(&mut ann_buf, &flags.annotations);
            if !ann_buf.is_empty() {
                out.push_str(ann_buf.trim_end());
                out.push(' ');
            }
            out.push_str("flags ");
            out.push_str(flags.name.as_str());
            out.push_str(" {");
            let mut body = String::new();
            let mut bits = flags.bits.clone();
            bits.sort_by_key(|b| b.bit);
            for b in &bits {
                let mut b_ann = String::new();
                emit_annotations(&mut b_ann, &b.annotations);
                if !b_ann.is_empty() {
                    body.push_str(b_ann.trim_end());
                    body.push(' ');
                }
                body.push_str(&format!("{} = {} ", b.name.as_str(), b.bit));
            }
            emit_tombstones(&mut body, &flags.tombstones);
            let body_trimmed = body.trim_end();
            if body_trimmed.is_empty() {
                out.push('}');
            } else {
                out.push(' ');
                out.push_str(body_trimmed);
                out.push_str(" }");
            }
        }
        TypeDef::Union(u) => {
            let mut ann_buf = String::new();
            emit_annotations(&mut ann_buf, &u.annotations);
            if !ann_buf.is_empty() {
                out.push_str(ann_buf.trim_end());
                out.push(' ');
            }
            out.push_str("union ");
            out.push_str(u.name.as_str());
            out.push_str(" {");
            let mut body = String::new();
            let mut variants = u.variants.clone();
            variants.sort_by_key(|v| v.ordinal);
            for var in &variants {
                let mut v_ann = String::new();
                emit_annotations(&mut v_ann, &var.annotations);
                if !v_ann.is_empty() {
                    body.push_str(v_ann.trim_end());
                    body.push(' ');
                }
                body.push_str(&format!("{} @{} {{", var.name.as_str(), var.ordinal));
                let mut var_body = String::new();
                let mut fields = var.fields.clone();
                fields.sort_by_key(|f| f.ordinal);
                for field in &fields {
                    let mut f_ann = String::new();
                    emit_annotations(&mut f_ann, &field.annotations);
                    let type_s = type_str(&field.resolved_type, registry);
                    let mut enc_buf = String::new();
                    emit_encoding(&mut enc_buf, &field.encoding);
                    let mut field_str =
                        format!("{} @{} : {}", field.name.as_str(), field.ordinal, type_s);
                    if !enc_buf.is_empty() {
                        field_str.push(' ');
                        field_str.push_str(enc_buf.trim_end());
                    }
                    if !f_ann.is_empty() {
                        field_str = format!("{} {}", f_ann.trim_end(), field_str);
                    }
                    var_body.push_str(&field_str);
                    var_body.push(' ');
                }
                emit_tombstones(&mut var_body, &var.tombstones);
                let var_body_trimmed = var_body.trim_end();
                if var_body_trimmed.is_empty() {
                    body.push('}');
                } else {
                    body.push(' ');
                    body.push_str(var_body_trimmed);
                    body.push_str(" }");
                }
                body.push(' ');
            }
            emit_tombstones(&mut body, &u.tombstones);
            let body_trimmed = body.trim_end();
            if body_trimmed.is_empty() {
                out.push('}');
            } else {
                out.push(' ');
                out.push_str(body_trimmed);
                out.push_str(" }");
            }
        }
        TypeDef::Newtype(nt) => {
            let mut ann_buf = String::new();
            emit_annotations(&mut ann_buf, &nt.annotations);
            if !ann_buf.is_empty() {
                out.push_str(ann_buf.trim_end());
                out.push(' ');
            }
            let type_s = type_str(&nt.inner_type, registry);
            out.push_str(&format!("newtype {} = {}", nt.name.as_str(), type_s));
        }
        TypeDef::Config(cfg) => {
            let mut ann_buf = String::new();
            emit_annotations(&mut ann_buf, &cfg.annotations);
            if !ann_buf.is_empty() {
                out.push_str(ann_buf.trim_end());
                out.push(' ');
            }
            out.push_str("config ");
            out.push_str(cfg.name.as_str());
            out.push_str(" {");
            let mut body = String::new();
            let mut fields = cfg.fields.clone();
            fields.sort_by(|a, b| a.name.cmp(&b.name));
            for field in &fields {
                let mut f_ann = String::new();
                emit_annotations(&mut f_ann, &field.annotations);
                if !f_ann.is_empty() {
                    body.push_str(f_ann.trim_end());
                    body.push(' ');
                }
                let type_s = type_str(&field.resolved_type, registry);
                body.push_str(&format!("{} : {}", field.name.as_str(), type_s));
                body.push_str(" = ");
                emit_default_value(&mut body, &field.default_value);
                body.push(' ');
            }
            let body_trimmed = body.trim_end();
            if body_trimmed.is_empty() {
                out.push('}');
            } else {
                out.push(' ');
                out.push_str(body_trimmed);
                out.push_str(" }");
            }
        }
        _ => {
            debug_assert!(false, "unknown TypeDef variant in canonical form");
        }
    }
}

// ---------------------------------------------------------------------------
// Default value emission (module-private)
// ---------------------------------------------------------------------------

fn emit_default_value(out: &mut String, val: &DefaultValue) {
    match val {
        DefaultValue::None => out.push_str("none"),
        DefaultValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        DefaultValue::Int(n) => out.push_str(&format!("{n}")),
        DefaultValue::UInt(n) => out.push_str(&format!("{n}")),
        DefaultValue::Float(f) => out.push_str(&format!("{f:?}")),
        DefaultValue::Str(s) => out.push_str(&format!("\"{s}\"")),
        DefaultValue::Ident(s) => out.push_str(s.as_str()),
        DefaultValue::UpperIdent(s) => out.push_str(s.as_str()),
        DefaultValue::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_default_value(out, &item.node);
            }
            out.push(']');
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{PrimitiveType, SemanticType, SubByteType};

    fn dummy_registry() -> crate::ir::TypeRegistry {
        crate::ir::TypeRegistry::new()
    }

    #[test]
    fn minimal_namespace_only() {
        let result = crate::compile("namespace test.minimal\nmessage Empty {}");
        let compiled = result.compiled.unwrap();
        let form = canonical_form(&compiled);
        assert!(form.starts_with("namespace test.minimal"));
        let hash = schema_hash(&compiled);
        assert_eq!(hash.len(), 32);
    }

    // -----------------------------------------------------------------------
    // Task 2: type string tests
    // -----------------------------------------------------------------------

    #[test]
    fn type_string_primitives() {
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::Bool),
                &dummy_registry()
            ),
            "bool"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::U32),
                &dummy_registry()
            ),
            "u32"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::I64),
                &dummy_registry()
            ),
            "i64"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::F64),
                &dummy_registry()
            ),
            "f64"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::Void),
                &dummy_registry()
            ),
            "void"
        );
    }

    #[test]
    fn type_string_sub_byte() {
        assert_eq!(
            type_str(
                &ResolvedType::SubByte(SubByteType {
                    bits: 3,
                    signed: false
                }),
                &dummy_registry()
            ),
            "u3"
        );
        assert_eq!(
            type_str(
                &ResolvedType::SubByte(SubByteType {
                    bits: 5,
                    signed: true
                }),
                &dummy_registry()
            ),
            "i5"
        );
    }

    #[test]
    fn type_string_semantic() {
        assert_eq!(
            type_str(
                &ResolvedType::Semantic(SemanticType::String),
                &dummy_registry()
            ),
            "string"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Semantic(SemanticType::Uuid),
                &dummy_registry()
            ),
            "uuid"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Semantic(SemanticType::Timestamp),
                &dummy_registry()
            ),
            "timestamp"
        );
    }

    #[test]
    fn type_string_parameterized() {
        let inner = ResolvedType::Primitive(PrimitiveType::U32);
        assert_eq!(
            type_str(
                &ResolvedType::Optional(Box::new(inner.clone())),
                &dummy_registry()
            ),
            "optional<u32>"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Array(Box::new(inner.clone())),
                &dummy_registry()
            ),
            "array<u32>"
        );
        let key = ResolvedType::Semantic(SemanticType::String);
        assert_eq!(
            type_str(
                &ResolvedType::Map(Box::new(key), Box::new(inner.clone())),
                &dummy_registry()
            ),
            "map<string, u32>"
        );
        let ok = ResolvedType::Primitive(PrimitiveType::U32);
        let err = ResolvedType::Semantic(SemanticType::String);
        assert_eq!(
            type_str(
                &ResolvedType::Result(Box::new(ok), Box::new(err)),
                &dummy_registry()
            ),
            "result<u32, string>"
        );
    }

    // -----------------------------------------------------------------------
    // Task 3: annotation + encoding tests
    // -----------------------------------------------------------------------

    #[test]
    fn annotation_sorting() {
        let result = crate::compile(
            r#"
            @version("2.0.0")
            namespace test.anno
            @since("1.0") @doc("hello") @deprecated(reason: "old") @revision(3)
            @non_exhaustive
            enum Foo { A @0 }
        "#,
        );
        let compiled = result.compiled.unwrap();
        let form = canonical_form(&compiled);
        // Schema annotations: @version after namespace
        assert!(
            form.contains("namespace test.anno @version(\"2.0.0\")"),
            "form was: {form}"
        );
        // Type annotations sorted: deprecated < doc < non_exhaustive < revision < since
        assert!(
            form.contains(
                "@deprecated(reason: \"old\") @doc(\"hello\") @non_exhaustive @revision(3) @since(\"1.0\") enum Foo"
            ),
            "form was: {form}"
        );
    }

    #[test]
    fn encoding_annotations() {
        let result = crate::compile(
            r#"
            namespace test.enc
            message Enc {
                a @0 : u32 @varint
                b @1 : i32 @zigzag
                c @2 : u64 @delta
                d @3 : u32 @delta @varint
                e @4 : string @limit(100)
            }
        "#,
        );
        let compiled = result.compiled.unwrap();
        let form = canonical_form(&compiled);
        assert!(form.contains("a @0 : u32 @varint"), "form was: {form}");
        assert!(form.contains("b @1 : i32 @zigzag"), "form was: {form}");
        assert!(form.contains("c @2 : u64 @delta"), "form was: {form}");
        assert!(
            form.contains("d @3 : u32 @delta @varint"),
            "form was: {form}"
        );
        assert!(
            form.contains("e @4 : string @limit(100)"),
            "form was: {form}"
        );
    }

    #[test]
    fn canonical_message() {
        let result = crate::compile("namespace t.m\nmessage Foo { x @0 : u32 y @1 : string }");
        let form = canonical_form(&result.compiled.unwrap());
        assert!(
            form.contains("message Foo { x @0 : u32 y @1 : string }"),
            "form was: {form}"
        );
    }

    #[test]
    fn canonical_enum() {
        let result = crate::compile("namespace t.e\nenum Color { Red @0 Green @1 Blue @2 }");
        let form = canonical_form(&result.compiled.unwrap());
        assert!(
            form.contains("enum Color { Red = 0 Green = 1 Blue = 2 }"),
            "form was: {form}"
        );
    }

    #[test]
    fn canonical_enum_with_backing() {
        let result = crate::compile("namespace t.e\nenum Small : u8 { A @0 B @1 }");
        let form = canonical_form(&result.compiled.unwrap());
        assert!(
            form.contains("enum Small : u8 { A = 0 B = 1 }"),
            "form was: {form}"
        );
    }

    #[test]
    fn canonical_flags() {
        let result = crate::compile("namespace t.f\nflags Perms { Read @0 Write @1 Exec @2 }");
        let form = canonical_form(&result.compiled.unwrap());
        assert!(
            form.contains("flags Perms { Read = 0 Write = 1 Exec = 2 }"),
            "form was: {form}"
        );
    }

    #[test]
    fn canonical_union() {
        let result = crate::compile("namespace t.u\nunion Shape { Circle @0 { radius @0 : f32 } Rect @1 { w @0 : f32 h @1 : f32 } }");
        let form = canonical_form(&result.compiled.unwrap());
        assert!(
            form.contains(
                "union Shape { Circle @0 { radius @0 : f32 } Rect @1 { w @0 : f32 h @1 : f32 } }"
            ),
            "form was: {form}"
        );
    }

    #[test]
    fn canonical_newtype() {
        let result = crate::compile("namespace t.n\nnewtype UserId : u64");
        let form = canonical_form(&result.compiled.unwrap());
        assert!(form.contains("newtype UserId = u64"), "form was: {form}");
    }

    #[test]
    fn canonical_config() {
        let result = crate::compile(
            "namespace t.c\nconfig Defaults { timeout : u32 = 30 name : string = \"hello\" }",
        );
        let form = canonical_form(&result.compiled.unwrap());
        // Config fields sorted by name: name < timeout
        assert!(
            form.contains("config Defaults { name : string = \"hello\" timeout : u32 = 30 }"),
            "form was: {form}"
        );
    }

    #[test]
    fn canonical_tombstones() {
        let result = crate::compile(
            r#"
            namespace t.t
            message Evolving {
                name @0 : string
                @removed(1, reason: "replaced by full_name")
                @removed(2, reason: "no longer needed", since: "2.0")
            }
        "#,
        );
        let form = canonical_form(&result.compiled.unwrap());
        assert!(form.contains("name @0 : string @removed(1, \"replaced by full_name\") @removed(2, \"no longer needed\", since: \"2.0\")"), "form was: {form}");
    }
}
