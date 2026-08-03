# Quickstart

The quickstart is the shortest complete Vexil path: check one schema, inspect
its canonical hash, generate a Rust codec, encode exact bytes, and decode them.

```sh
cargo run --manifest-path examples/quickstart/Cargo.toml
```

The schema uses a four-bit channel, a compact enum, unsigned LEB128, and ZigZag
encoding. The executable prints the complete schema hash and payload before
verifying the round trip.

Read the guided [example README](https://github.com/vexil-lang/vexil/tree/main/examples/quickstart)
for the file-by-file walkthrough and regeneration command.

Next: [Project Evolution](./project-evolution.md).
