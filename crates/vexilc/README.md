# vexilc

The [Vexil](https://github.com/vexil-lang/vexil) schema compiler CLI.

## Installation

```sh
cargo install vexilc
```

Or download a pre-built binary from the [releases page](https://github.com/vexil-lang/vexil/releases).

## Usage

**Check a schema for errors:**

```sh
vexilc check schema.vexil
```

**Generate Rust code from a schema:**

```sh
vexilc codegen schema.vexil --output out.rs
vexilc codegen schema.vexil --output out.rs --target rust
```

**Compile a multi-file project:**

```sh
vexilc build root.vexil --include ./schemas --output ./generated
```

On success, `check` prints the BLAKE3 schema hash. `build` writes one `.rs` file per
schema plus a `mod.rs` into the output directory.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
