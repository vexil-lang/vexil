use vexil_lang::diagnostic::{ErrorClass, Severity};
use vexil_lang::ir::{Encoding, ResolvedType, TypeDef, WireSize};

fn read_corpus(dir: &str, file: &str) -> String {
    let path = format!("{}/../../corpus/{dir}/{file}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

#[test]
fn valid_corpus_compiles() {
    let valid_files = [
        "001_minimal.vexil",
        "002_primitives.vexil",
        "003_sub_byte.vexil",
        "004_semantic_types.vexil",
        "005_parameterized.vexil",
        "006_message.vexil",
        "007_enum.vexil",
        "008_flags.vexil",
        "009_union.vexil",
        "010_newtype.vexil",
        "011_config.vexil",
        "012_imports.vexil",
        "013_annotations.vexil",
        "014_keywords_as_fields.vexil",
        "015_forward_refs.vexil",
        "016_recursive.vexil",
        "017_escapes.vexil",
        "018_comments.vexil",
        "019_evolution_append_field.vexil",
        "020_evolution_add_variant.vexil",
        "021_empty_optionals.vexil",
        "022_nested_schemas.vexil",
        "023_recursive_depth.vexil",
        "024_zero_length_payload.vexil",
        "025_evolution_deprecate.vexil",
        "026_required_to_optional.vexil",
        "027_delta_on_message.vexil",
        "028_typed_tombstone.vexil",
    ];
    for file in &valid_files {
        let source = read_corpus("valid", file);
        let result = vexil_lang::compile(&source);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "expected no errors in {file}, got: {errors:#?}"
        );
        assert!(
            result.compiled.is_some(),
            "expected CompiledSchema for valid {file}"
        );
    }
}

#[test]
fn compile_simple_message() {
    let source = "namespace test.simple\nmessage Foo { a @0 : u32  b @1 : bool }";
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "errors: {errors:#?}");
    let compiled = result.compiled.as_ref().unwrap();
    assert_eq!(compiled.declarations.len(), 1);
    assert_eq!(compiled.registry.len(), 1);
}

/// Forward references resolve to correct TypeIds.
#[test]
fn type_resolution_forward_ref() {
    let source = r#"
namespace test.resolve
message Container { item @0 : Item }
message Item { value @0 : u32 }
"#;
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let container_id = compiled.declarations[0];
    let item_id = compiled.declarations[1];
    let container = compiled.registry.get(container_id).unwrap();
    if let TypeDef::Message(msg) = container {
        assert_eq!(msg.fields.len(), 1);
        assert_eq!(msg.fields[0].resolved_type, ResolvedType::Named(item_id));
    } else {
        panic!("expected Message");
    }
}

/// Newtype inner type resolves to primitive.
#[test]
fn newtype_resolution() {
    let source = "namespace test.nt\nnewtype SessionId : u64";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Newtype(nt) = compiled.registry.get(id).unwrap() {
        assert_eq!(
            nt.inner_type,
            ResolvedType::Primitive(vexil_lang::ast::PrimitiveType::U64)
        );
    } else {
        panic!("expected Newtype");
    }
}

/// Enum backing is None when unspecified (auto-sized by typeck).
#[test]
fn enum_default_backing() {
    let source = "namespace test.en\nenum Dir { North @0  South @1 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Enum(en) = compiled.registry.get(id).unwrap() {
        assert_eq!(en.backing, None);
        assert_eq!(en.variants.len(), 2);
    } else {
        panic!("expected Enum");
    }
}

/// Encoding annotations are correctly computed.
#[test]
fn encoding_varint() {
    let source = "namespace test.enc\nmessage M { v @0 @varint : u32 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(msg.fields[0].encoding.encoding, Encoding::Varint);
        assert_eq!(msg.fields[0].encoding.limit, None);
    } else {
        panic!("expected Message");
    }
}

/// @delta wraps the base encoding.
#[test]
fn encoding_delta_varint() {
    let source = "namespace test.dv\nmessage M { v @0 @delta @varint : u32 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(
            msg.fields[0].encoding.encoding,
            Encoding::Delta(Box::new(Encoding::Varint))
        );
    } else {
        panic!("expected Message");
    }
}

/// @limit is extracted correctly.
#[test]
fn encoding_limit() {
    let source = "namespace test.lim\nmessage M { s @0 : string @limit(1024) }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(msg.fields[0].encoding.encoding, Encoding::Default);
        assert_eq!(msg.fields[0].encoding.limit, Some(1024));
    } else {
        panic!("expected Message");
    }
}

