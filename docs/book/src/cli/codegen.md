# codegen

Generate code from a single Vexil schema file.

## Usage

```sh
vexilc codegen <file.vexil> [--target <target>] [--output <path>]
```

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `--target <target>` | `rust` | Code generation target: `rust`, `typescript`, or `go` |
| `--output <path>` | stdout | Write output to a file instead of stdout |

## Examples

```sh
# Generate Rust to stdout
vexilc codegen sensor.vexil

# Generate TypeScript to a file
vexilc codegen sensor.vexil --target typescript --output sensor.ts

# Generate Go
vexilc codegen sensor.vexil --target go --output sensor.go
```

## Notes

- For schemas with imports, use [`build`](./build.md) instead
- The generated code depends on the corresponding runtime library (`vexil-runtime` for Rust, `@vexil-lang/runtime` for TypeScript)
- Schema errors are reported before code generation begins
