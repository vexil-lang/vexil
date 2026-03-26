# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.2.0 (2026-03-27)

### Documentation

 - <csr-id-b8e73670714e4abfd438d76dd305140fd4bd0a19/> add per-crate README files for crates.io
   Add README.md to each crate directory and wire readme.workspace = true
   so crates.io displays documentation for each published crate.

### New Features

 - <csr-id-f97b94fa748ccd75b3f13050f5f47ea4c71bbfa7/> annotation emission — doc, deprecated, since, non_exhaustive, tombstones
 - <csr-id-0a833d74137adf8c078e355201f1d60c94854862/> BitReader — LSB-first reader with LEB128, ZigZag, recursion depth tracking
 - <csr-id-32e99fd11e0f20d13cbb2c97af44b6702e066b8b/> BitWriter — LSB-first bit accumulator with LE integers, NaN canonicalization, LEB128
 - <csr-id-48349d7f0d6939f0666ffa3eeb27a288c3e9fc51/> LEB128 encode/decode + ZigZag mapping
 - <csr-id-5d0560e0b78574009c11881e7fff2cacf2ed6571/> scaffold crate — error types, Pack/Unpack traits, global limits

### Bug Fixes

 - <csr-id-16b9cd38cfb4db532c1f0b6c77e7a04a3129e118/> resolve 4 pre-existing codegen bugs found by compile_check
   - boxing.rs: detect direct Named cycles (mutual recursion like Expr/ExprKind)
   - message.rs: delta fields read/write with base encoding in standard Pack/Unpack
   - union_gen.rs: fix payload writer/reader redirect for Named type pack(w)/unpack(r)
   - union_gen.rs: add deref for primitive fields in union variant destructuring
   - config.rs: wrap Optional defaults in Some() when default is not None
   - vexil-runtime: add Pack/Unpack blanket impls for Box<T>

### Test

 - <csr-id-36aabd62fed3949fa170d02a8eccae6dc4268357/> wire format round-trip tests — sub-byte, optional, result, delta, union, LEB128

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 1 calendar day.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add per-crate README files for crates.io ([`b8e7367`](https://github.com/vexil-lang/vexil/commit/b8e73670714e4abfd438d76dd305140fd4bd0a19))
    - Resolve 4 pre-existing codegen bugs found by compile_check ([`16b9cd3`](https://github.com/vexil-lang/vexil/commit/16b9cd38cfb4db532c1f0b6c77e7a04a3129e118))
    - Annotation emission — doc, deprecated, since, non_exhaustive, tombstones ([`f97b94f`](https://github.com/vexil-lang/vexil/commit/f97b94fa748ccd75b3f13050f5f47ea4c71bbfa7))
    - Wire format round-trip tests — sub-byte, optional, result, delta, union, LEB128 ([`36aabd6`](https://github.com/vexil-lang/vexil/commit/36aabd62fed3949fa170d02a8eccae6dc4268357))
    - BitReader — LSB-first reader with LEB128, ZigZag, recursion depth tracking ([`0a833d7`](https://github.com/vexil-lang/vexil/commit/0a833d74137adf8c078e355201f1d60c94854862))
    - BitWriter — LSB-first bit accumulator with LE integers, NaN canonicalization, LEB128 ([`32e99fd`](https://github.com/vexil-lang/vexil/commit/32e99fd11e0f20d13cbb2c97af44b6702e066b8b))
    - LEB128 encode/decode + ZigZag mapping ([`48349d7`](https://github.com/vexil-lang/vexil/commit/48349d7f0d6939f0666ffa3eeb27a288c3e9fc51))
    - Scaffold crate — error types, Pack/Unpack traits, global limits ([`5d0560e`](https://github.com/vexil-lang/vexil/commit/5d0560e0b78574009c11881e7fff2cacf2ed6571))
</details>

