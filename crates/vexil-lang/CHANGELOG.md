# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.2.0 (2026-03-27)

### Chore

 - <csr-id-d41cf8a79bc4784811b8f7b26ff4375868472f60/> add repo governance, CI, and community health files
   - Add LICENSE-MIT and LICENSE-APACHE (dual MIT OR Apache-2.0)
   - Add README, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT, CHANGELOG
   - Add CI workflow (test + clippy + fmt, ubuntu + windows)
   - Add release workflow (vexilc binaries for Linux/Windows/macOS on v* tags)
   - Add Dependabot, CODEOWNERS, issue templates, PR template
   - Add .editorconfig and rust-toolchain.toml (stable + rustfmt + clippy)
   - Consolidate workspace package metadata into [workspace.package]
 - <csr-id-e6f05c55601e2fbf22e00e330fd025d8599a7a9b/> review fixes — #[non_exhaustive], remove unwrap, add safety comments

### Documentation

 - <csr-id-b8e73670714e4abfd438d76dd305140fd4bd0a19/> add per-crate README files for crates.io
   Add README.md to each crate directory and wire readme.workspace = true
   so crates.io displays documentation for each published crate.
 - <csr-id-582208ba536a89902befa5de67d32a6e0112f20e/> add stability tier annotations to all vexil-lang modules

