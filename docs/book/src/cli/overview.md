# vexilc

`vexilc` is the Vexil schema compiler. It validates schemas, generates code for multiple target languages, and provides tools for schema evolution and binary file inspection.

## Usage

```
vexilc <subcommand> [args]
```

## Subcommands

| Command | Description |
|---------|-------------|
| [`check`](./check.md) | Validate a schema and print its hash |
| [`codegen`](./codegen.md) | Generate code for a single schema file |
| [`build`](./build.md) | Generate code for a multi-file project |
| [`watch`](./watch.md) | Watch files and rebuild on changes |
| [`compat`](./compat.md) | Compare schemas for breaking changes |
| [`init`](./init.md) | Create a new schema file |
| [`hash`](./hash.md) | Print the BLAKE3 schema hash |
| `pack` | Encode a `.vx` text file to `.vxb` binary |
| `unpack` | Decode a `.vxb` binary file to `.vx` text |
| `format` | Format a `.vx` text file |
| `info` | Inspect `.vxb`/`.vxc` file headers |
| `compile` | Compile a schema to `.vxc` binary format |

## Global options

| Option | Description |
|--------|-------------|
| `-V`, `--version` | Print version |
| `-h`, `--help` | Print help |

## Targets

The `--target` option (used by `codegen`, `build`, and `watch`) accepts:

- `rust` (default)
- `typescript`
- `go`
