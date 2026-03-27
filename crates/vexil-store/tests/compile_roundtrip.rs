//! Tests for the `compile` pipeline: CompiledSchema -> Value -> encode -> decode -> Value roundtrip.

use vexil_store::{compiled_schema_to_value, decode, encode, meta_schema, schema_store_to_value};

/// Compile a real schema, convert to Value, encode, decode, and verify equality.
#[test]
fn compile_roundtrip_simple_message() {
    let source = r#"
        namespace test.simple

        message Ping {
            id @0 : u32
            payload @1 : string
        }
    "#;
    let result = vexil_lang::compile(source);
    assert!(
        result.compiled.is_some(),
        "compilation failed: {:?}",
        result.diagnostics
    );
    let compiled = result.compiled.as_ref().unwrap();
    let meta = meta_schema();

    let value = compiled_schema_to_value(compiled);

    // Encode to bitpack bytes using the meta-schema
    let bytes = encode(&value, "CompiledSchema", meta).expect("encode should succeed");
    assert!(!bytes.is_empty());

    // Decode back
    let decoded = decode(&bytes, "CompiledSchema", meta).expect("decode should succeed");

    // Value roundtrip must be lossless
    assert_eq!(value, decoded);
}

/// Roundtrip with enum, flags, and union types.
#[test]
fn compile_roundtrip_complex_types() {
    let source = r#"
        namespace test.complex

        enum Color {
            Red @0
            Green @1
            Blue @2
        }

        flags Permissions {
            Read @0
            Write @1
            Execute @2
        }

        union Shape {
            Circle @0 {
                radius @0 : f64
            }
            Rect @1 {
                width @0 : f64
                height @1 : f64
            }
        }

        message Canvas {
            bg @0 : Color
            perms @1 : Permissions
            shape @2 : Shape
            label @3 : optional<string>
            tags @4 : array<string>
        }
    "#;
    let result = vexil_lang::compile(source);
    assert!(
        result.compiled.is_some(),
        "compilation failed: {:?}",
        result.diagnostics
    );
    let compiled = result.compiled.as_ref().unwrap();
    let meta = meta_schema();

    let value = compiled_schema_to_value(compiled);
    let bytes = encode(&value, "CompiledSchema", meta).expect("encode should succeed");
    let decoded = decode(&bytes, "CompiledSchema", meta).expect("decode should succeed");
    assert_eq!(value, decoded);
}

/// Roundtrip with newtype.
#[test]
fn compile_roundtrip_newtype() {
    let source = r#"
        namespace test.newtype

        newtype UserId : u64

        message User {
            id @0 : UserId
            name @1 : string
        }
    "#;
    let result = vexil_lang::compile(source);
    assert!(
        result.compiled.is_some(),
        "compilation failed: {:?}",
        result.diagnostics
    );
    let compiled = result.compiled.as_ref().unwrap();
    let meta = meta_schema();

    let value = compiled_schema_to_value(compiled);
    let bytes = encode(&value, "CompiledSchema", meta).expect("encode should succeed");
    let decoded = decode(&bytes, "CompiledSchema", meta).expect("decode should succeed");
    assert_eq!(value, decoded);
}

/// Roundtrip with sub-byte fields.
#[test]
fn compile_roundtrip_sub_byte() {
    let source = r#"
        namespace test.subbyte

        message Compact {
            flag @0 : u1
            nibble @1 : u4
            small @2 : i3
        }
    "#;
    let result = vexil_lang::compile(source);
    assert!(
        result.compiled.is_some(),
        "compilation failed: {:?}",
        result.diagnostics
    );
    let compiled = result.compiled.as_ref().unwrap();
    let meta = meta_schema();

    let value = compiled_schema_to_value(compiled);
    let bytes = encode(&value, "CompiledSchema", meta).expect("encode should succeed");
    let decoded = decode(&bytes, "CompiledSchema", meta).expect("decode should succeed");
    assert_eq!(value, decoded);
}

