# vexil-lang

`vexil-lang` is the compiler library for the [Vexil](../../README.md) schema
language. It turns source text into diagnostics and a resolved `CompiledSchema`
for checking, hashing, compatibility analysis, and code generation.

## Compiler boundary

```text
source -> lexer -> parser -> source-faithful AST -> lowering -> resolved IR -> checks
```

The AST preserves source structure and spans. The IR resolves declarations and
imports for semantic checks and generators. `CompiledSchema` represents one
resolved schema; `ProjectResult` owns a topologically ordered multi-file result.

## Compile one schema

```rust
use vexil_lang::{compile, Severity};

let result = compile(source);
if result
    .diagnostics
    .iter()
    .any(|diagnostic| diagnostic.severity == Severity::Error)
{
    // Render or return the diagnostics.
}

if let Some(compiled) = result.compiled {
    let hash = vexil_lang::canonical::schema_hash(&compiled);
    println!("{hash:?}");
}
```

## Compile a project

```rust
use std::path::PathBuf;
use vexil_lang::{compile_project, resolve::FilesystemLoader};

let root_path = PathBuf::from("schemas/app/root.vexil");
let loader = FilesystemLoader::new(vec![PathBuf::from("schemas")]);
let project = compile_project(root_source, &root_path, &loader)?;
```

Backends implement the public `codegen::CodegenBackend` trait for single-schema
and project output. Backends own target-specific imports, names, and file layout.

## Contracts and stability

The language and wire specifications are authoritative over explanatory docs or
current implementation accidents. Tier 1 compiler APIs and generated wire
behavior are maintained as stable 0.x contracts; see the public API docs for the
current tier annotations.

- [Language specification](../../spec/language.md)
- [Wire-format specification](../../spec/wire-format.md)
- [Documentation book](../../docs/book/src/SUMMARY.md)
- [Compatibility limits](../../docs/limitations-and-gaps.md)

## License

Licensed under either [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE), at your option.
