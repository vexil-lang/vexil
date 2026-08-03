# vexil-codegen-py

Python backend for the [Vexil](../../README.md) compiler. It emits typed
dataclasses, encode/decode methods, schema constants, and stateful delta codecs
for the `vexil-runtime` Python package.

```rust
use vexil_codegen_py::PythonBackend;
use vexil_lang::codegen::CodegenBackend;

let source = PythonBackend.generate(&compiled)?;
let files = PythonBackend.generate_project(&project)?;
```

```sh
vexilc codegen schema.vexil --target python --output generated.py
vexilc build root.vexil --include schemas --target python --output generated
```

Generated Python is parsed, statically checked, and executed with the native
runtime over a representative shared wire matrix. It does not yet have the same
breadth of generated-code evidence as Rust and TypeScript.

The generator crate and Python runtime are separate packages. See the
[Python runtime guide](../../docs/book/src/runtime/python.md) for the current
installation boundary.

Licensed under either [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE), at your option.
