# vexil-codegen-rust

Rust code generation backend for the [Vexil](https://github.com/vexil-lang/vexil) schema compiler.

Takes a `CompiledSchema` from `vexil-lang` and emits Rust structs, enums, and `Pack`/`Unpack`
trait implementations that serialize to the Vexil wire format.

## Usage

### Single file

```rust
use vexil_lang::compile;
use vexil_codegen_rust::generate;

let result = compile(source);
let compiled = result.compiled.expect("no errors");
let code: String = generate(&compiled)?;
```

### Multi-file project

```rust
use vexil_lang::compile_project;
use vexil_codegen_rust::RustBackend;
use vexil_lang::codegen::CodegenBackend;

let project = compile_project(&root_source, &root_path, &loader)?;
let files: Vec<(PathBuf, String)> = RustBackend.generate_project(&project)?;
// files includes per-schema .rs files and a mod.rs with re-exports
```

Generated code depends on [`vexil-runtime`](https://crates.io/crates/vexil-runtime) for
bit-level I/O and the `Pack`/`Unpack` traits.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
