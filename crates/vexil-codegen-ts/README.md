# vexil-codegen-ts

TypeScript backend for the [Vexil](../../README.md) compiler. It emits typed
interfaces, encode/decode functions, schema constants, and stateful delta codecs
that use `@vexil-lang/runtime`.

```rust
use vexil_codegen_ts::TypeScriptBackend;
use vexil_lang::codegen::CodegenBackend;

let source = TypeScriptBackend.generate(&compiled)?;
let files = TypeScriptBackend.generate_project(&project)?;
```

```sh
vexilc codegen schema.vexil --target typescript --output generated.ts
vexilc build root.vexil --include schemas --target typescript --output generated
```

Project generation creates relative imports and namespace barrel files while
keeping each schema's hash and version distinct. Generated TypeScript is checked
with native TypeScript tooling and broad byte-vector coverage against Rust.

See the [TypeScript runtime guide](../../docs/book/src/runtime/typescript.md) and
[support matrix](../../docs/book/src/getting-started/support-matrix.md).

Licensed under either [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE), at your option.
