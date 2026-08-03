# Cross-Language Interop

The cross-language example asks Rust, TypeScript, Go, and Python to encode the
same fixture from generated code. Each target decodes its payload and prints a
machine-readable schema hash and hex value. The runner requires all four pairs
to match.

```sh
python scripts/examples.py check cross-language
```

The fixture covers sub-byte integers, an enum, floats, a string, and optional
coordinates. It is deliberately small enough to understand and strict enough
to fail on a real representation difference.

This example is representative evidence. The maintained generated-wire matrix
and target test suites cover more shapes, but no repository example establishes
compatibility for every schema, runtime version, or environment.

Read the guided [example README](https://github.com/vexil-lang/vexil/tree/main/examples/cross-language)
for prerequisites, generated files, and regeneration.

Next: [Live Telemetry](./live-telemetry.md).
