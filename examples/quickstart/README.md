# Quickstart: Your First Wire Contract

Start here. This example turns one Vexil schema into a Rust codec, encodes a
compact device frame, prints the exact wire bytes, and decodes the frame again.

You will see the three ideas that define Vexil:

- `u4` occupies exactly four bits;
- `@varint` and `@zigzag` select the integer encoding in the schema;
- generated code carries the schema's canonical BLAKE3 hash.

## Prerequisites

- Rust 1.94 or later
- this repository checkout

## Run

From the repository root:

```sh
cargo run --manifest-path examples/quickstart/Cargo.toml
```

Expected result:

```text
schema: 4d4ca91b40a3bba1694d7426ac4352f5...
wire:   2a 00 00 00 02 00 00 2e 09 01 63 11 00 64 19 02 00 5f
round trip: device 42, 2 readings, battery 95%
```

The full schema hash is printed. The abbreviated value above keeps the guide
readable; the checked example verifies the complete round trip.

## Walkthrough

1. [`telemetry.vexil`](./telemetry.vexil) defines the wire contract.
2. [`src/generated.rs`](./src/generated.rs) is generated, never hand-edited.
3. [`src/main.rs`](./src/main.rs) constructs, encodes, and decodes a value.

## Regenerate

```sh
python scripts/examples.py regenerate quickstart
python scripts/examples.py check quickstart
```

Continue with [project evolution](../project-evolution/) when you are ready to
split schemas across files and check compatible changes.
