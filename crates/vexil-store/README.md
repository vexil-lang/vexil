# vexil-store

Schema-driven file formats for [Vexil](https://github.com/vexil-lang/vexil) data.

Encodes and decodes Vexil `Value`s against a compiled schema in two formats:
- `.vx` -- human-readable text, inspectable and diffable
- `.vxb` -- compact binary with a typed file header (magic bytes + schema hash)

## Usage

```toml
[dependencies]
vexil-store = "0.2"
vexil-lang = "0.2"
```

```rust
use vexil_store::{encode, decode, Value};

// encode a value against a compiled schema
let bytes = encode(&value, "SensorReading", &compiled)?;

// decode it back
let decoded = decode(&bytes, "SensorReading", &compiled)?;
assert_eq!(value, decoded);
```

## CLI

`vexilc` wraps this crate for command-line use:

```sh
vexilc pack  data.vx  --schema s.vexil --type T -o data.vxb  # text -> binary
vexilc unpack data.vxb --schema s.vexil --type T              # binary -> text
vexilc format data.vx  --schema s.vexil --type T              # pretty-print text
```

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
