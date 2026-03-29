# Development Setup

## Prerequisites

- Rust 1.94 or later
- Node.js 18+ (for TypeScript runtime tests)
- Go 1.21+ (for Go runtime tests)

## Clone and build

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
cargo build --workspace
```

## Run tests

```sh
# All Rust tests (~500)
cargo test --workspace

# Core compiler only
cargo test -p vexil-lang

# Rust codegen + golden + compliance tests
cargo test -p vexil-codegen-rust

# TypeScript codegen + golden tests
cargo test -p vexil-codegen-ts

# TypeScript runtime tests (120)
cd packages/runtime-ts && npx vitest run
```

## Linting and formatting

```sh
# Must be clean (CI enforces this)
cargo clippy --workspace -- -D warnings

# Format all code
cargo fmt --all

# Check format without modifying
cargo fmt --all -- --check
```

## Golden files

Codegen tests compare output against golden files. To update after intentional changes:

```sh
UPDATE_GOLDEN=1 cargo test -p vexil-codegen-rust
UPDATE_GOLDEN=1 cargo test -p vexil-codegen-ts
```

## Benchmarks

```sh
cargo bench -p vexil-bench
```

## Pre-commit hook

The repo has a pre-commit hook that runs `cargo fmt --all` and re-stages formatted files. Commits are always formatted. Set `VEXIL_NO_FMT=1` to bypass.

## Git workflow

- Trunk-based development: small fixes go directly on main
- Milestone-sized features use `feature/<name>` branches merged via PR
- Always `git pull origin main` before starting work
- CI/release workflow changes must go on a branch
