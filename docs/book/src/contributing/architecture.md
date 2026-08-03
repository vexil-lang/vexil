# Architecture

Vexil separates source-oriented compiler data from the resolved model consumed
by checks and generators:

```text
source -> lexer -> parser -> AST -> lowering -> IR -> checks -> CompiledSchema
```

The AST keeps syntax and spans for diagnostics. The IR resolves declarations,
types, and imports. A `CompiledSchema` represents one resolved schema;
`ProjectResult` contains a multi-file project in topological order.

## Workspace boundaries

```text
vexil-lang
├── vexil-codegen-rust
├── vexil-codegen-ts
├── vexil-codegen-go
├── vexil-codegen-py
├── vexil-store
└── vexilc

vexil-runtime          Rust wire runtime
packages/runtime-ts    TypeScript wire runtime
packages/runtime-go    Go wire runtime
packages/runtime-py    Python wire runtime
```

Generators implement `vexil_lang::codegen::CodegenBackend`. Each backend owns
its target's imports, names, file layout, and runtime bindings. The compiler
does not impose a shared host-language layout.

## Contract flow

Language and wire specifications are authoritative. Corpus cases express
accepted and rejected schemas; compliance vectors express exact bytes. Compiler
and generator changes should update those contract fixtures when behavior
changes, then run the target's native checks.

See the [language specification](../../../../spec/language.md),
[wire-format specification](../../../../spec/wire-format.md), and
[contributor guide](../../../../CONTRIBUTING.md).