/// Annotations are resolved into structured form.
#[test]
fn resolved_annotations() {
    let source = r#"
namespace test.ann
@doc("A test") @deprecated(since: "1.0", reason: "use B")
message A { v @0 : u32 }
"#;
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(msg.annotations.doc, vec!["A test"]);
        let dep = msg.annotations.deprecated.as_ref().unwrap();
        assert_eq!(dep.reason.as_str(), "use B");
        assert_eq!(dep.since.as_deref(), Some("1.0"));
    } else {
        panic!("expected Message");
    }
}

/// @deprecated carries both reason and since fields.
#[test]
fn deprecated_info_has_since() {
    let src = r#"
        namespace test.deprecated
        message Old {
            @deprecated(since: "1.0", reason: "use New")
            name @0 : string
        }
    "#;
    let result = vexil_lang::compile(src);
    assert!(result
        .diagnostics
        .iter()
        .all(|d| d.severity != Severity::Error));
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    let msg = match compiled.registry.get(id).unwrap() {
        TypeDef::Message(m) => m,
        _ => panic!("expected message"),
    };
    let dep = msg.fields[0].annotations.deprecated.as_ref().unwrap();
    assert_eq!(dep.reason.as_str(), "use New");
    assert_eq!(dep.since.as_deref(), Some("1.0"));
}

/// Import stubs are created in the registry.
#[test]
fn import_named_creates_stubs() {
    let source = r#"
namespace test.imp
import { Shape, Color } from test.unions
message M { s @0 : Shape }
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "errors: {errors:#?}");
    let compiled = result.compiled.as_ref().unwrap();
    assert!(compiled.registry.lookup("Shape").is_some());
    assert!(compiled.registry.lookup("Color").is_some());
}

/// Qualified type reference through alias.
#[test]
fn import_aliased_qualified_ref() {
    let source = r#"
namespace test.alias
import test.enums as E
message M { kind @0 : E.ClientKind }
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "errors: {errors:#?}");
    assert!(result.compiled.is_some());
}

/// Wildcard imports suppress unknown type errors.
#[test]
fn import_wildcard_suppresses_unknown() {
    let source = r#"
namespace test.wild
import test.newtypes
message M { id @0 : SessionId }
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "errors: {errors:#?}");
    assert!(result.compiled.is_some());
}

/// Invalid corpus files produce error diagnostics.
#[test]
fn invalid_corpus_produces_errors() {
    let invalid_dir = format!("{}/../../corpus/invalid", env!("CARGO_MANIFEST_DIR"));
    let entries = std::fs::read_dir(&invalid_dir).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("vexil") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let result = vexil_lang::compile(&source);
        let has_error = result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error);
        assert!(
            has_error,
            "expected errors for invalid file {}, got none",
            path.display()
        );
    }
}

/// Wire size for a message with u32 + bool = Fixed(33 bits).
#[test]
fn wire_size_fixed_message() {
    let source = "namespace test.ws\nmessage M { a @0 : u32  b @1 : bool }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(msg.wire_size, Some(WireSize::Fixed(33)));
    } else {
        panic!("expected Message");
    }
}

/// Wire size for a message with string = Variable.
#[test]
fn wire_size_variable_string() {
    let source = "namespace test.vs\nmessage M { s @0 : string }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert!(matches!(msg.wire_size, Some(WireSize::Variable { .. })));
    } else {
        panic!("expected Message");
    }
}

/// Wire size for optional<u8> = Variable(min=1, max=9).
#[test]
fn wire_size_optional() {
    let source = "namespace test.opt\nmessage M { v @0 : optional<u8> }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert!(matches!(
            msg.wire_size,
            Some(WireSize::Variable {
                min_bits: 1,
                max_bits: Some(9)
            })
        ));
    } else {
        panic!("expected Message");
    }
}

/// @varint on u32 makes it Variable(min=8, max=40).
#[test]
fn wire_size_varint() {
    let source = "namespace test.vw\nmessage M { v @0 @varint : u32 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert!(matches!(
            msg.wire_size,
            Some(WireSize::Variable {
                min_bits: 8,
                max_bits: Some(40)
            })
        ));
    } else {
        panic!("expected Message");
    }
}

/// Enum wire size = Fixed(wire_bits), tested via message embedding.
/// Auto-sized enum with 1 variant (ordinal 0) → 1 bit.
#[test]
fn wire_size_enum() {
    let source = "namespace test.ew2\nenum Dir { N @0 }\nmessage M { d @0 : Dir }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let msg_id = compiled.declarations[1];
    if let TypeDef::Message(msg) = compiled.registry.get(msg_id).unwrap() {
        assert_eq!(msg.wire_size, Some(WireSize::Fixed(1)));
    } else {
        panic!("expected Message");
    }
}

/// Enum wire size with explicit backing = Fixed(backing bits).
#[test]
fn wire_size_enum_explicit_backing() {
    let source = "namespace test.ew3\nenum Dir : u32 { N @0 }\nmessage M { d @0 : Dir }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let msg_id = compiled.declarations[1];
    if let TypeDef::Message(msg) = compiled.registry.get(msg_id).unwrap() {
        assert_eq!(msg.wire_size, Some(WireSize::Fixed(32)));
    } else {
        panic!("expected Message");
    }
}

