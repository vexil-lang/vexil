# vexil-store

Human-readable (`.vx`) and binary (`.vxb`) file formats for Vexil schemas and data.

Part of the [Vexil](https://github.com/vexil-lang/vexil) project.

## Overview

`vexil-store` provides encode/decode for Vexil values against a compiled schema,
a human-readable text format (`.vx`), and a compact binary format (`.vxb`) with
a typed file header.

## Usage

```toml
[dependencies]
vexil-store = "0.1"
vexil-lang = "0.1"
```

```rust
use vexil_store::{encode, decode, meta_schema, Value};

let schema = meta_schema();
let value = Value::String("hello".to_string());
let bytes = encode(&value, "MyType", schema).unwrap();
let decoded = decode(&bytes, "MyType", schema).unwrap();
assert_eq!(value, decoded);
```

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
