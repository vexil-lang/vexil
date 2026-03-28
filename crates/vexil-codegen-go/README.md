# vexil-codegen-go

Go code generation backend for the [Vexil](https://github.com/vexil-lang/vexil) schema compiler.

Takes a `CompiledSchema` from [`vexil-lang`](https://crates.io/crates/vexil-lang) and emits Go structs, enums, and encode/decode functions. Generated code depends on [`github.com/vexil-lang/vexil/packages/runtime-go`](https://github.com/vexil-lang/vexil/tree/main/packages/runtime-go) for bit-level I/O.

## Single file

```rust
use vexil_lang::compile;
use vexil_codegen_go::generate;

let result = compile(source);
let compiled = result.compiled.expect("no errors");
let code: String = generate(&compiled)?;
```

## Multi-file project

```rust
use vexil_lang::{compile_project, codegen::CodegenBackend};
use vexil_codegen_go::GoBackend;

let project = compile_project(&root_source, &root_path, &loader)?;
let files: Vec<(PathBuf, String)> = GoBackend.generate_project(&project)?;
// one .go file per schema + package-level organization
```

## What gets generated

For each message: a Go struct with `Pack`/`Unpack` methods. For enums: typed constants with `iota`. For flags: a named integer type with bitwise helpers. For unions: an interface with per-variant concrete types. For `@delta` messages: a stateful encoder/decoder pair that transmits field-level deltas.

## CLI usage

```sh
vexilc codegen schema.vexil --output out.go --target go
vexilc build root.vexil --include ./schemas --output ./generated --target go
```

## Wire compatibility

Generated Go produces byte-identical output to the Rust and TypeScript backends. This is verified by the [compliance vector suite](../../compliance/vectors/).

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
