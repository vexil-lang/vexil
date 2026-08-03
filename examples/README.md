# Vexil Examples

The examples form one path from a first schema to a live multi-language
protocol. Each directory is a guided project, not a loose syntax gallery.

| Example | Level | Prerequisites | What it proves |
| --- | --- | --- | --- |
| [Quickstart](./quickstart/) | Start here | Rust | One schema can define exact bits, generate a codec, and round-trip a value. |
| [Project evolution](./project-evolution/) | Core workflow | Rust, Python | Imports generate a project tree and compatibility checks distinguish safe and breaking changes. |
| [Cross-language](./cross-language/) | Interop | Rust, Node, Go, Python | Four generated targets agree on one schema hash and one wire payload. |
| [Live telemetry](./live-telemetry/) | Flagship | Rust, Node, browser | Stateful delta frames move from a Rust service to a generated browser decoder. |

## Verify the suite

Install the Node dependencies used by the TypeScript examples once:

```sh
cd examples/cross-language/node-dashboard && npm ci
cd ../../live-telemetry && npm ci
cd ../..
```

Then run from the repository root:

```sh
python scripts/examples.py check all
```

Generated sources and the browser bundle are tracked so the examples are easy
to inspect. CI regenerates them into temporary directories and rejects drift.

## Regenerate after a schema or generator change

```sh
python scripts/examples.py regenerate all
python scripts/examples.py check all
```

Inspect every generated diff before committing it.
