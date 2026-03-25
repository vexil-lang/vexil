<h1 align="center">Vexil</h1>
<p align="center"><em>A typed schema definition language with first-class encoding semantics.</em></p>

<p align="center">
  <a href="https://github.com/vexil-lang/vexil/actions/workflows/ci.yml">
    <img src="https://github.com/vexil-lang/vexil/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="License: MIT OR Apache-2.0">
  <img src="https://img.shields.io/badge/rust-1.80%2B-orange" alt="Rust 1.80+">
</p>

---

## What is Vexil?

Vexil (Validated Exchange Language) is a schema definition language (SDL) in the tradition of Protocol Buffers and Cap'n Proto, distinguished by two properties:

**Encoding semantics are part of the type system.** The type `u4` means exactly 4 bits on the wire — not "an integer that fits in 4 bits." The annotation `@varint` on a `u64` field changes the wire encoding to unsigned LEB128. The schema is the wire contract, not just the shape contract.

**The schema is the single source of truth.** Each schema has a deterministic BLAKE3 hash. That hash is embedded in generated code as a compile-time constant. A mismatch between the schema a sender compiled against and the schema a receiver compiled against is detectable at runtime, before any data corruption occurs.

## Features

- **Sub-byte integer types** — `u1`..`u7` and `i1`..`i7`; each occupies exactly N bits on the wire with LSB-first bit packing
- **Encoding annotations** — `@varint` (unsigned LEB128), `@zigzag` (ZigZag + LEB128), `@delta` (delta from previous value) directly in the schema
- **Rich type vocabulary** — `message`, `enum`, `flags`, `union`, `newtype`, and `config` declarations
- **Schema versioning** — BLAKE3 hash of the canonical schema form; mismatch is detectable at the protocol boundary
- **Structured error model** — every invalid input produces a distinct error class with file, line, column, and a human-readable description
- **74-file conformance corpus** — 18 valid schemas and 56 invalid schemas; a conformant implementation must accept all valid and reject all invalid

## Installation

### Pre-built binaries

Pre-built binaries for Linux, Windows, and macOS are available on the
[Releases page](https://github.com/vexil-lang/vexil/releases).

### From source

Requires Rust 1.80 or later ([install via rustup](https://rustup.rs)).

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
cargo build --release --bin vexilc
# Binary is at target/release/vexilc
```

## Usage

### CLI

Validate and compile a `.vexil` schema file:

```sh
vexilc schema.vexil
```

Errors are rendered with source spans and structured diagnostics:

```
Error: duplicate field name
   --> schema.vexil:8:5
    |
  8 |     value: u32,
    |     ^^^^^ field "value" was already declared on line 5
```

### Library

Add `vexil-lang` to your `Cargo.toml`:

```toml
[dependencies]
vexil-lang = { git = "https://github.com/vexil-lang/vexil" }
```

Parse and compile a schema programmatically:

```rust
let result = vexil_lang::compile(source);
if result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
    // handle errors
}
if let Some(compiled) = result.compiled {
    // use compiled schema
}
```

## Repository Structure

```
spec/
  vexil-spec.md        # Language specification (normative)
  vexil-grammar.peg    # Formal PEG grammar derived from spec
corpus/
  valid/               # 18 conformant schemas — all must be accepted
  invalid/             # 56 invalid schemas — all must be rejected
crates/
  vexil-lang/          # Compiler library: lexer, parser, IR, type checker
  vexilc/              # CLI frontend with ariadne error rendering
```

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a pull request.
For architectural decisions, language changes, and protocol modifications, see [GOVERNANCE.md](./GOVERNANCE.md).

## License

Licensed under either of

- [MIT License](./LICENSE-MIT)
- [Apache License, Version 2.0](./LICENSE-APACHE)

at your option.
