# vexil-store

Schema-driven value and file formats for [Vexil](../../README.md):

- `.vx` is human-readable, inspectable text;
- `.vxb` is a binary value container with a typed header and schema hash;
- compiled schema containers use the corresponding `.vxc`/project forms.

The crate validates values against a resolved `CompiledSchema` rather than
serializing arbitrary Rust memory.

```rust
use vexil_store::{decode, encode, Value};

let value = Value::U64(42);
let bytes = encode(&value, "Counter", &compiled)?;
let decoded = decode(&bytes, "Counter", &compiled)?;
assert_eq!(decoded, value);
```

The CLI exposes the common value workflow:

```sh
vexilc format value.vx --schema schema.vexil --type Counter
vexilc pack value.vx --schema schema.vexil --type Counter --output value.vxb
vexilc unpack value.vxb --schema schema.vexil --type Counter
```

The store format does not discover schemas or authenticate input. Applications
must obtain the expected schema through a trusted channel and apply their own
filesystem and resource policy.

Licensed under either [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE), at your option.
