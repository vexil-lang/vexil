# Vexil

Vexil is a schema language and toolchain for exact binary protocols. The schema
defines the data model and its representation: bit widths, field ordinals,
integer encodings, collection bounds, and evolution metadata are reviewable in
one contract.

<div class="vexil-start">
  <a href="getting-started/installation.html">Install vexilc</a>
  <a href="getting-started/first-schema.html">Write a schema</a>
  <a href="examples/quickstart.html">Run the quickstart</a>
</div>

> **Pre-1.0:** Vexil's components version independently. Read the
> [support matrix](getting-started/support-matrix.md) and
> [compatibility limits](getting-started/compatibility.md) before adoption.

## The defining choice

In many schema systems, a type describes a value while the codec decides how it
is represented. Vexil makes representation part of the type contract:

```text
message Reading {
    channel  @0 : u4
    value    @1 : u16
    sequence @2 : u32 @varint
    offset   @3 : i32 @zigzag
}
```

`channel` is four bits. `sequence` uses unsigned LEB128. `offset` uses ZigZag
followed by LEB128. Those are language rules rather than conventions hidden in
application code.

## What the toolchain provides

- a compiler with source-spanned diagnostics;
- Rust, TypeScript, Go, and Python code generation;
- deterministic canonical schema hashes using BLAKE3;
- compatibility classification for schema evolution;
- Rust, TypeScript, Go, and Python runtimes with different documented evidence
  levels;
- a conformance corpus and golden byte vectors.

The wire is not self-describing, and Vexil does not define transport,
authentication, discovery, or compression. Applications own those layers.

## A practical path

1. [Install the compiler](getting-started/installation.md).
2. [Write and check a schema](getting-started/first-schema.md).
3. [Generate code for your target](getting-started/generating-code.md).
4. [Run the curated examples](examples/quickstart.md).
5. Review the [support matrix](getting-started/support-matrix.md) for the exact
   target combination you intend to ship.
