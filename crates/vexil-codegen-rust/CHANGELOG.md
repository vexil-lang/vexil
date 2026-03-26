# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.2.0 (2026-03-27)

### Chore

 - <csr-id-1e063095980b21595d324edf57b99316ddd7f8f2/> rename vexil-codegen to vexil-codegen-rust
   Make room for vexil-codegen-ts (TypeScript backend) in a future
   milestone. Also add version fields to path dependencies for cargo
   publish compatibility.

### Documentation

 - <csr-id-b8e73670714e4abfd438d76dd305140fd4bd0a19/> add per-crate README files for crates.io
   Add README.md to each crate directory and wire readme.workspace = true
   so crates.io displays documentation for each published crate.

### New Features

 - <csr-id-49c386697379837fac4529f732095e524533cfe3/> wire cross-file import use statements in generate_project
 - <csr-id-a19d409d3f3170e019644d2b35c6cf0e662a54fb/> add RustBackend implementing CodegenBackend trait

### Bug Fixes

 - <csr-id-a2ea862c3a3873386fb24b1b0e8636e56bf66d02/> review fixes — Tier 1 re-exports, generate_with_imports visibility, span tier marker

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add per-crate README files for crates.io ([`b8e7367`](https://github.com/vexil-lang/vexil/commit/b8e73670714e4abfd438d76dd305140fd4bd0a19))
    - Wire cross-file import use statements in generate_project ([`49c3866`](https://github.com/vexil-lang/vexil/commit/49c386697379837fac4529f732095e524533cfe3))
    - Review fixes — Tier 1 re-exports, generate_with_imports visibility, span tier marker ([`a2ea862`](https://github.com/vexil-lang/vexil/commit/a2ea862c3a3873386fb24b1b0e8636e56bf66d02))
    - Add RustBackend implementing CodegenBackend trait ([`a19d409`](https://github.com/vexil-lang/vexil/commit/a19d409d3f3170e019644d2b35c6cf0e662a54fb))
    - Rename vexil-codegen to vexil-codegen-rust ([`1e06309`](https://github.com/vexil-lang/vexil/commit/1e063095980b21595d324edf57b99316ddd7f8f2))
</details>

