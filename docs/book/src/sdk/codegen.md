# Writing a Codegen Backend

Vexil exposes code generation as a Rust API. Use it when an application needs
to generate a built-in target without starting `vexilc`, or when you are
building a target that lives outside the Vexil workspace.

This API is not a CLI plugin system. `vexilc` selects Rust, TypeScript, Go, and
Python through a closed match in the binary. A third-party backend is called by
your Rust program unless you maintain a custom `vexilc` build.

## Use a built-in backend

Each built-in backend crate exports a zero-sized backend value:

```rust
use vexil_codegen_rust::RustBackend;
use vexil_lang::{CodegenBackend, CodegenError};

fn generate_rust(source: &str) -> Result<Option<String>, CodegenError> {
    let result = vexil_lang::compile(source);
    if result.has_errors() {
        for diagnostic in result.errors() {
            eprintln!("{}", diagnostic.message);
        }
        return Ok(None);
    }

    match result.compiled {
        Some(compiled) => RustBackend.generate(&compiled).map(Some),
        None => Ok(None),
    }
}
```

The equivalent values are `TypeScriptBackend`, `GoBackend`, and
`PythonBackend` from their respective `vexil-codegen-*` crates. Use the trait
method rather than the backend crate's convenience function when the caller
needs to select a backend dynamically.

## Understand the two inputs

`CompiledSchema` is one resolved schema. Its `declarations` list contains only
types declared in that source file. Its registry can also contain imported
types, so iterating the complete registry would duplicate dependency output.

`ProjectResult` contains every compiled schema in dependency-first topological
order plus the combined diagnostics. Pass the complete project to
`generate_project`; the backend needs that context to produce imports and
module scaffolding.

Compilation can return IR alongside diagnostics. Treat any error-severity
diagnostic as a stop condition. Generating from a result with errors can turn a
useful compiler diagnostic into confusing or incomplete target output.

## Implement the trait

The complete compiling example lives in the
[`CodegenBackend` rustdoc](https://docs.rs/vexil-lang/latest/vexil_lang/codegen/trait.CodegenBackend.html).
Its shape is:

```rust
pub trait CodegenBackend {
    fn name(&self) -> &str;
    fn file_extension(&self) -> &str;
    fn generate(&self, schema: &CompiledSchema) -> Result<String, CodegenError>;
    fn generate_project(
        &self,
        project: &ProjectResult,
    ) -> Result<BTreeMap<PathBuf, String>, CodegenError>;
}
```

Before implementing it, answer these questions:

1. Which Vexil declarations, resolved types, and annotations does the target
   support?
2. How are authored names escaped, and what is the collision domain after
   escaping?
3. Which runtime package or generated support code owns wire operations?
4. How do imported types map to target-language imports?
5. Which relative file path belongs to each namespace, and which barrel or
   module files are required?
6. How will the backend reject unsupported constructs before emitting partial
   output?
7. Which golden, native compiler, and wire-vector checks prove the result?

## Output ownership and determinism

`generate` returns one source string. It does not choose a filename or write to
disk.

`generate_project` returns a `BTreeMap<PathBuf, String>`. Every path is relative
to the output directory chosen by the caller. Build that map with
`ProjectOutputBuilder`:

```rust
use vexil_lang::ProjectOutputBuilder;

let mut output = ProjectOutputBuilder::new();
output.add("demo/generated.rs", "// generated source\n")?;
let files = output.finish();
# Ok::<(), vexil_lang::OutputPathError>(())
```

The builder accepts a conservative portable path grammar: one or more ASCII
components containing letters, digits, `_`, `-`, or `.`, separated by either
path separator. It rejects roots and drive prefixes, `.` and `..`, repeated,
mixed, or trailing separators, Windows device names, and case-insensitive
collisions. This is intentionally narrower than any one host filesystem.

Existing backend implementations remain valid and are not deprecated. A custom
backend may continue returning a raw map, while a caller can apply
`validate_project_output` and write the canonical map it returns. The function
consumes the original map and reconstructs every accepted path using the host's
separator. Validation prevents lexical path escape. It does not follow symlinks
or junctions and does not make a sequence of filesystem writes transactional.

For identical compiler input and backend configuration, return identical paths
and bytes. Sort any data derived from hash maps or sets before emitting it. The
`BTreeMap` stabilizes file iteration, but it cannot make each file's contents
deterministic for you.

The caller owns directory creation, generated-output writes, overwrite policy,
atomic replacement, formatting, and cleanup of stale files. Keeping those
operations outside the backend makes generation testable without generated-file
side effects. A backend may still read an explicitly configured auxiliary
resource such as a template.

## Error model

Use the narrow shared variants when they describe the failure:

- `UnsupportedType` for a resolved type the target cannot represent.
- `MissingAnnotation` when the target contract requires explicit schema
  metadata.
- `Io` only when the backend itself performs necessary I/O. Most backends do
  not need it because callers write returned files.
- `BackendSpecific` for a typed target error such as an escaped-name collision
  or an unsupported target-language construct.

`ProjectOutputBuilder::add` returns `OutputPathError` directly. When a backend
uses `?` from `generate_project`, Vexil places that error in the existing
`CodegenError::BackendSpecific` variant. A caller can downcast the boxed error
to `OutputPathError` and use its stable `diagnostic_id`.

Validate the whole input before expensive emission where practical. On error,
return no project map. Do not make callers distinguish trustworthy files from
partial files.

## Project checklist

A project backend should test at least:

- a single schema with no imports;
- a direct import and a transitive import;
- a diamond dependency without duplicate output;
- authored aliases and target-name collisions;
- deterministic paths and file contents across repeated runs;
- an unsupported construct that returns an error and no partial project;
- target-native parsing or compilation of every generated file;
- applicable shared wire vectors when the backend emits codecs.

The built-in backends are useful implementation references, but the
[language specification](https://github.com/vexil-lang/vexil/blob/main/spec/language.md),
[wire-format specification](https://github.com/vexil-lang/vexil/blob/main/spec/wire-format.md),
corpus, and compliance vectors are the contract authorities.
