# vexilc

CLI for the [Vexil](https://github.com/vexil-lang/vexil) schema compiler.

## Install

```sh
cargo install vexilc
```

Pre-built binaries: [Releases page](https://github.com/vexil-lang/vexil/releases).

## Commands

**check** -- validate a schema and print its BLAKE3 hash:

```sh
vexilc check schema.vexil
```

**codegen** -- generate code from a single schema:

```sh
vexilc codegen schema.vexil --output out.rs                     # Rust (default)
vexilc codegen schema.vexil --output out.ts --target typescript  # TypeScript
vexilc codegen schema.vexil --output out.go --target go          # Go
```

**build** -- compile a multi-file project with import resolution:

```sh
vexilc build root.vexil --include ./schemas --output ./generated
vexilc build root.vexil --include ./schemas --output ./generated --target typescript
```

Writes one file per schema plus a `mod.rs` (Rust) or `index.ts` (TypeScript).

**pack / unpack** -- convert between `.vx` text and `.vxb` binary:

```sh
vexilc pack  data.vx  --schema s.vexil --type T -o data.vxb
vexilc unpack data.vxb --schema s.vexil --type T
```

**format** -- pretty-print a `.vx` text file:

```sh
vexilc format data.vx --schema s.vexil --type T
```

**watch** -- auto-rebuild on schema changes:

```sh
vexilc watch root.vexil --include ./schemas --output ./generated
```

**init** -- scaffold a new schema file:

```sh
vexilc init my_schema.vexil --namespace my.namespace
```

**hash** -- print the BLAKE3 schema hash:

```sh
vexilc hash schema.vexil
```

**compat** -- compare schemas for breaking changes:

```sh
vexilc compat old.vexil new.vexil
```

**info** -- inspect a compiled schema or binary file:

```sh
vexilc info file.vxb
```

## Flags

- `--version` -- print the vexilc version
- `--help` -- print usage information

## Error output

Errors render with source spans via [ariadne](https://crates.io/crates/ariadne):

```
Error: duplicate field name
   --> schema.vexil:8:5
    |
  8 |     value: u32,
    |     ^^^^^ field "value" was already declared on line 5
```

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
