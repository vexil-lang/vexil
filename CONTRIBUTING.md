# Contributing to Vexil

## Code of Conduct

This project follows the [Contributor Covenant](./CODE_OF_CONDUCT.md). By participating, you agree to uphold it.

## Ways to Contribute

- **Bug reports** — open an issue. Include the schema that triggers the bug and the error output.
- **Feature requests** — open an issue and explain the problem you're solving. "Add X" without context is hard to evaluate.
- **Spec corrections** — typos, ambiguities, or things that don't match the implementation. The spec is in `spec/vexil-spec.md`.
- **Corpus additions** — new test cases for `corpus/valid/` or `corpus/invalid/`. Add the `.vexil` file and a line in `corpus/MANIFEST.md`.
- **Documentation** — typos, better examples, things that are wrong. If something confused you, it'll confuse someone else.
- **Code** — see below.

## Development Setup

You need Rust 1.94 or later. Install via [rustup](https://rustup.rs).

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
git config core.hooksPath .githooks   # pre-commit fmt hook
cargo build --workspace
```

### Run Tests

```sh
cargo test --workspace
```

There are 540+ tests across 34 test suites. All 41 valid corpus schemas must compile and all 64 invalid schemas must produce errors. If you add a corpus file, update `corpus/MANIFEST.md`.

### Lint & Format

```sh
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

Both are enforced in CI. Run them before you push — CI failures for formatting are annoying for everyone.

## Making Changes

1. Fork and branch:
   - Features: `feat/description`
   - Bug fixes: `fix/description`
   - Spec/corpus: `spec/description` or `corpus/description`
2. Make your changes
3. `cargo test --workspace` — all pass
4. `cargo fmt --all` + `cargo clippy --workspace -- -D warnings` — clean
5. Push and open a PR

## Submitting a Pull Request

- One concern per PR. A PR that changes the parser AND adds a new type AND updates the spec is too much.
- Add tests. For compiler changes, add corpus files where possible — they're the easiest tests to maintain.
- Update `corpus/MANIFEST.md` if you add entries.
- For language features, wire format changes, or anything architectural: go through the RFC process in [GOVERNANCE.md](./GOVERNANCE.md). Don't just show up with a 2000-line PR changing the wire format.

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

- Formatted with `cargo fmt` (enforced in CI)
- Clippy lints are errors in CI (`-D warnings`)
- No `unwrap()` or `expect()` in non-test code — use `?` or explicit error handling
- All `unsafe` blocks need a `// SAFETY:` comment explaining the invariant
- Public API items in `vexil-lang` need doc comments
- `#[derive(Debug, Clone, PartialEq)]` on all data types unless there's a specific reason not to
