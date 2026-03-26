# vexil-lang

Compiler library for the [Vexil](https://github.com/vexil-lang/vexil) schema definition language.

Vexil (Validated Exchange Language) is a typed schema SDL with first-class encoding semantics —
sub-byte integer types, `@varint`/`@zigzag`/`@delta` annotations, and BLAKE3 schema hashing for
wire-level mismatch detection.

## Pipeline

```
source → lexer → parser → AST → lowering → IR → type checker → CompiledSchema
```

## Usage

```rust
use vexil_lang::compile;

let result = compile(source);
if result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
    // diagnostics carry file, line, column, and a structured ErrorClass
}
if let Some(compiled) = result.compiled {
    let hash = vexil_lang::canonical::schema_hash(&compiled);
    // pass `compiled` to a CodegenBackend
}
```

For multi-file projects:

```rust
use vexil_lang::{compile_project, resolve::FilesystemLoader};

let loader = FilesystemLoader::new(vec!["./schemas".into()]);
let result = compile_project(&root_source, &root_path, &loader)?;
```

## Crates in this workspace

| Crate | Purpose |
|-------|---------|
| `vexil-lang` | This crate — compiler library |
| [`vexil-codegen-rust`](https://crates.io/crates/vexil-codegen-rust) | Rust code generation backend |
| [`vexil-runtime`](https://crates.io/crates/vexil-runtime) | Runtime support for generated code |
| [`vexilc`](https://crates.io/crates/vexilc) | CLI compiler |

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
