# Development Setup

## Prerequisites

- Rust 1.94 or later
- Node.js 22.12+ for TypeScript targets and examples
- Go 1.22+ for the Go runtime and interop example
- Python 3.10+ for Python runtime work and repository checks

## Build the workspace

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
cargo build --workspace
```

## Rust baseline

```sh
cargo fmt --all
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

There is no repository pre-commit hook. Run formatting explicitly; CI checks it
without changing the contributor's staged files.

## Target suites

```sh
cd packages/runtime-ts
npm ci
npm run build
npm test

cd ../runtime-go
go test ./...

cd ../runtime-py
python -m pytest
```

Return to the repository root before running the curated examples:

```sh
python scripts/examples.py check all
```

## Generated output

Generator tests compare source against checked-in goldens. Regenerate only for
an intentional output change:

```sh
UPDATE_GOLDEN=1 cargo test -p vexil-codegen-rust
UPDATE_GOLDEN=1 cargo test -p vexil-codegen-ts
UPDATE_GOLDEN=1 cargo test -p vexil-codegen-go
UPDATE_GOLDEN=1 cargo test -p vexil-codegen-py
```

Inspect every generated diff. A passing updated snapshot is not evidence that
the new output is correct.

## Documentation

```sh
python scripts/check-doc-links.py
cd docs/book && mdbook build
```

## Benchmarks

`crates/vexil-bench` is excluded from the main workspace. Run its Criterion
benchmarks explicitly when performance is in scope:

```sh
cargo bench --manifest-path crates/vexil-bench/Cargo.toml
```

Read the root [contribution guide](https://github.com/vexil-lang/vexil/blob/main/CONTRIBUTING.md)
for change boundaries, contract tests, and pull-request expectations.