/// Flags wire size = Fixed(wire_bytes * 8). One bit in @0 → 1 byte → 8 bits.
#[test]
fn wire_size_flags() {
    let source = "namespace test.fw\nflags F { R @0 }\nmessage M { f @0 : F }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let msg_id = compiled.declarations[1];
    if let TypeDef::Message(msg) = compiled.registry.get(msg_id).unwrap() {
        assert_eq!(msg.wire_size, Some(WireSize::Fixed(8)));
    } else {
        panic!("expected Message");
    }
}

/// Newtype wire size = same as inner type.
#[test]
fn wire_size_newtype() {
    let source = "namespace test.nw\nnewtype Id : u64\nmessage M { id @0 : Id }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let msg_id = compiled.declarations[1];
    if let TypeDef::Message(msg) = compiled.registry.get(msg_id).unwrap() {
        assert_eq!(msg.wire_size, Some(WireSize::Fixed(64)));
    } else {
        panic!("expected Message");
    }
}

/// Valid recursion through optional — no error.
#[test]
fn recursion_through_optional_valid() {
    let source = r#"
namespace test.rec
message Node {
    value @0 : i32
    next  @1 : optional<Node>
}
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "should allow recursion through optional: {errors:#?}"
    );
}

/// Valid recursion through array — no error.
#[test]
fn recursion_through_array_valid() {
    let source = r#"
namespace test.rec2
message Tree {
    value    @0 : i32
    children @1 : array<Tree>
}
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "should allow recursion through array: {errors:#?}"
    );
}

/// Valid mutual recursion through union — no error (corpus 016).
#[test]
fn recursion_through_union_valid() {
    let source = r#"
namespace test.rec3
message Expr {
    kind @0 : ExprKind
}
union ExprKind {
    Literal @0 { value @0 : i64 }
    Binary  @1 { left @0 : Expr  op @1 : u8  right @2 : Expr }
}
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "should allow mutual recursion through union: {errors:#?}"
    );
}

/// Invalid direct self-recursion — error.
#[test]
fn recursion_direct_invalid() {
    let source = r#"
namespace test.rec4
message Bad {
    self_ref @0 : Bad
}
"#;
    let result = vexil_lang::compile(source);
    let has_recursive_error = result
        .diagnostics
        .iter()
        .any(|d| d.class == ErrorClass::RecursiveTypeInfinite);
    assert!(
        has_recursive_error,
        "should detect direct infinite recursion"
    );
}

/// Invalid mutual direct recursion (A -> B -> A) — error.
#[test]
fn recursion_mutual_direct_invalid() {
    let source = r#"
namespace test.rec5
message A { b @0 : B }
message B { a @0 : A }
"#;
    let result = vexil_lang::compile(source);
    let has_recursive_error = result
        .diagnostics
        .iter()
        .any(|d| d.class == ErrorClass::RecursiveTypeInfinite);
    assert!(
        has_recursive_error,
        "should detect mutual direct infinite recursion"
    );
}

/// Newtype terminal type resolves to primitive.
#[test]
fn newtype_terminal_type() {
    let source = "namespace test.ntterm\nnewtype Id : u64";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Newtype(nt) = compiled.registry.get(id).unwrap() {
        assert_eq!(
            nt.terminal_type,
            ResolvedType::Primitive(vexil_lang::ast::PrimitiveType::U64)
        );
    } else {
        panic!("expected Newtype");
    }
}

/// Schema-level @version annotation is preserved.
#[test]
fn schema_annotations_preserved() {
    let source = "@version(\"1.2.0\")\nnamespace test.sa\nmessage M { v @0 : u32 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    assert_eq!(compiled.annotations.version.as_deref(), Some("1.2.0"));
}

/// EnumDef.wire_bits: exhaustive enum with no explicit backing — minimal bits.
/// Direction: 4 variants (0-3) → ceil(log2(4)) = 2 bits.
#[test]
fn enum_wire_bits_exhaustive_no_backing() {
    let src = r#"
        namespace test.wire
        enum Direction { North @0  South @1  East @2  West @3 }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Enum(e) => assert_eq!(e.wire_bits, 2),
        _ => panic!("expected enum"),
    }
}

/// EnumDef.wire_bits: non-exhaustive with 4 variants → max(ceil(log2(4)), 8) = 8.
#[test]
fn enum_wire_bits_non_exhaustive() {
    let src = r#"
        namespace test.wire2
        @non_exhaustive
        enum Kind { A @0  B @1  C @2  D @3 }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Enum(e) => assert_eq!(e.wire_bits, 8),
        _ => panic!("expected enum"),
    }
}