/// Roundtrip with map and result types.
#[test]
fn compile_roundtrip_map_result() {
    let source = r#"
        namespace test.containers

        message Config {
            env @0 : map<string, string>
            status @1 : result<u32, string>
        }
    "#;
    let result = vexil_lang::compile(source);
    assert!(
        result.compiled.is_some(),
        "compilation failed: {:?}",
        result.diagnostics
    );
    let compiled = result.compiled.as_ref().unwrap();
    let meta = meta_schema();

    let value = compiled_schema_to_value(compiled);
    let bytes = encode(&value, "CompiledSchema", meta).expect("encode should succeed");
    let decoded = decode(&bytes, "CompiledSchema", meta).expect("decode should succeed");
    assert_eq!(value, decoded);
}

/// SchemaStore roundtrip (multiple schemas packed into one .vxcp).
#[test]
fn compile_roundtrip_schema_store() {
    let src1 = r#"
        namespace test.one
        message Foo { x @0 : u32 }
    "#;
    let src2 = r#"
        namespace test.two
        enum Bar { A @0  B @1 }
    "#;
    let r1 = vexil_lang::compile(src1);
    let r2 = vexil_lang::compile(src2);
    let c1 = r1.compiled.as_ref().unwrap();
    let c2 = r2.compiled.as_ref().unwrap();
    let meta = meta_schema();

    let store_value = schema_store_to_value(&[c1, c2]);
    let bytes = encode(&store_value, "SchemaStore", meta).expect("encode should succeed");
    let decoded = decode(&bytes, "SchemaStore", meta).expect("decode should succeed");
    assert_eq!(store_value, decoded);
}

/// Full .vxc binary file roundtrip: compile -> convert -> encode -> write header -> read header -> decode -> compare.
#[test]
fn compile_roundtrip_vxc_binary() {
    let source = r#"
        namespace test.binary

        message Event {
            ts @0 : timestamp
            kind @1 : string
        }
    "#;
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let meta = meta_schema();

    let value = compiled_schema_to_value(compiled);
    let payload = encode(&value, "CompiledSchema", meta).unwrap();

    let meta_hash = vexil_lang::canonical::schema_hash(meta);
    let header = vexil_store::VxbHeader {
        magic: vexil_store::Magic::Vxc,
        format_version: vexil_store::FORMAT_VERSION,
        compressed: false,
        schema_hash: meta_hash,
        namespace: "vexil.schema".to_string(),
        schema_version: String::new(),
    };

    // Write binary
    let mut buf = Vec::new();
    vexil_store::write_header(&header, &mut buf);
    buf.extend_from_slice(&payload);

    // Read back
    let (read_header, offset) = vexil_store::read_header(&buf).expect("header read should succeed");
    assert_eq!(read_header.magic, vexil_store::Magic::Vxc);
    assert_eq!(read_header.namespace, "vexil.schema");
    assert_eq!(read_header.schema_hash, meta_hash);

    // Decode payload
    let decoded = decode(&buf[offset..], "CompiledSchema", meta).expect("decode should succeed");
    assert_eq!(value, decoded);
}

/// Verify that the meta-schema itself can be compiled and roundtripped.
#[test]
fn compile_roundtrip_meta_schema_self_hosting() {
    let meta = meta_schema();
    let value = compiled_schema_to_value(meta);
    let bytes = encode(&value, "CompiledSchema", meta).expect("encode meta-schema should succeed");
    let decoded =
        decode(&bytes, "CompiledSchema", meta).expect("decode meta-schema should succeed");
    assert_eq!(value, decoded);
}

/// Verify annotations (doc, deprecated, since, version) survive roundtrip.
#[test]
fn compile_roundtrip_annotations() {
    let source = r#"
        namespace test.annotated

        @doc "A test message"
        @doc "with multiple doc lines"
        message Documented {
            @doc "The identifier"
            id @0 : u32
        }
    "#;
    let result = vexil_lang::compile(source);
    assert!(
        result.compiled.is_some(),
        "compilation failed: {:?}",
        result.diagnostics
    );
    let compiled = result.compiled.as_ref().unwrap();
    let meta = meta_schema();

    let value = compiled_schema_to_value(compiled);
    let bytes = encode(&value, "CompiledSchema", meta).expect("encode should succeed");
    let decoded = decode(&bytes, "CompiledSchema", meta).expect("decode should succeed");
    assert_eq!(value, decoded);
}
