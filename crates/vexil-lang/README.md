# vexil-lang

Compiler library for [Vexil](https://github.com/vexil-lang/vexil), a typed schema definition language with first-class encoding semantics.

Parses `.vexil` source, type-checks it, and produces a `CompiledSchema` that codegen backends consume. Handles single-file and multi-file projects with transitive import resolution.

## Pipeline

```
source -> Lexer -> Parser -> AST -> Lower -> IR -> TypeCheck -> CompiledSchema
```

## Single-file compilation

```rust
use vexil_lang::{compile, Severity};

let result = compile(source);
if result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
    // diagnostics carry file, line, column, and a structured ErrorClass
}
if let Some(compiled) = result.compiled {
    let hash = vexil_lang::canonical::schema_hash(&compiled);
    // pass compiled to a CodegenBackend
}
```

## Multi-file projects

```rust
use vexil_lang::{compile_project, resolve::FilesystemLoader};

let loader = FilesystemLoader::new(vec!["./schemas".into()]);
let project = compile_project(&root_source, &root_path, &loader)?;
// project: Vec<(String, CompiledSchema)> in topological order
```

## Code generation

Pass a `CompiledSchema` or `ProjectResult` to any `CodegenBackend`:

```rust
use vexil_codegen_rust::RustBackend;
use vexil_lang::codegen::CodegenBackend;

// single file
let code: String = RustBackend.generate(&compiled)?;

// multi-file project
let files: Vec<(PathBuf, String)> = RustBackend.generate_project(&project)?;
```

## Workspace crates

| Crate | Role |
|-------|------|
| `vexil-lang` | This crate -- compiler library |
| [`vexil-codegen-rust`](https://crates.io/crates/vexil-codegen-rust) | Rust code generation |
| [`vexil-codegen-ts`](https://crates.io/crates/vexil-codegen-ts) | TypeScript code generation |
| [`vexil-runtime`](https://crates.io/crates/vexil-runtime) | Rust runtime for generated code |
| [`vexil-store`](https://crates.io/crates/vexil-store) | `.vx` text and `.vxb` binary file formats |
| [`vexilc`](https://crates.io/crates/vexilc) | CLI compiler |

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
