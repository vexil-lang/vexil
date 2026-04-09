# vexil-lang

Compiler library for [Vexil](https://github.com/vexil-lang/vexil), a typed schema definition language with first-class encoding semantics.

Parses `.vexil` source, type-checks it, and produces a `CompiledSchema` that codegen backends consume. Handles single-file and multi-file projects with transitive import resolution. Includes the `compat` module for schema evolution and breaking change detection.

## Pipeline

```
source -> Lexer -> Parser -> AST -> Validate -> Lower -> IR -> TypeCheck -> CompiledSchema
```

## Type system (v1.0)

Vexil schemas support these declaration kinds:

| Kind | Example | Wire impact |
|------|---------|-------------|
| `message` | `message Point { x @0 : f32 y @1 : f32 }` | Field packing |
| `enum` | `enum Color : u8 { Red @0 Blue @1 }` | Compact ordinal |
| `flags` | `flags Perm : u8 { Read @0 Write @1 }` | Bit set |
| `union` | `union Shape { Circle @0 Rect @1 }` | Tagged variant |
| `newtype` | `newtype UserId = u64` | Transparent wrapper |
| `config` | `config Settings { debug @0 : bool = false }` | Compile-time only |
| `type` | `type Token = u64` | Transparent alias |
| `const` | `const MaxSize : u32 = 1024` | Compile-time only |

Primitive types: `bool`, `u8`–`u64`, `i8`–`i64`, `f32`, `f64`, `fixed32` (Q16.16), `fixed64` (Q32.32).

Parameterized types: `optional<T>`, `array<T>`, `array<T, N>`, `map<K,V>`, `result<T,E>`, `set<T>`.

Geometric types: `vec2<T>`, `vec3<T>`, `vec4<T>`, `quat<T>`, `mat3<T>`, `mat4<T>` (T = fixed32/fixed64/f32/f64).

Inline bitfields: `bits { r, w, x }`.

Constraint expressions: `field @0 : type where value > 0 && value < 100`.

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
| [`vexil-codegen-go`](https://crates.io/crates/vexil-codegen-go) | Go code generation |
| [`vexil-runtime`](https://crates.io/crates/vexil-runtime) | Rust runtime for generated code |
| [`vexil-store`](https://crates.io/crates/vexil-store) | `.vx` text and `.vxb` binary file formats |
| [`vexilc`](https://crates.io/crates/vexilc) | CLI compiler |

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
