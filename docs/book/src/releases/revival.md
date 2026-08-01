# Revival Release Candidate

Vexil's next release is a focused 0.x stabilization release after a long pause.
It is not a 1.0 claim. The goal is a trustworthy compiler-to-runtime path,
clear package boundaries, and documentation that says exactly what is verified.

## Highlights

- Import version requirements are enforced as SemVer across direct and
  transitive project graphs.
- Concrete type aliases work for containers and imported named types while
  remaining transparent on the wire.
- Unsupported message invariants fail closed with a dedicated diagnostic.
- Result encoding retains the published `0 = Err`, `1 = Ok` contract.
- Unknown non-exhaustive union values preserve their discriminant and payload
  in Rust, TypeScript, Go, and Python.
- Generated-code checks compile and execute target output, including expanded
  Go and Python compliance coverage.
- The Python runtime is prepared for a first 0.1.0 PyPI publication through
  Trusted Publishing.

## Upgrade notes

Regenerate code with the matching `vexilc` release and run your target's native
test suite. If you use non-exhaustive unions, handle or retain the generated
unknown case. If a schema contains `invariant`, compilation now rejects it
instead of implying enforcement that did not exist.

Import requirements such as `@ ^0.5.0` now have effect. Give imported schemas
a valid schema-level `@version`; a missing version produces a warning and a
mismatch produces an error.

## Not in this release

This candidate does not add a new wire format, cross-field constraints, regex
constraints, invariant execution, RPC definitions, a standard library, a
transport profile, or encryption. These remain separately decidable work and
have no promised version number.

## Package plan

The intended release train is:

1. Release `vexil-lang` so generator packages can resolve the new compiler API.
2. Release generator crates in dependency order, publishing
   `vexil-codegen-py` to crates.io for the first time before `vexilc`.
3. Release `vexil-runtime`, `vexil-store`, and `vexilc` as their changes require.
4. Create the CLI GitHub Release from the `vexilc-v*` tag.
5. Publish Python 0.1.0 only after its PyPI Trusted Publisher is configured.

Tags, registry publication, GitHub Releases, and provider configuration remain
maintainer actions after the candidate diff and dry-run plans are reviewed.
