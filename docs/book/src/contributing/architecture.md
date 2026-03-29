# Architecture

## Compiler pipeline

The Vexil compiler (`vexil-lang` crate) processes schemas through a multi-stage pipeline:

```
Source → Lexer → Parser → AST → Lower → IR → TypeCheck → Validate → CompiledSchema
```

**AST** is source-faithful -- it preserves spans, comments, and original syntax. This is used for error reporting and (future) LSP support.

**IR** is resolved -- types are looked up, ordinals are validated, and the structure is ready for type checking and code generation.

## Crate dependency graph

```
vexil-lang          (core compiler, zero internal deps)
├── vexil-codegen-rust   (Rust backend)
├── vexil-codegen-ts     (TypeScript backend)
├── vexil-codegen-go     (Go backend)
├── vexil-store          (binary file formats, depends on vexil-runtime)
└── vexilc               (CLI, depends on all of the above)

vexil-runtime       (Rust runtime, zero workspace deps)
@vexil-lang/runtime (TypeScript runtime, zero deps, separate npm package)
```

## Key types

| Type | Description |
|------|-------------|
| `CompiledSchema` | Single-file compilation result: registry, declarations, namespace, wire size |
| `ProjectResult` | Multi-file result: `Vec<(String, CompiledSchema)>` in topological order |
| `TypeRegistry` | Opaque type store indexed by `TypeId` |
| `TypeDef` | Sum type: Message, Enum, Flags, Union, Newtype, Config |
| `CodegenBackend` | Trait for pluggable code generation backends |

## SDK architecture

The `CodegenBackend` trait in `vexil-lang::codegen` defines the interface for code generation:

- `generate(&CompiledSchema) -> Result<String>` -- single file
- `generate_project(&ProjectResult) -> Result<Vec<(String, String)>>` -- multi-file

Each backend (Rust, TypeScript, Go) implements this trait. The CLI dispatches to the appropriate backend based on `--target`.

## API stability tiers

| Tier | Stability | Contents |
|------|-----------|----------|
| Tier 1 | Stable | `compile()`, IR types, `CodegenBackend` trait |
| Tier 2 | Semi-stable | AST types, individual pipeline stages |
| Tier 3 | Internal | Lexer, parser, canonical form internals |

## Build sequence

The project follows a spec-driven development approach:

1. Specification (normative language spec)
2. PEG grammar (derived from spec)
3. Corpus (valid/invalid test files with spec references)
4. Reference implementation (Rust workspace + TypeScript runtime)

## Source

- [Language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md)
- [PEG grammar](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-grammar.peg)
- [Corpus manifest](https://github.com/vexil-lang/vexil/blob/main/corpus/MANIFEST.md)
