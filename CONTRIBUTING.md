# Contributing to Vexil

Focused contributions are welcome: reproducible bugs, clearer documentation,
new conformance cases, and bounded implementation improvements all help.

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before opening a change

- For a bug, include the smallest schema and command that reproduce it.
- For an ambiguous or missing language rule, point to the relevant
  [language](spec/language.md) or [wire](spec/wire-format.md) section.
- For a language feature or wire change, open an RFC issue before implementation
  as described in [GOVERNANCE.md](GOVERNANCE.md).
- Keep one concern per pull request whenever practical.

Good first contributions include diagnostic corrections, documentation fixes,
focused regression tests, and corpus cases for already-defined behavior.

## Development setup

The Rust workspace requires Rust 1.94 or later. Optional target suites use
Node.js 22.12+, Go 1.22+, and Python 3.10+.

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
cargo build --workspace
```

There is no repository pre-commit hook. Formatting is an explicit command and
CI performs the non-mutating check.

## Validate your change

Run the narrowest relevant test while iterating, then the Rust baseline:

```sh
cargo fmt --all
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

Run target-specific suites when their runtime, generator, documentation, or
examples are affected:

```sh
cd packages/runtime-ts && npm ci && npm run build && npm test
cd packages/runtime-go && go test ./...
cd packages/runtime-py && python -m pytest
python scripts/examples.py check all
cd docs/book && mdbook build
```

The final two commands require their corresponding tools and dependencies.

## Tests that express the contract

- Use `corpus/valid/` and `corpus/invalid/` for language acceptance and
  rejection. Add the schema and its `corpus/MANIFEST.md` entry together.
- Use `compliance/vectors/` for byte-level wire contracts.
- Use focused unit or integration tests for local compiler, CLI, generator, and
  runtime behavior.
- Use target golden tests for intentional generated-source changes. Inspect the
  complete generated diff; do not update goldens simply to make a failure pass.

Avoid hard-coding corpus or test counts in documentation. They change as the
contract grows.

## Implementation expectations

- Preserve the source-faithful AST and resolved IR boundary.
- Do not use `unwrap()` or `expect()` in production code.
- Explain every `unsafe` block with a `// SAFETY:` comment.
- Give public `vexil-lang` APIs useful documentation.
- Do not add dependencies without a demonstrated need.
- Keep generated wire behavior stable unless an accepted contract decision says
  otherwise.

## Pull requests

Explain the problem, the chosen boundary, and the evidence you ran. Call out
skipped target suites or environmental limits directly. A local test run does
not prove hosted CI, package publication, or cross-environment compatibility.

Use [Conventional Commits](https://www.conventionalcommits.org/) for commit
subjects:

```text
feat(vexil-lang): add defined capability
fix(vexilc): preserve the reported source span
docs: clarify schema evolution guidance
test(codegen): cover a generated target contract
```

Common types are `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, and `perf`.