/// EnumDef.wire_bits: explicit backing u16 → 16 bits.
#[test]
fn enum_wire_bits_explicit_backing() {
    let src = r#"
        namespace test.wire3
        enum Status : u16 { Ok @0  Err @1 }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Enum(e) => assert_eq!(e.wire_bits, 16),
        _ => panic!("expected enum"),
    }
}

/// FlagsDef.wire_bytes: 4 bits all in 0-7 range → 1 byte.
#[test]
fn flags_wire_bytes_low_bits() {
    let src = r#"
        namespace test.wire4
        flags Perms { Read @0  Write @1  Exec @2  Del @3 }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Flags(f) => assert_eq!(f.wire_bytes, 1),
        _ => panic!("expected flags"),
    }
}

/// FlagsDef.wire_bytes: bit @32 → 8 bytes.
#[test]
fn flags_wire_bytes_high_bits() {
    let src = r#"
        namespace test.wire5
        flags Wide { Low @0  High @32 }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Flags(f) => assert_eq!(f.wire_bytes, 8),
        _ => panic!("expected flags"),
    }
}

/// Custom annotations are preserved through compilation into the IR.
#[test]
fn custom_annotations_preserved() {
    let source = r#"
@version("1.0.0")
namespace test.custom

@priority("Critical")
@routing("broadcast")
message Alert {
    code @0 : u32
}
"#;
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.expect("should compile");
    let alert_id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(alert_id).unwrap() {
        assert_eq!(msg.annotations.custom.len(), 2);
        assert_eq!(msg.annotations.custom[0].name.as_str(), "priority");
        assert_eq!(msg.annotations.custom[1].name.as_str(), "routing");
    } else {
        panic!("expected Message");
    }
}

#[test]
fn impl_extra_functions_rejected() {
    let schema = r#"
        namespace test.extra_fn
        
        trait Foo {
            fn method() -> u32
        }
        
        message Bar {
            value @0 : u32
        }
        
        impl Foo for Bar {
            fn method() -> u32
            fn extra_method() -> u64
        }
        "#;

    let result = vexil_lang::compile(schema);
    let has_error = result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    assert!(
        has_error,
        "expected error for extra impl function not in Foo"
    );
}

#[test]
fn external_function_rejected() {
    let schema = r#"
        namespace test.external_fn
        
        trait Timestamped {
            fn get_timestamp() -> u64
        }
        
        message Event {
            timestamp @0 : u64
        }
        
        impl Timestamped for Event {
            fn get_timestamp() -> u64
        }
        "#;

    let result = vexil_lang::compile(schema);
    let has_error = result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    assert!(has_error, "expected error for external function");
}

/// Generic trait conformance: type arguments are properly substituted
/// before comparing field types.
#[test]
fn generic_trait_impl_type_arg_substitution() {
    let schema = r#"
        namespace test.generic_impl
        
        trait Tagged<T> {
            tag @0 : T
            label @1 : string
        }
        
        message Event {
            tag @0 : u64
            label @1 : string
        }
        
        impl Tagged<u64> for Event { }
        "#;

    let result = vexil_lang::compile(schema);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no errors for valid generic impl, got: {errors:#?}"
    );
    assert!(result.compiled.is_some(), "expected CompiledSchema");
}

/// Generic trait impl with wrong type should fail conformance.
#[test]
fn generic_trait_impl_wrong_type_rejected() {
    let schema = r#"
        namespace test.wrong_type
        
        trait Tagged<T> {
            tag @0 : T
        }
        
        message Event {
            tag @0 : string  # wrong type - should be u64
        }
        
        impl Tagged<u64> for Event { }
        "#;

    let result = vexil_lang::compile(schema);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        !errors.is_empty(),
        "expected error for type mismatch in generic impl"
    );
}

/// Generic trait impl with type parameter in nested position.
#[test]
fn generic_trait_nested_type_parameter() {
    let schema = r#"
        namespace test.nested
        
        trait Container<T> {
            items @0 : array<T>
        }
        
        message EventList {
            items @0 : array<u64>
        }
        
        impl Container<u64> for EventList { }
        "#;

    let result = vexil_lang::compile(schema);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no errors for nested type param, got: {errors:#?}"
    );
}

/// Generic trait impl with multiple type parameters.
#[test]
fn generic_trait_multi_type_params() {
    let schema = r#"
        namespace test.multi
        
        trait Pair<A, B> {
            first @0 : A
            second @1 : B
        }
        
        message KeyValue {
            first @0 : u32
            second @1 : string
        }
        
        impl Pair<u32, string> for KeyValue { }
        "#;

    let result = vexil_lang::compile(schema);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no errors for multi-param generic impl, got: {errors:#?}"
    );
}
