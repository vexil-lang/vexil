use std::collections::BTreeMap;
use vexil_lang::diagnostic::Severity;
use vexil_store::{
    decode, encode, format, meta_schema, parse, read_header, write_header, FormatOptions, Magic,
    Value, VxbHeader, FORMAT_VERSION,
};

fn compile_schema(source: &str) -> vexil_lang::CompiledSchema {
    let result = vexil_lang::compile(source);
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    assert!(!has_errors, "schema errors: {:?}", result.diagnostics);
    result.compiled.expect("schema should compile")
}

/// Test 1: Full pipeline .vx text -> Value -> encode -> bytes -> decode -> Value -> format -> text -> re-parse -> verify equality
#[test]
fn full_roundtrip_text_to_binary_to_text() {
    let schema = compile_schema(
        r#"
        namespace test.roundtrip
        enum Status { Active @0  Inactive @1 }
        message Item {
            id     @0 : u32
            name   @1 : string
            status @2 : Status
            tags   @3 : array<string>
            score  @4 : optional<f64>
        }
    "#,
    );

    // Build a Value
    let mut fields = BTreeMap::new();
    fields.insert("id".to_string(), Value::U32(42));
    fields.insert("name".to_string(), Value::String("widget".to_string()));
    fields.insert("status".to_string(), Value::Enum("Active".to_string()));
    fields.insert(
        "tags".to_string(),
        Value::Array(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ]),
    );
    fields.insert("score".to_string(), Value::Some(Box::new(Value::F64(9.5))));
    let original = Value::Message(fields);

    // Encode -> decode
    let bytes = encode(&original, "Item", &schema).expect("encode should succeed");
    let decoded = decode(&bytes, "Item", &schema).expect("decode should succeed");
    assert_eq!(original, decoded, "encode/decode roundtrip failed");

    // Format to .vx text
    let opts = FormatOptions {
        emit_schema_directive: true,
        ..Default::default()
    };
    let text = format(std::slice::from_ref(&decoded), "Item", &schema, &opts)
        .expect("format should succeed");
    assert!(text.contains("@schema"), "text should contain @schema");
    assert!(text.contains("42"), "text should contain id value");
    assert!(text.contains("widget"), "text should contain name value");
    assert!(
        text.contains("Active"),
        "text should contain status variant"
    );

    // Re-parse the formatted text
    let reparsed = parse(&text, &schema).expect("re-parse should succeed");
    assert_eq!(reparsed.len(), 1);
    assert_eq!(
        decoded, reparsed[0],
        "re-parsed value should equal original decoded"
    );
}

/// Test 2: Encode a Value, write binary file with header, read back, decode, verify.
#[test]
fn full_roundtrip_with_binary_file() {
    let schema = compile_schema(
        r#"
        namespace test.binfile
        message Point { x @0 : u32  y @1 : u32 }
    "#,
    );

    let mut fields = BTreeMap::new();
    fields.insert("x".to_string(), Value::U32(100));
    fields.insert("y".to_string(), Value::U32(200));
    let original = Value::Message(fields);

    // Encode
    let payload = encode(&original, "Point", &schema).expect("encode");

    // Build header
    let hash = vexil_lang::canonical::schema_hash(&schema);
    let header = VxbHeader {
        magic: Magic::Vxb,
        format_version: FORMAT_VERSION,
        compressed: false,
        schema_hash: hash,
        namespace: "test.binfile".to_string(),
        schema_version: "1.0.0".to_string(),
    };
    let mut file_bytes = Vec::new();
    write_header(&header, &mut file_bytes);
    file_bytes.extend_from_slice(&payload);

    // Read back
    let (decoded_header, header_size) = read_header(&file_bytes).expect("read_header");
    assert_eq!(decoded_header.namespace, "test.binfile");
    assert_eq!(decoded_header.magic, Magic::Vxb);

    let payload_back = &file_bytes[header_size..];
    let decoded = decode(payload_back, "Point", &schema).expect("decode");
    assert_eq!(original, decoded);
}

/// Test 3: Meta-schema self-hosting full pipeline — encode CompiledSchema as Value, decode, format, check output.
#[test]
fn meta_schema_self_hosting_full_pipeline() {
    let schema = meta_schema();

    // Build a simple CompiledSchema Value
    let annotations = Value::Message({
        let mut m = BTreeMap::new();
        m.insert("deprecated".to_string(), Value::None);
        m.insert("since".to_string(), Value::None);
        m.insert("doc".to_string(), Value::Array(Vec::new()));
        m.insert("revision".to_string(), Value::None);
        m.insert("non_exhaustive".to_string(), Value::Bool(false));
        m.insert("version".to_string(), Value::None);
        m
    });

    let compiled_schema_value = Value::Message({
        let mut m = BTreeMap::new();
        m.insert(
            "namespace".to_string(),
            Value::Array(vec![
                Value::String("meta".to_string()),
                Value::String("test".to_string()),
            ]),
        );
        m.insert("types".to_string(), Value::Array(Vec::new()));
        m.insert("declarations".to_string(), Value::Array(Vec::new()));
        m.insert("schema_hash".to_string(), Value::Hash([0u8; 32]));
        m.insert("annotations".to_string(), annotations);
        m
    });

    // Encode
    let bytes =
        encode(&compiled_schema_value, "CompiledSchema", schema).expect("encode CompiledSchema");

    // Decode
    let decoded = decode(&bytes, "CompiledSchema", schema).expect("decode CompiledSchema");
    assert_eq!(compiled_schema_value, decoded);

    // Format as .vx text
    let opts = FormatOptions {
        emit_schema_directive: false,
        ..Default::default()
    };
    let text = format(&[decoded], "CompiledSchema", schema, &opts).expect("format CompiledSchema");

    // Check that formatted text contains expected type names
    assert!(
        text.contains("CompiledSchema"),
        "text should mention CompiledSchema"
    );
    assert!(
        text.contains("namespace"),
        "text should mention namespace field"
    );
}
