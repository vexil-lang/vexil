# Compatibility and Limitations

Vexil's binary wire format hasn't changed since April 2026 and breaking it
would require a major version bump, while the language specification remains
a draft. Rust and TypeScript generated paths have cross-language byte-vector
coverage. Verify generated Go or Python output for the schemas and runtime
versions you intend to ship. The wire format has not yet been exercised by
external implementations or independently audited.

The maintained list of verified behavior, known limitations, and missing work
is available in the repository's [limitations and gaps document](https://github.com/vexil-lang/vexil/blob/main/docs/limitations-and-gaps.md).

For an adoption path, start with [installation](./installation.md), write a
[first schema](./first-schema.md), and then follow the target-specific
[generation guide](./generating-code.md).
