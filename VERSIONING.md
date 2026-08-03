# Versioning Vexil

Vexil has two independent version axes:

1. specification generations describe the language and wire contract;
2. component versions describe individual crates and runtime packages.

They do not move in lockstep.

## Specification generations

[`spec/language.md`](spec/language.md) is currently a `1.0.0-draft` language
specification. [`spec/wire-format.md`](spec/wire-format.md) describes the
stabilizing first wire-format generation.

“1.0” here identifies the target contract generation. It is not a claim that a
compiler, generator, runtime, or the project as a whole has reached a final 1.0
release. The wire format has internal conformance evidence but no independent
implementation or external audit.

A future incompatible wire contract would require a new specification
generation. That decision is separate from the SemVer level of whichever
components implement it.

## Component versions

Each published component follows SemVer on its own public API and behavior:

| Component family | Distribution |
| --- | --- |
| Compiler, CLI, Rust runtime, store, and code generators | crates.io |
| TypeScript runtime | npm |
| Go runtime | versioned module tag and Go proxy |
| Python runtime | PyPI |

Different version numbers are expected. A low version is an honest statement
about that component's maturity and release history, not a synchronization bug.

When selecting dependencies, check the exact component version, its changelog,
the [support matrix](docs/book/src/getting-started/support-matrix.md), and the
[compatibility limits](FAQ.md#is-vexil-production-ready). Do not infer one
component's guarantees from another component or from the draft spec number.
