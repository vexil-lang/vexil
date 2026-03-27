# vexil-codegen-rust

Rust code generation backend for the [Vexil](https://github.com/vexil-lang/vexil) schema compiler.

Takes a `CompiledSchema` from [`vexil-lang`](https://crates.io/crates/vexil-lang) and emits Rust structs, enums, and `Pack`/`Unpack` implementations. Generated code depends on [`vexil-runtime`](https://crates.io/crates/vexil-runtime) for bit-level I/O.

## Single file

```rust
use vexil_lang::compile;
use vexil_codegen_rust::generate;

let result = compile(source);
let compiled = result.compiled.expect("no errors");
let code: String = generate(&compiled)?;
```

## Multi-file project

```rust
use vexil_lang::{compile_project, codegen::CodegenBackend};
use vexil_codegen_rust::RustBackend;

let project = compile_project(&root_source, &root_path, &loader)?;
let files: Vec<(PathBuf, String)> = RustBackend.generate_project(&project)?;
// one .rs file per schema + a mod.rs with re-exports
```

## What gets generated

For each message: a struct with named fields, a `Pack` impl (encode), and an `Unpack` impl (decode). For enums: a Rust enum with `TryFrom<u*>`. For flags: a newtype over the backing integer with bitwise operations. For unions: a Rust enum with length-prefixed variants. For `@delta` messages: a stateful encoder/decoder pair that transmits field-level deltas.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
