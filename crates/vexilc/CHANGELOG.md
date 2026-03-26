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
 - <csr-id-3656af5367c59ab587e01a4338d559b75eb28f19/> Bump ariadne from 0.5.1 to 0.6.0
   chore(deps): Bump ariadne from 0.5.1 to 0.6.0
 - <csr-id-b33326ee4556ad5cbe9c8ba322fe28b51f348674/> Bump ariadne from 0.5.1 to 0.6.0
   Bumps [ariadne](https://github.com/zesterer/ariadne) from 0.5.1 to 0.6.0.
   - [Changelog](https://github.com/zesterer/ariadne/blob/main/CHANGELOG.md)
   - [Commits](https://github.com/zesterer/ariadne/commits)
   
   ---
   updated-dependencies:
   - dependency-name: ariadne
     dependency-version: 0.6.0
     dependency-type: direct:production
     update-type: version-update:semver-minor
   ...
 - <csr-id-d41cf8a79bc4784811b8f7b26ff4375868472f60/> add repo governance, CI, and community health files
   - Add LICENSE-MIT and LICENSE-APACHE (dual MIT OR Apache-2.0)
   - Add README, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT, CHANGELOG
   - Add CI workflow (test + clippy + fmt, ubuntu + windows)
   - Add release workflow (vexilc binaries for Linux/Windows/macOS on v* tags)
   - Add Dependabot, CODEOWNERS, issue templates, PR template
   - Add .editorconfig and rust-toolchain.toml (stable + rustfmt + clippy)
   - Consolidate workspace package metadata into [workspace.package]

### Documentation

 - <csr-id-b8e73670714e4abfd438d76dd305140fd4bd0a19/> add per-crate README files for crates.io
   Add README.md to each crate directory and wire readme.workspace = true
   so crates.io displays documentation for each published crate.

### New Features

 - <csr-id-2bad0a676e9d670401f54cd071f35d18b322fd08/> schema-driven bitpack encoder/decoder with binary file format
   * chore: add .worktrees to .gitignore, document feature branch workflow in CLAUDE.md
   
   * feat(vexil-store): scaffold crate with Value enum and error types
 - <csr-id-7eb493fee26f72351903c674742ab02475e69ba0/> wire --target flag in codegen subcommand
 - <csr-id-b29320355c716a1ad392780d21a830c1f5331436/> update release workflow
 - <csr-id-eb4f7bdaa0bea087b3203d4784722d52a11279dc/> add --target flag to vexilc build, dispatch via CodegenBackend
 - <csr-id-8e22aae81a2652daf0dbae1dbf8e8ceb76536294/> add build command for multi-file compilation
 - <csr-id-788f9f038274ce2e375dba3ca018792f8e5ca578/> print schema hash on successful check
 - <csr-id-d958f51c7d05681cb4deba0c4815aa8ab28a5b99/> codegen subcommand — compile + generate Rust output
 - <csr-id-0bd48203bfb62c9c7f0e75cfde80f0a56743b4f0/> vexilc CLI with ariadne error rendering
 - <csr-id-a1305325bd6c90d033ef4c3c8260b94dd963771f/> message, enum, flags declaration parsing
   Implement declaration parsing in parser/decl.rs:
   - parse_type_decl() dispatches to message/enum/flags parsers
   - parse_message_decl() with field parsing (name, ordinal, type, annotations)
   - parse_enum_decl() with optional backing type and variant parsing
   - parse_flags_decl() with bit parsing
   - parse_tombstone() for @removed(...) with reason validation
   - Keywords accepted as namespace components (fixes test.message etc.)
   
   Replace skip_decl() placeholder in mod.rs with real parse_type_decl()
   calls. Add 7 corpus tests: 002-005 (type expressions) and 006-008
   (message, enum, flags declarations). All 28 tests pass.
 - <csr-id-ebbe585e2ef31b467d5a7950ab8b83201df684c5/> workspace scaffold — vexil-lang lib + vexilc bin

### Bug Fixes

 - <csr-id-6b0d833ebeff5f28d142e9a6486432b2e1af380f/> audit fixes — release.yml integrity, README accuracy, CHANGELOG ownership
   - Remove continue-on-error from vexil-runtime publish (blocks release on failure)
   - Set fail-fast: false in release test matrix
   - Align all release.yml checkouts to actions/checkout@v6
   - Add macos-latest to ci.yml test matrix
   - Fix README CLI usage (subcommands: check/codegen/build)
   - Fix README repo structure (add vexil-codegen-rust, vexil-runtime, corpus/projects)
   - Fix README library install (git path → semver "0.1")
   - Replace manual CHANGELOG entries with cliff-generated stub
   - Fix GOVERNANCE.md docs/decisions/ → GitHub issues labeled decision
   - Add keywords/categories to vexilc Cargo.toml

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release over the course of 1 calendar day.
 - 16 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Schema-driven bitpack encoder/decoder with binary file format ([`2bad0a6`](https://github.com/vexil-lang/vexil/commit/2bad0a676e9d670401f54cd071f35d18b322fd08))
    - Add per-crate README files for crates.io ([`b8e7367`](https://github.com/vexil-lang/vexil/commit/b8e73670714e4abfd438d76dd305140fd4bd0a19))
    - Audit fixes — release.yml integrity, README accuracy, CHANGELOG ownership ([`6b0d833`](https://github.com/vexil-lang/vexil/commit/6b0d833ebeff5f28d142e9a6486432b2e1af380f))
    - Wire --target flag in codegen subcommand ([`7eb493f`](https://github.com/vexil-lang/vexil/commit/7eb493fee26f72351903c674742ab02475e69ba0))
    - Update release workflow ([`b293203`](https://github.com/vexil-lang/vexil/commit/b29320355c716a1ad392780d21a830c1f5331436))
    - Add --target flag to vexilc build, dispatch via CodegenBackend ([`eb4f7bd`](https://github.com/vexil-lang/vexil/commit/eb4f7bdaa0bea087b3203d4784722d52a11279dc))
    - Rename vexil-codegen to vexil-codegen-rust ([`1e06309`](https://github.com/vexil-lang/vexil/commit/1e063095980b21595d324edf57b99316ddd7f8f2))
    - Add build command for multi-file compilation ([`8e22aae`](https://github.com/vexil-lang/vexil/commit/8e22aae81a2652daf0dbae1dbf8e8ceb76536294))
    - Print schema hash on successful check ([`788f9f0`](https://github.com/vexil-lang/vexil/commit/788f9f038274ce2e375dba3ca018792f8e5ca578))
    - Codegen subcommand — compile + generate Rust output ([`d958f51`](https://github.com/vexil-lang/vexil/commit/d958f51c7d05681cb4deba0c4815aa8ab28a5b99))
    - Bump ariadne from 0.5.1 to 0.6.0 ([`3656af5`](https://github.com/vexil-lang/vexil/commit/3656af5367c59ab587e01a4338d559b75eb28f19))
    - Merge pull request #1 from vexil-lang/chore/repo-governance ([`9898d9a`](https://github.com/vexil-lang/vexil/commit/9898d9afedc7bd22393e00ac4119241b13ecbfae))
    - Bump ariadne from 0.5.1 to 0.6.0 ([`b33326e`](https://github.com/vexil-lang/vexil/commit/b33326ee4556ad5cbe9c8ba322fe28b51f348674))
    - Add repo governance, CI, and community health files ([`d41cf8a`](https://github.com/vexil-lang/vexil/commit/d41cf8a79bc4784811b8f7b26ff4375868472f60))
    - Vexilc CLI with ariadne error rendering ([`0bd4820`](https://github.com/vexil-lang/vexil/commit/0bd48203bfb62c9c7f0e75cfde80f0a56743b4f0))
    - Message, enum, flags declaration parsing ([`a130532`](https://github.com/vexil-lang/vexil/commit/a1305325bd6c90d033ef4c3c8260b94dd963771f))
    - Workspace scaffold — vexil-lang lib + vexilc bin ([`ebbe585`](https://github.com/vexil-lang/vexil/commit/ebbe585e2ef31b467d5a7950ab8b83201df684c5))
</details>

