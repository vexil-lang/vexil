# vexil-codegen-ts

TypeScript code generation backend for the [Vexil](https://github.com/vexil-lang/vexil) schema compiler.

Takes a `CompiledSchema` from [`vexil-lang`](https://crates.io/crates/vexil-lang) and emits TypeScript interfaces, encode functions, and decode functions. Generated code depends on [`@vexil-lang/runtime`](https://www.npmjs.com/package/@vexil-lang/runtime) for bit-level I/O.

## Single file

```rust
use vexil_lang::compile;
use vexil_codegen_ts::generate;

let result = compile(source);
let compiled = result.compiled.expect("no errors");
let code: String = generate(&compiled)?;
```

## Multi-file project

```rust
use vexil_lang::{compile_project, codegen::CodegenBackend};
use vexil_codegen_ts::TypeScriptBackend;

let project = compile_project(&root_source, &root_path, &loader)?;
let files: Vec<(PathBuf, String)> = TypeScriptBackend.generate_project(&project)?;
// one .ts file per schema + barrel index.ts files for namespace directories
```

## What gets generated

For each message: a TypeScript interface, an `encode*` function, and a `decode*` function. Enums become string literal union types. Flags become numeric constants with bitwise helpers. Unions become discriminated unions. For `@delta` messages: a stateful encoder/decoder class pair.

Cross-file imports use relative paths. Namespace directories get barrel `index.ts` files that re-export their contents.

## Wire compatibility

Generated TypeScript produces byte-identical output to the Rust backend. This is verified by the [compliance vector suite](../../compliance/vectors/).

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
