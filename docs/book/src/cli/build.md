# build

Generate code for a multi-file Vexil project with imports.

## Usage

```sh
vexilc build <root.vexil> --include <dir> --output <dir> [--target <target>]
```

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `--include <dir>` | (none) | Directory to search for imported schemas (can be repeated) |
| `--output <dir>` | (required) | Output directory for generated code |
| `--target <target>` | `rust` | Code generation target: `rust`, `typescript`, or `go` |

## Example

```sh
vexilc build protocol.vexil \
  --include ./schemas \
  --output ./generated \
  --target rust
```

Output:

```
  wrote ./generated/common/types.rs
  wrote ./generated/protocol.rs
build complete: 3 schemas compiled
```

## How it works

1. Parses the root schema file
2. Discovers `import` statements and resolves them against `--include` directories
3. Compiles all schemas in topological order (dependencies before dependents)
4. Generates one output file per namespace, with cross-file references handled by the backend
5. Handles diamond dependencies by deduplicating shared imports