### New Features

 - <csr-id-2bad0a676e9d670401f54cd071f35d18b322fd08/> schema-driven bitpack encoder/decoder with binary file format
   * chore: add .worktrees to .gitignore, document feature branch workflow in CLAUDE.md
   
   * feat(vexil-store): scaffold crate with Value enum and error types
 - <csr-id-0150b6489a9c85754851dab39d260f1218d3f257/> add CodegenBackend trait and CodegenError to vexil-lang
 - <csr-id-4518669b0769df5e3207688f72bd7383a9b686a5/> compile_project() multi-file orchestrator
 - <csr-id-caaa02e514b1a3c249fdc975f190169f1162af70/> name resolution precedence + wildcard collision detection
   Track wildcard import origins in LowerCtx and detect ambiguous type
   references when the same name is provided by multiple wildcard imports.
   Local declarations and named imports take precedence over wildcards.
 - <csr-id-ad904a6efa7d982cd7fc81bc9a440bbcf6811335/> dependency context injection in lowering phase
   Add DependencyContext struct and lower_with_deps() to support injecting
   real compiled types from dependencies during lowering, replacing stubs
   when dependency context is available.
 - <csr-id-2e9c86dd742986e14e06382183661b9b369d7974/> TypeId remapping for cross-file type injection
 - <csr-id-a7efaa4be6aa41c78e1c7e96b895d22631d4029e/> import graph builder with DFS cycle detection
   Adds project.rs with ImportGraph, ProjectError, and build_import_graph().
   DFS traversal discovers all transitive imports, detects cycles eagerly via
   stack tracking, deduplicates diamonds via visited set, and emits topo_order
   in dependency-first order. 4 tests: simple, direct cycle, transitive cycle,
   diamond.
 - <csr-id-cf2d475cc96e8951a7b6713a473a43405b16b3fe/> SchemaLoader trait + FilesystemLoader + InMemoryLoader
   Implements Task 2 of Milestone F: resolution abstraction layer.
   Adds LoadError enum, SchemaLoader trait, InMemoryLoader (pub schemas
   field for test access), and FilesystemLoader (multi-root search with
   Ambiguous detection). Five tests cover all success and failure paths.
 - <csr-id-8880b4cb339cf57344b4ef28a9440c43ac45bf53/> add source_file to Diagnostic for multi-file errors
 - <csr-id-741d86525242336d593a2e254c83c2697d34015f/> full canonical form — all 6 type kinds + spec fixes
   - Tombstones now emitted AFTER content (fields/variants/bits) in all 4 type kinds (Message, Enum, Flags, Union)
   - Enum backing type emitted as `: u8/u16/u32/u64` between name and `{`
   - Union variant fields now emit field-level annotations
   - Config fields sorted by name, emit field annotations, and emit `= {default}` values
   - Added emit_default_value() helper covering all DefaultValue variants
   - Added 8 new tests covering all 6 type kinds plus tombstone ordering
 - <csr-id-ca1acb119b082d5fa75145f012c07200f1c5eddf/> canonical annotation + encoding emission
   Add emit_annotations, emit_deprecated, emit_encoding, emit_encoding_inner,
   emit_tombstones, and emit_type_def. Wire them into canonical_form to emit
   schema-level annotations and all 6 declaration kinds (message, enum, flags,
   union, newtype, config) with annotations, encoding hints, and tombstones.
   Tests cover annotation sort order and all encoding modifiers (@varint,
   @zigzag, @delta, @limit).
 - <csr-id-319b6d408f2a867697d79806856d56b5062366c0/> canonical type string emission
   Add type_str, primitive_str, sub_byte_str, semantic_str, type_def_name
   helpers for mapping IR types to their canonical string representation per
   spec §6. Includes tests for all primitive, sub-byte, semantic, and
   parameterized type variants.
 - <csr-id-1effbf09d95bc106e9a41d998f3ccdea402a5564/> scaffold canonical module + blake3 dependency
 - <csr-id-01ce474afae2b1a10a4f0da30ba944124ca8fd3d/> EnumDef.wire_bits + FlagsDef.wire_bytes for codegen
   - Change EnumDef.backing from EnumBacking to Option<EnumBacking> so the
     IR can distinguish user-explicit backing (Some) from auto-sized (None)
   - Add EnumDef.wire_bits (u8): minimal bit width for auto-sized enums,
     explicit backing width otherwise; computed in typeck pass 1
   - Add FlagsDef.wire_bytes (u8): 1/2/4/8 bytes based on highest bit
     position; computed in typeck pass 1
   - Restructure typeck::check() so wire_bits/wire_bytes are set before
     message/union wire sizes (pass 2), ensuring named_type_wire_size
     sees correct values for embedded enum/flags fields
   - Update wire_size_enum and wire_size_flags tests to reflect auto-sized
     semantics; add wire_size_enum_explicit_backing test
   - Add 5 new tests: enum_wire_bits_exhaustive_no_backing,
     enum_wire_bits_non_exhaustive, enum_wire_bits_explicit_backing,
     flags_wire_bytes_low_bits, flags_wire_bytes_high_bits
 - <csr-id-d8ab6a03313dbce9a20c763e7bf53ba1b911f5f4/> DeprecatedInfo struct — split reason + since for codegen
 - <csr-id-1c61670783c7370aca13edc1c5ee508c691319bd/> type checker — wire size computation and recursive type detection
   Implements typeck::check(): detects infinite direct-recursive types via DFS
   with direct_path/visited sets, and computes WireSize for all Message/Union
   declarations using a computing-set cycle guard. Adds 7 wire size integration
   tests covering fixed, variable, optional, varint, enum, flags, and newtype.
 - <csr-id-f56343f5bf9ad31afd526c45cf9ac0937b6eccfc/> lowering pass and compile() API
   Implements lower::lower() mapping AST→IR for all six declaration kinds
   (message, enum, flags, union, newtype, config), type expression resolution
   with forward-reference and import-stub support, field encoding annotation
   extraction, and annotation lowering.  Adds compile() pipeline function
   (parse→validate→lower→typeck) with CompileResult.  All 18 valid corpus
   files compile without errors; 91 tests pass (89 pre-existing + 2 new).
   Also adds Default impl and is_empty() to TypeRegistry to satisfy clippy.
 - <csr-id-446c5e33337cc211b379a43d3df2486675838024/> IR type definitions and ErrorClass variants
 - <csr-id-d485a824217e784013ae8e457444c95510770625/> semantic validation — all 74 corpus tests passing
 - <csr-id-a38fb340bd2267413c0bc56115e11871a14684be/> union, newtype, config declaration parsing
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
 - <csr-id-3923d8406f00fc489dd0e31b6ac23db72ba3ccce/> type expression + literal value parsing
   Implement parse_type_expr() in parser/expr.rs supporting all type forms:
   primitives, sub-byte (u1-u63, i2-i63), semantic types (string, bytes,
   rgb, uuid, timestamp, hash), named/qualified types, and parameterized
   containers (optional, array, map, result) with recursive nesting.
   
   Also implement parse_literal_value() for config defaults and add
   LBracket/RBracket tokens to the lexer for array literal syntax.
 - <csr-id-03cfadfd8287bc29a7612d3cca1f88e5c9627185/> parser core + namespace parsing + first corpus tests
 - <csr-id-b60f318045f12c07dfa181dc13cda9cc09aed663/> AST node types — all declaration kinds, types, annotations
 - <csr-id-e3dde0523fd05cd879b0e800687b3addd412685a/> lexer — tokenizes all Vexil syntax
 - <csr-id-6565f1305d26d87717e37a869f27e47f7535d460/> span, diagnostic, and token type definitions
 - <csr-id-ebbe585e2ef31b467d5a7950ab8b83201df684c5/> workspace scaffold — vexil-lang lib + vexilc bin

