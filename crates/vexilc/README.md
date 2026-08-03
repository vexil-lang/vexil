# vexilc

`vexilc` is the command-line compiler for [Vexil](../../README.md).

```sh
cargo install vexilc
vexilc --help
```

## Core workflow

```sh
# Validate and identify a schema
vexilc check schema.vexil
vexilc hash schema.vexil

# Generate one target
vexilc codegen schema.vexil --target rust --output generated.rs
vexilc codegen schema.vexil --target typescript --output generated.ts
vexilc codegen schema.vexil --target go --output generated.go
vexilc codegen schema.vexil --target python --output generated.py

# Resolve a multi-file project
vexilc build root.vexil --include schemas --target rust --output generated

# Classify an evolution
vexilc compat old.vexil new.vexil
```

## Data and schema tools

```sh
vexilc format value.vx --schema schema.vexil --type Message
vexilc pack value.vx --schema schema.vexil --type Message --output value.vxb
vexilc unpack value.vxb --schema schema.vexil --type Message
vexilc compile schema.vexil --output schema.vxc
vexilc info value.vxb
```

`watch` rebuilds a schema or project when source files change, and `init`
creates a starting schema. The complete options and exit-code behavior are in
the [CLI reference](../../docs/book/src/cli/overview.md).

The CLI does not define transport or deployment behavior. Generated target
support and current evidence are documented in the
[support matrix](../../docs/book/src/getting-started/support-matrix.md).

Licensed under either [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE), at your option.
