use std::collections::BTreeMap;
use vexil_store::{decode, encode, meta_schema, pack_schema, Value};

#[test]
fn meta_schema_compiles() {
    let compiled = vexil_lang::meta_schema();
    let ns: Vec<&str> = compiled
        .namespace
        .iter()
        .map(|s: &smol_str::SmolStr| s.as_str())
        .collect();
    assert_eq!(ns, vec!["vexil", "schema"]);
    assert!(compiled.registry.lookup("CompiledSchema").is_some());
    assert!(compiled.registry.lookup("TypeDef").is_some());
    assert!(compiled.registry.lookup("ResolvedType").is_some());
    assert!(compiled.registry.lookup("SchemaStore").is_some());
}

#[test]
fn pack_schema_compiles() {
    let compiled = vexil_lang::pack_schema();
    let ns: Vec<&str> = compiled
        .namespace
        .iter()
        .map(|s: &smol_str::SmolStr| s.as_str())
        .collect();
    assert_eq!(ns, vec!["vexil", "pack"]);
    assert!(compiled.registry.lookup("DataPack").is_some());
    assert!(compiled.registry.lookup("DataEntry").is_some());
}

#[test]
fn meta_schema_loads_via_api() {
    let schema = vexil_store::meta_schema();
    // namespace is Vec<SmolStr>
    let ns: Vec<&str> = schema.namespace.iter().map(|s| s.as_str()).collect();
    assert_eq!(ns, vec!["vexil", "schema"]);
    // Should return the same reference on repeated calls
    let schema2 = vexil_store::meta_schema();
    assert!(std::ptr::eq(schema, schema2));
}

#[test]
fn pack_schema_loads_via_api() {
    let schema = vexil_store::pack_schema();
    let ns: Vec<&str> = schema.namespace.iter().map(|s| s.as_str()).collect();
    assert_eq!(ns, vec!["vexil", "pack"]);
}

fn default_annotations() -> Value {
    Value::Message({
        let mut m = BTreeMap::new();
        m.insert("deprecated".to_string(), Value::None);
        m.insert("since".to_string(), Value::None);
        m.insert("doc".to_string(), Value::Array(Vec::new()));
        m.insert("revision".to_string(), Value::None);
        m.insert("non_exhaustive".to_string(), Value::Bool(false));
        m.insert("version".to_string(), Value::None);
        m
    })
}

fn make_simple_compiled_schema_value() -> Value {
    let ping_message = Value::Message({
        let mut m = BTreeMap::new();
        m.insert("name".to_string(), Value::String("Ping".to_string()));
        m.insert("fields".to_string(), Value::Array(Vec::new()));
        m.insert("tombstones".to_string(), Value::Array(Vec::new()));
        m.insert("annotations".to_string(), default_annotations());
        m
    });

    let type_def = Value::Union {
        variant: "Message".to_string(),
        fields: {
            let mut m = BTreeMap::new();
            m.insert("def".to_string(), ping_message);
            m
        },
    };

    Value::Message({
        let mut m = BTreeMap::new();
        m.insert(
            "namespace".to_string(),
            Value::Array(vec![
                Value::String("test".to_string()),
                Value::String("simple".to_string()),
            ]),
        );
        m.insert("types".to_string(), Value::Array(vec![type_def]));
        m.insert(
            "declarations".to_string(),
            Value::Array(vec![Value::U32(0)]),
        );
        m.insert("schema_hash".to_string(), Value::Hash([0u8; 32]));
        m.insert("annotations".to_string(), default_annotations());
        m
    })
}

#[test]
fn meta_schema_encode_decode_roundtrip() {
    let schema = meta_schema();
    let value = make_simple_compiled_schema_value();
    let bytes =
        encode(&value, "CompiledSchema", schema).expect("encoding CompiledSchema should succeed");
    let decoded =
        decode(&bytes, "CompiledSchema", schema).expect("decoding CompiledSchema should succeed");
    assert_eq!(value, decoded);
}

#[test]
fn meta_schema_encodes_schema_store() {
    let schema = meta_schema();
    let compiled = make_simple_compiled_schema_value();
    let store = Value::Message({
        let mut m = BTreeMap::new();
        m.insert("schemas".to_string(), Value::Array(vec![compiled]));
        m
    });
    let bytes = encode(&store, "SchemaStore", schema).unwrap();
    let decoded = decode(&bytes, "SchemaStore", schema).unwrap();
    assert_eq!(store, decoded);
}

#[test]
fn pack_schema_encode_decode_roundtrip() {
    let schema = pack_schema();
    let data_pack = Value::Message({
        let mut m = BTreeMap::new();
        m.insert(
            "entries".to_string(),
            Value::Array(vec![Value::Message({
                let mut e = BTreeMap::new();
                e.insert("schema_hash".to_string(), Value::Hash([1u8; 32]));
                e.insert("type_name".to_string(), Value::String("MyType".to_string()));
                e.insert("payload".to_string(), Value::Bytes(vec![0x01, 0x02, 0x03]));
                e
            })]),
        );
        m
    });
    let bytes = encode(&data_pack, "DataPack", schema).unwrap();
    let decoded = decode(&bytes, "DataPack", schema).unwrap();
    assert_eq!(data_pack, decoded);
}
