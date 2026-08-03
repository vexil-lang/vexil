# vexil-codegen-go

Go backend for the [Vexil](../../README.md) compiler. It emits Go declarations,
pack/unpack methods, schema constants, and stateful delta codecs that use the
versioned Go runtime module.

```rust
use vexil_codegen_go::GoBackend;
use vexil_lang::codegen::CodegenBackend;

let source = GoBackend.generate(&compiled)?;
let files = GoBackend.generate_project(&project)?;
```

```sh
vexilc codegen schema.vexil --target go --output generated.go
vexilc build root.vexil --include schemas --target go --output generated
```

Project generation owns Go packages, imports, and file layout. Generated Go and
the runtime execute a representative shared wire matrix; that evidence is not
exhaustive for every schema or deployment environment.

See the [Go runtime guide](../../docs/book/src/runtime/go.md) and
[support matrix](../../docs/book/src/getting-started/support-matrix.md).

Licensed under either [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE), at your option.
