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

fn schema_hash(source: &str) -> Option<[u8; 32]> {
    let result = compile(source);
    if result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return None;
    }

    result
        .compiled
        .map(|compiled| vexil_lang::canonical::schema_hash(&compiled))
}
```

## Compile a project

```rust
use std::path::PathBuf;
use vexil_lang::project::ProjectError;
use vexil_lang::{compile_project, resolve::FilesystemLoader, ProjectResult};

fn compile_app(root_source: &str) -> Result<ProjectResult, ProjectError> {
    let root_path = PathBuf::from("schemas/app/root.vexil");
    let loader = FilesystemLoader::new([PathBuf::from("schemas")]);
    compile_project(root_source, &root_path, &loader)
}
```

## Generate code programmatically

Built-in backend crates expose zero-sized backend values that implement
`CodegenBackend`:

```rust
use vexil_codegen_rust::RustBackend;
use vexil_lang::{CodegenBackend, CodegenError};

fn generate_rust(source: &str) -> Result<Option<String>, CodegenError> {
    let result = vexil_lang::compile(source);
    if result.has_errors() {
        return Ok(None);
    }

    match result.compiled {
        Some(compiled) => RustBackend.generate(&compiled).map(Some),
        None => Ok(None),
    }
}
```

For projects, call `compile_project`, stop if `ProjectResult::diagnostics`
contains an error, then pass the whole result to `generate_project`. The backend
returns a `BTreeMap<PathBuf, String>` and owns target-specific imports, names,
relative output paths, and scaffolding such as `mod.rs` or `index.ts`. Your
application owns the final output directory and all file I/O.

Use `ProjectOutputBuilder` when implementing project generation. It rejects
rooted, traversing, non-portable, and case-colliding paths before returning a
map. Callers that accept maps from other implementations can use
`validate_project_output` and write the canonical map it returns.

Custom backends implement the same trait. They are programmatic extensions;
the `vexilc` CLI recognizes only its four built-in targets and does not load
third-party plugins. See [Writing a Codegen Backend](../../docs/book/src/sdk/codegen.md)
for the full contract, a compiling minimal backend, error guidance, and a test
checklist.

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