### Bug Fixes

 - <csr-id-00f41a3fe4ccc791b170bcee7a127174b905c15f/> move schema files into vexil-lang crate for packaging; update README
   Move schemas/vexil/*.vexil into crates/vexil-lang/schemas/ so they are
   included in the crate package. Files outside the crate root are not
   included in `cargo package` tarballs, causing compile errors on install.
   
   Also add vexil-store to README repository structure.
 - <csr-id-a2ea862c3a3873386fb24b1b0e8636e56bf66d02/> review fixes — Tier 1 re-exports, generate_with_imports visibility, span tier marker
 - <csr-id-9c0b1cfd4a2c602d3e3582ca0d25eebc4e241230/> transitive type remapping + diamond dedup in clone_types_into
   clone_types_into now walks all ResolvedType fields to discover
   transitively referenced TypeIds before cloning. Types already in the
   target registry (diamond dependency dedup) are reused by name lookup.
   
   Diamond tests upgraded from enum to message at leaf level.
 - <csr-id-644c431e297acf2c12b843e70632dc6a2a757cad/> aliased import TypeId remapping for cross-references
   Use clone_types_into for aliased imports instead of per-type remap
   with empty map. Add TypeRegistry::rename() for alias qualification.
 - <csr-id-bd89fcfd4ce52c04b934e2c288e8a568a538f683/> reject schemas without namespace in import graph
   namespace_string() now returns Result and errors instead of silently
   returning an empty string, preventing HashMap key collisions.

### Test

 - <csr-id-ac36cd31fe61d2c57237082e56a61ca67cad9769/> canonical form whitespace invariance + hash stability
   Add whitespace_invariance and field_order_invariance tests to verify the
   canonical form is truly independent of source formatting and field order.
   Add pinned BLAKE3 hash stability tests for 8 corpus files (006–011, 013,
   016); 012_imports excluded (requires import resolution not yet implemented).
 - <csr-id-4e3633f126139e9a4b45c8d6be0dd5f013805a8d/> recursion detection, newtype chains, and schema annotations
 - <csr-id-fc46c4804bd0ac21843c95d71342c53bf9cd0bc6/> type resolution, encoding, import stubs, and invalid corpus tests
 - <csr-id-29b2b1b7eee7c443dffb0c86ba059be63d1731ab/> parser-detectable invalid corpus tests — all passing
   Add 13 new invalid corpus tests (17 total): duplicate namespace,
   namespace empty, invalid escape, namespace invalid component,
   decl name lowercase/underscore, field name uppercase, union/enum
   variant lowercase, config missing default/has ordinal, removed
   missing reason, version after namespace.
   
   Parser fixes for 4 initially-failing tests:
   - Detect duplicate namespace keyword
   - Reject underscores in UpperIdent declaration names
   - Don't consume @removed as post-type annotation on preceding field
   - Flag @version annotations appearing after namespace declaration
   - Move @version before namespace in valid 013_annotations corpus
 - <csr-id-0ca9752758384b7b6ad11b2f0b83a25528768c6b/> all 18 valid corpus tests passing

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 42 commits contributed to the release over the course of 1 calendar day.
 - 41 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Move schema files into vexil-lang crate for packaging; update README ([`00f41a3`](https://github.com/vexil-lang/vexil/commit/00f41a3fe4ccc791b170bcee7a127174b905c15f))
    - Schema-driven bitpack encoder/decoder with binary file format ([`2bad0a6`](https://github.com/vexil-lang/vexil/commit/2bad0a676e9d670401f54cd071f35d18b322fd08))
    - Add per-crate README files for crates.io ([`b8e7367`](https://github.com/vexil-lang/vexil/commit/b8e73670714e4abfd438d76dd305140fd4bd0a19))
    - Review fixes — Tier 1 re-exports, generate_with_imports visibility, span tier marker ([`a2ea862`](https://github.com/vexil-lang/vexil/commit/a2ea862c3a3873386fb24b1b0e8636e56bf66d02))
    - Add stability tier annotations to all vexil-lang modules ([`582208b`](https://github.com/vexil-lang/vexil/commit/582208ba536a89902befa5de67d32a6e0112f20e))
    - Add CodegenBackend trait and CodegenError to vexil-lang ([`0150b64`](https://github.com/vexil-lang/vexil/commit/0150b6489a9c85754851dab39d260f1218d3f257))
    - Transitive type remapping + diamond dedup in clone_types_into ([`9c0b1cf`](https://github.com/vexil-lang/vexil/commit/9c0b1cfd4a2c602d3e3582ca0d25eebc4e241230))
    - Compile_project() multi-file orchestrator ([`4518669`](https://github.com/vexil-lang/vexil/commit/4518669b0769df5e3207688f72bd7383a9b686a5))
    - Name resolution precedence + wildcard collision detection ([`caaa02e`](https://github.com/vexil-lang/vexil/commit/caaa02e514b1a3c249fdc975f190169f1162af70))
    - Aliased import TypeId remapping for cross-references ([`644c431`](https://github.com/vexil-lang/vexil/commit/644c431e297acf2c12b843e70632dc6a2a757cad))
    - Dependency context injection in lowering phase ([`ad904a6`](https://github.com/vexil-lang/vexil/commit/ad904a6efa7d982cd7fc81bc9a440bbcf6811335))
    - TypeId remapping for cross-file type injection ([`2e9c86d`](https://github.com/vexil-lang/vexil/commit/2e9c86dd742986e14e06382183661b9b369d7974))
    - Reject schemas without namespace in import graph ([`bd89fcf`](https://github.com/vexil-lang/vexil/commit/bd89fcfd4ce52c04b934e2c288e8a568a538f683))
    - Import graph builder with DFS cycle detection ([`a7efaa4`](https://github.com/vexil-lang/vexil/commit/a7efaa4be6aa41c78e1c7e96b895d22631d4029e))
    - SchemaLoader trait + FilesystemLoader + InMemoryLoader ([`cf2d475`](https://github.com/vexil-lang/vexil/commit/cf2d475cc96e8951a7b6713a473a43405b16b3fe))
    - Add source_file to Diagnostic for multi-file errors ([`8880b4c`](https://github.com/vexil-lang/vexil/commit/8880b4cb339cf57344b4ef28a9440c43ac45bf53))
    - Canonical form whitespace invariance + hash stability ([`ac36cd3`](https://github.com/vexil-lang/vexil/commit/ac36cd31fe61d2c57237082e56a61ca67cad9769))
    - Full canonical form — all 6 type kinds + spec fixes ([`741d865`](https://github.com/vexil-lang/vexil/commit/741d86525242336d593a2e254c83c2697d34015f))
    - Canonical annotation + encoding emission ([`ca1acb1`](https://github.com/vexil-lang/vexil/commit/ca1acb119b082d5fa75145f012c07200f1c5eddf))
    - Canonical type string emission ([`319b6d4`](https://github.com/vexil-lang/vexil/commit/319b6d408f2a867697d79806856d56b5062366c0))
    - Scaffold canonical module + blake3 dependency ([`1effbf0`](https://github.com/vexil-lang/vexil/commit/1effbf09d95bc106e9a41d998f3ccdea402a5564))
    - EnumDef.wire_bits + FlagsDef.wire_bytes for codegen ([`01ce474`](https://github.com/vexil-lang/vexil/commit/01ce474afae2b1a10a4f0da30ba944124ca8fd3d))
    - DeprecatedInfo struct — split reason + since for codegen ([`d8ab6a0`](https://github.com/vexil-lang/vexil/commit/d8ab6a03313dbce9a20c763e7bf53ba1b911f5f4))
    - Merge pull request #1 from vexil-lang/chore/repo-governance ([`9898d9a`](https://github.com/vexil-lang/vexil/commit/9898d9afedc7bd22393e00ac4119241b13ecbfae))
    - Add repo governance, CI, and community health files ([`d41cf8a`](https://github.com/vexil-lang/vexil/commit/d41cf8a79bc4784811b8f7b26ff4375868472f60))
    - Recursion detection, newtype chains, and schema annotations ([`4e3633f`](https://github.com/vexil-lang/vexil/commit/4e3633f126139e9a4b45c8d6be0dd5f013805a8d))
    - Type checker — wire size computation and recursive type detection ([`1c61670`](https://github.com/vexil-lang/vexil/commit/1c61670783c7370aca13edc1c5ee508c691319bd))
    - Type resolution, encoding, import stubs, and invalid corpus tests ([`fc46c48`](https://github.com/vexil-lang/vexil/commit/fc46c4804bd0ac21843c95d71342c53bf9cd0bc6))
    - Review fixes — #[non_exhaustive], remove unwrap, add safety comments ([`e6f05c5`](https://github.com/vexil-lang/vexil/commit/e6f05c55601e2fbf22e00e330fd025d8599a7a9b))
    - Lowering pass and compile() API ([`f56343f`](https://github.com/vexil-lang/vexil/commit/f56343f5bf9ad31afd526c45cf9ac0937b6eccfc))
    - IR type definitions and ErrorClass variants ([`446c5e3`](https://github.com/vexil-lang/vexil/commit/446c5e33337cc211b379a43d3df2486675838024))
    - Semantic validation — all 74 corpus tests passing ([`d485a82`](https://github.com/vexil-lang/vexil/commit/d485a824217e784013ae8e457444c95510770625))
    - Parser-detectable invalid corpus tests — all passing ([`29b2b1b`](https://github.com/vexil-lang/vexil/commit/29b2b1b7eee7c443dffb0c86ba059be63d1731ab))
    - All 18 valid corpus tests passing ([`0ca9752`](https://github.com/vexil-lang/vexil/commit/0ca9752758384b7b6ad11b2f0b83a25528768c6b))
    - Union, newtype, config declaration parsing ([`a38fb34`](https://github.com/vexil-lang/vexil/commit/a38fb340bd2267413c0bc56115e11871a14684be))
    - Message, enum, flags declaration parsing ([`a130532`](https://github.com/vexil-lang/vexil/commit/a1305325bd6c90d033ef4c3c8260b94dd963771f))
    - Type expression + literal value parsing ([`3923d84`](https://github.com/vexil-lang/vexil/commit/3923d8406f00fc489dd0e31b6ac23db72ba3ccce))
    - Parser core + namespace parsing + first corpus tests ([`03cfadf`](https://github.com/vexil-lang/vexil/commit/03cfadfd8287bc29a7612d3cca1f88e5c9627185))
    - AST node types — all declaration kinds, types, annotations ([`b60f318`](https://github.com/vexil-lang/vexil/commit/b60f318045f12c07dfa181dc13cda9cc09aed663))
    - Lexer — tokenizes all Vexil syntax ([`e3dde05`](https://github.com/vexil-lang/vexil/commit/e3dde0523fd05cd879b0e800687b3addd412685a))
    - Span, diagnostic, and token type definitions ([`6565f13`](https://github.com/vexil-lang/vexil/commit/6565f1305d26d87717e37a869f27e47f7535d460))
    - Workspace scaffold — vexil-lang lib + vexilc bin ([`ebbe585`](https://github.com/vexil-lang/vexil/commit/ebbe585e2ef31b467d5a7950ab8b83201df684c5))
</details>

