# vexil-codegen-rust

Rust backend for the [Vexil](../../README.md) compiler. It consumes a resolved
`CompiledSchema` or `ProjectResult` and emits Rust data types plus
`vexil-runtime` `Pack` and `Unpack` implementations.

```rust
use vexil_codegen_rust::RustBackend;
use vexil_lang::codegen::CodegenBackend;

let source = RustBackend.generate(&compiled)?;
let files = RustBackend.generate_project(&project)?;
```

The CLI exposes the same backend:

```sh
vexilc codegen schema.vexil --target rust --output generated.rs
vexilc build root.vexil --include schemas --target rust --output generated
```

Single-schema output includes the canonical schema hash, generated declarations,
wire implementations, and stateful delta helpers when requested by the schema.
Project output owns the Rust module tree and cross-schema imports.

Generated Rust is covered by golden output, native compilation, Clippy, and wire
tests. See the [support matrix](../../docs/book/src/getting-started/support-matrix.md)
for the current evidence boundary.

Licensed under either [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE), at your option.
