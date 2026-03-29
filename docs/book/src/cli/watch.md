# watch

Watch for file changes and automatically rebuild.

## Usage

```sh
vexilc watch <root.vexil> [--include <dir>] [--output <dir>] [--target <target>]
```

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `--include <dir>` | (none) | Additional directories to watch and search for imports |
| `--output <dir>` | (none) | Output directory (if omitted, runs `check` only) |
| `--target <target>` | `rust` | Code generation target |

## Example

```sh
vexilc watch protocol.vexil --include ./schemas --output ./generated --target typescript
```

Output:

```
[watch] Initial build...
  wrote ./generated/protocol.ts
build complete: 2 schemas compiled
[watch] Ready. Watching for changes...
```

## Behavior

- Performs an initial build on startup
- Watches the root file's directory and all `--include` directories recursively
- Only reacts to `.vexil` file changes (creates and modifications)
- Debounces rapid changes with a 200ms delay
- If `--output` is omitted, runs `check` on each change instead of a full build
