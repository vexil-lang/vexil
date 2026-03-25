# Contributing to Vexil

Thank you for your interest in contributing! This document explains how to get
involved, what we expect, and how to get your changes merged.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Ways to Contribute](#ways-to-contribute)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Submitting a Pull Request](#submitting-a-pull-request)
- [Commit Messages](#commit-messages)
- [Code Style](#code-style)

---

## Code of Conduct

This project follows the [Contributor Covenant](./CODE_OF_CONDUCT.md).
By participating, you agree to uphold it.

## Ways to Contribute

- **Bug reports** — open an issue using the bug report template
- **Feature requests** — open an issue and describe the problem you're solving
- **Spec clarifications** — corrections or improvements to `spec/vexil-spec.md` or `spec/vexil-grammar.peg`
- **Corpus additions** — new test cases for `corpus/valid/` or `corpus/invalid/` with a corresponding `MANIFEST.md` entry
- **Documentation** — typos, clarifications, and examples are always welcome
- **Code** — see below for how to set up a dev environment

## Development Setup

### Prerequisites

- Rust 1.94 or later ([install via rustup](https://rustup.rs))
- `cargo` (included with Rust)

### Build

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
git config core.hooksPath .githooks   # enable the pre-push fmt hook
cargo build --workspace
```

### Run Tests

```sh
cargo test --workspace
```

The test suite includes corpus-driven tests. All 18 valid corpus schemas must
be accepted and all 56 invalid schemas must be rejected.

### Lint & Format

```sh
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

Both checks are enforced in CI. Run them locally before pushing.

## Making Changes

1. Fork the repository and create a branch:
   - Features: `feat/your-feature`
   - Bug fixes: `fix/your-fix`
   - Spec/corpus work: `spec/description` or `corpus/description`
2. Make your changes
3. Run `cargo test --workspace` and ensure all tests pass
4. Run `cargo fmt --all` and `cargo clippy --workspace -- -D warnings`
5. Push and open a pull request

## Submitting a Pull Request

- Keep PRs focused — one concern per PR
- Add tests for new behavior; for compiler changes, add corpus files where possible
- Update `corpus/MANIFEST.md` if you add corpus entries
- Fill in the PR template

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org):

```
feat(vexil-lang): add @packed annotation support
fix(vexilc): handle missing file argument gracefully
docs: clarify §3.4 encoding semantics in spec
test: add corpus case for duplicate config default
chore: bump thiserror to 2.1
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`

Scope is optional but useful: `vexil-lang`, `vexilc`, `spec`, `corpus`.

## Code Style

- Code is formatted with `cargo fmt` (enforced in CI)
- Clippy lints are treated as errors in CI (`-D warnings`)
- No `unwrap()` or `expect()` in non-test code — use `?` or explicit error handling
- All `unsafe` blocks require a `// SAFETY:` comment explaining the invariant
- Public API items in `vexil-lang` must have doc comments
- `#[derive(Debug, Clone, PartialEq)]` on all data types unless there is a specific reason not to
