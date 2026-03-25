# Milestone D — Rust Codegen Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Rust code generation backend to the Vexil compiler — two new crates (`vexil-runtime`, `vexil-codegen`), IR prerequisites in `vexil-lang`, and a `codegen` subcommand in `vexilc`.

**Architecture:** `vexil-runtime` provides bit-level I/O primitives and `Pack`/`Unpack` traits. `vexil-codegen` walks the `CompiledSchema` IR and emits a single `.rs` file that `use`s `vexil_runtime`. The compiler pipeline is: `.vexil` source → `vexil_lang::compile()` → `CompiledSchema` → `vexil_codegen::generate()` → Rust source string.

**Tech Stack:** Rust (edition 2021, MSRV 1.80), `thiserror` for error types, `smol_str` in vexil-lang IR changes, `test-case` for parameterized tests.

**Spec:** `docs/superpowers/specs/2026-03-25-milestone-d-codegen-design.md`

---

## File Structure

### New crate: `crates/vexil-runtime/`

| File | Responsibility |
|---|---|
| `Cargo.toml` | Crate manifest — only external dep is `thiserror` |
| `src/lib.rs` | Re-exports, global limit constants |
| `src/bit_writer.rs` | `BitWriter` struct — accumulate bits LSB-first, flush to byte buffer |
| `src/bit_reader.rs` | `BitReader` struct — read bits LSB-first from byte slice, recursion tracking |
| `src/leb128.rs` | LEB128 encode/decode with byte count limits, overlong rejection |
| `src/zigzag.rs` | ZigZag mapping and reverse mapping |
| `src/traits.rs` | `Pack` and `Unpack` trait definitions |
| `src/error.rs` | `EncodeError` and `DecodeError` enums |

### New crate: `crates/vexil-codegen/`

| File | Responsibility |
|---|---|
| `Cargo.toml` | Crate manifest — depends on `vexil-lang`, `thiserror` |
| `src/lib.rs` | `pub fn generate()`, orchestration, `CodegenError` |
| `src/emit.rs` | Code emission helpers: `CodeWriter` (indentation, line management) |
| `src/types.rs` | `ResolvedType` → Rust type string mapping |
| `src/boxing.rs` | Cycle detection → `HashSet<(TypeId, usize)>` of fields needing `Box` |
| `src/message.rs` | Message struct + `Pack`/`Unpack` generation |
| `src/enum_gen.rs` | Enum type + `Pack`/`Unpack` generation |
| `src/flags.rs` | Flags struct + bitwise ops + `Pack`/`Unpack` generation |
| `src/union_gen.rs` | Union enum + `Pack`/`Unpack` generation |
| `src/newtype.rs` | Newtype wrapper + `Pack`/`Unpack` generation |
| `src/config.rs` | Config struct + `Default` generation |
| `src/delta.rs` | Stateful `Encoder`/`Decoder` struct generation |
| `src/annotations.rs` | `@doc`, `@deprecated`, `@since`, `@non_exhaustive`, `@type`/`@domain`/`@revision` as doc comments, tombstones |
| `tests/golden.rs` | Golden file test runner |
| `tests/golden/*.rs` | Expected codegen output snapshots |

### Modified files in `crates/vexil-lang/`

| File | Change |
|---|---|
| `src/ir/types.rs` | Add `DeprecatedInfo` struct; change `ResolvedAnnotations.deprecated` type; add `wire_bits` to `EnumDef`, `wire_bytes` to `FlagsDef` |
| `src/ir/mod.rs` | Add `DeprecatedInfo` to re-exports; add fields to `EnumDef`, `FlagsDef` |
| `src/lower.rs` | Extract `since` arg from `@deprecated` annotation into `DeprecatedInfo` |
| `src/typeck.rs` | Compute `wire_bits` for enums, `wire_bytes` for flags |
| `tests/compile.rs` | Update tests that check `deprecated` field |

### Modified files in `crates/vexilc/`

| File | Change |
|---|---|
| `Cargo.toml` | Add `vexil-codegen` dependency |
| `src/main.rs` | Add `codegen` subcommand with `--output` flag |

### Workspace

| File | Change |
|---|---|
| `Cargo.toml` | Add `vexil-runtime` and `vexil-codegen` to workspace members |

---

## Task 1: IR Prerequisites — `DeprecatedInfo` struct

**Files:**
- Modify: `crates/vexil-lang/src/ir/types.rs:154-162`
- Modify: `crates/vexil-lang/src/ir/mod.rs:4`
- Modify: `crates/vexil-lang/src/lower.rs:410-412`
- Modify: `crates/vexil-lang/tests/compile.rs` (any test checking `.deprecated`)

- [ ] **Step 1: Write failing test for DeprecatedInfo**

In `crates/vexil-lang/tests/compile.rs`, add a test that compiles a schema with `@deprecated(since: "1.0", reason: "use Foo")` and asserts the field's `annotations.deprecated` is `Some(DeprecatedInfo { reason: "use Foo".into(), since: Some("1.0".into()) })`.

```rust
#[test]
fn deprecated_info_has_since() {
    let src = r#"
        namespace test.deprecated
        message Old {
            @deprecated(since: "1.0", reason: "use New")
            name @0 : string
        }
    "#;
    let result = vexil_lang::compile(src);
    assert!(result.diagnostics.iter().all(|d| d.severity != Severity::Error));
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    let msg = match compiled.registry.get(id).unwrap() {
        TypeDef::Message(m) => m,
        _ => panic!("expected message"),
    };
    let dep = msg.fields[0].annotations.deprecated.as_ref().unwrap();
    assert_eq!(dep.reason.as_str(), "use New");
    assert_eq!(dep.since.as_deref(), Some("1.0"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vexil-lang --test compile deprecated_info_has_since`
Expected: FAIL — `DeprecatedInfo` type does not exist yet.

- [ ] **Step 3: Add `DeprecatedInfo` to IR types**

In `crates/vexil-lang/src/ir/types.rs`, add after `ResolvedAnnotations`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct DeprecatedInfo {
    pub reason: SmolStr,
    pub since: Option<SmolStr>,
}
```

Change `ResolvedAnnotations.deprecated` from `Option<SmolStr>` to `Option<DeprecatedInfo>`.

In `crates/vexil-lang/src/ir/mod.rs`, add `DeprecatedInfo` to the re-export line.

- [ ] **Step 4: Update `lower.rs` to populate `DeprecatedInfo`**

In `crates/vexil-lang/src/lower.rs`, change the `"deprecated"` arm (~line 410) to:

```rust
"deprecated" => {
    let reason = extract_string_arg(ann, "reason").unwrap_or_default();
    let since = extract_string_arg(ann, "since");
    result.deprecated = Some(ir::DeprecatedInfo { reason, since });
}
```

- [ ] **Step 5: Fix any existing tests that reference the old `deprecated` type**

Search `compile.rs` for any assertion on `.deprecated` that expects `Option<SmolStr>` and update to `Option<DeprecatedInfo>`.

- [ ] **Step 6: Run all tests**

Run: `cargo test -p vexil-lang`
Expected: All pass including the new `deprecated_info_has_since` test.

- [ ] **Step 7: Clippy + format**

Run: `cargo clippy -p vexil-lang --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: Clean.

- [ ] **Step 8: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-lang/src/ir/types.rs crates/vexil-lang/src/ir/mod.rs crates/vexil-lang/src/lower.rs crates/vexil-lang/tests/compile.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): DeprecatedInfo struct — split reason + since for codegen"
```

---

## Task 2: IR Prerequisites — `EnumDef.wire_bits` and `FlagsDef.wire_bytes`

**Files:**
- Modify: `crates/vexil-lang/src/ir/mod.rs:52-59` (EnumDef) and `crates/vexil-lang/src/ir/mod.rs:69-76` (FlagsDef)
- Modify: `crates/vexil-lang/src/lower.rs` (set initial values)
- Modify: `crates/vexil-lang/src/typeck.rs` (compute values)
- Modify: `crates/vexil-lang/tests/compile.rs`

- [ ] **Step 1: Write failing tests for wire_bits and wire_bytes**

In `crates/vexil-lang/tests/compile.rs`:

```rust
#[test]
fn enum_wire_bits_exhaustive_no_backing() {
    // Direction: 4 variants (0-3) → ceil(log2(4)) = 2 bits
    let src = r#"
        namespace test.wire
        enum Direction {
            North @0
            South @1
            East  @2
            West  @3
        }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Enum(e) => assert_eq!(e.wire_bits, 2),
        _ => panic!("expected enum"),
    }
}

#[test]
fn enum_wire_bits_non_exhaustive() {
    // Non-exhaustive with 4 variants → max(ceil(log2(4)), 8) = 8
    let src = r#"
        namespace test.wire
        @non_exhaustive
        enum Kind {
            A @0
            B @1
            C @2
            D @3
        }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Enum(e) => assert_eq!(e.wire_bits, 8),
        _ => panic!("expected enum"),
    }
}

#[test]
fn enum_wire_bits_explicit_backing() {
    let src = r#"
        namespace test.wire
        enum Status : u16 {
            Ok @0
            Err @1
        }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Enum(e) => assert_eq!(e.wire_bits, 16),
        _ => panic!("expected enum"),
    }
}

#[test]
fn flags_wire_bytes_low_bits() {
    // Bits 0-3 → 1 byte
    let src = r#"
        namespace test.wire
        flags Perms {
            Read @0
            Write @1
            Exec @2
            Del @3
        }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Flags(f) => assert_eq!(f.wire_bytes, 1),
        _ => panic!("expected flags"),
    }
}

#[test]
fn flags_wire_bytes_high_bits() {
    // Bit 32 → 8 bytes (u64)
    let src = r#"
        namespace test.wire
        flags Wide {
            Low @0
            High @32
        }
    "#;
    let result = vexil_lang::compile(src);
    let compiled = result.compiled.unwrap();
    let id = compiled.declarations[0];
    match compiled.registry.get(id).unwrap() {
        TypeDef::Flags(f) => assert_eq!(f.wire_bytes, 8),
        _ => panic!("expected flags"),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-lang --test compile enum_wire_bits`
Expected: FAIL — `wire_bits` field does not exist on `EnumDef`.

- [ ] **Step 3: Add fields to IR structs**

In `crates/vexil-lang/src/ir/mod.rs`, add to `EnumDef`:
```rust
pub wire_bits: u8,
```

Add to `FlagsDef`:
```rust
pub wire_bytes: u8,
```

In `crates/vexil-lang/src/lower.rs`, set `wire_bits: 0` and `wire_bytes: 0` as initial values wherever `EnumDef` and `FlagsDef` are constructed (these are placeholders, typeck fills them in).

- [ ] **Step 4: Compute wire_bits and wire_bytes in typeck**

In `crates/vexil-lang/src/typeck.rs`, add to the `check()` function after wire size computation:

```rust
// Compute enum wire_bits and flags wire_bytes.
for &id in &decl_ids {
    match compiled.registry.get(id) {
        Some(TypeDef::Enum(en)) => {
            let wire_bits = compute_enum_wire_bits(en);
            if let Some(TypeDef::Enum(en)) = compiled.registry.get_mut(id) {
                en.wire_bits = wire_bits;
            }
        }
        Some(TypeDef::Flags(fl)) => {
            let wire_bytes = compute_flags_wire_bytes(fl);
            if let Some(TypeDef::Flags(fl)) = compiled.registry.get_mut(id) {
                fl.wire_bytes = wire_bytes;
            }
        }
        _ => {}
    }
}
```

Add helper functions:

```rust
fn compute_enum_wire_bits(en: &EnumDef) -> u8 {
    // Explicit backing type → use backing width.
    if let Some(backing) = &en.backing {
        return match backing {
            EnumBacking::U8 => 8,
            EnumBacking::U16 => 16,
            EnumBacking::U32 => 32,
            EnumBacking::U64 => 64,
        };
    }
    // Auto-sized: ceil(log2(max_ordinal + 1)), min 1 (or min 8 if non-exhaustive).
    let max_ordinal = en.variants.iter().map(|v| v.ordinal).max().unwrap_or(0);
    let min_bits = if max_ordinal == 0 {
        1
    } else {
        let n = u64::from(max_ordinal) + 1;
        (64 - (n - 1).leading_zeros()) as u8
    };
    if en.annotations.non_exhaustive {
        std::cmp::max(min_bits, 8)
    } else {
        std::cmp::max(min_bits, 1)
    }
}

fn compute_flags_wire_bytes(fl: &FlagsDef) -> u8 {
    let max_bit = fl.bits.iter().map(|b| b.bit).max().unwrap_or(0);
    match max_bit {
        0..=7 => 1,
        8..=15 => 2,
        16..=31 => 4,
        _ => 8,
    }
}
```

**Required IR change for `backing`:** Change `EnumDef.backing` from `EnumBacking` to `Option<EnumBacking>`. In `lower.rs`, set `None` when no explicit backing is parsed, `Some(...)` when explicit. Update `typeck.rs` — the existing `named_type_wire_size` function that matches on `en.backing` must handle `Option`. Update any tests that reference `en.backing` directly.

- [ ] **Step 5: Run all tests**

Run: `cargo test -p vexil-lang`
Expected: All pass.

- [ ] **Step 6: Clippy + format**

Run: `cargo clippy -p vexil-lang --all-targets -- -D warnings && cargo fmt --all -- --check`

- [ ] **Step 7: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-lang/
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): EnumDef.wire_bits + FlagsDef.wire_bytes for codegen"
```

---

## Task 3: Scaffold `vexil-runtime` crate + error types

**Files:**
- Create: `crates/vexil-runtime/Cargo.toml`
- Create: `crates/vexil-runtime/src/lib.rs`
- Create: `crates/vexil-runtime/src/error.rs`
- Create: `crates/vexil-runtime/src/traits.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Create crate directory and Cargo.toml**

```toml
[package]
name = "vexil-runtime"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Runtime support for Vexil generated code — bit-level I/O, Pack/Unpack traits, wire encoding primitives"
keywords = ["schema", "serialization", "binary", "protocol"]
categories = ["encoding"]

[dependencies]
thiserror = "2"
```

Add `"crates/vexil-runtime"` to workspace `members` in root `Cargo.toml`.

- [ ] **Step 2: Create error types**

`crates/vexil-runtime/src/error.rs`:

```rust
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EncodeError {
    #[error("field `{field}`: value does not fit in {bits} bits")]
    ValueOutOfRange { field: &'static str, bits: u8 },
    #[error("field `{field}`: length {actual} exceeds limit {limit}")]
    LimitExceeded {
        field: &'static str,
        limit: u64,
        actual: u64,
    },
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DecodeError {
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("invalid UTF-8 in string field")]
    InvalidUtf8,
    #[error("invalid or overlong varint encoding")]
    InvalidVarint,
    #[error("field `{field}`: length {actual} exceeds limit {limit}")]
    LimitExceeded {
        field: &'static str,
        limit: u64,
        actual: u64,
    },
    #[error("unknown enum variant {value} for type `{type_name}`")]
    UnknownEnumVariant {
        type_name: &'static str,
        value: u64,
    },
    #[error("unknown union variant {discriminant} for type `{type_name}`")]
    UnknownUnionVariant {
        type_name: &'static str,
        discriminant: u64,
    },
    #[error("decoded removed field ordinal {ordinal} (removed in {removed_in}): {reason}")]
    RemovedField {
        ordinal: u16,
        removed_in: &'static str,
        reason: &'static str,
    },
    #[error("field `{field}`: {message}")]
    InvalidValue {
        field: &'static str,
        message: String,
    },
    #[error("recursive type nesting exceeded 64 levels")]
    RecursionLimitExceeded,
    #[error("schema hash mismatch")]
    SchemaMismatch {
        local: [u8; 32],
        remote: [u8; 32],
    },
}
```

- [ ] **Step 3: Create Pack/Unpack traits**

`crates/vexil-runtime/src/traits.rs`:

```rust
use crate::bit_reader::BitReader;
use crate::bit_writer::BitWriter;
use crate::error::{DecodeError, EncodeError};

pub trait Pack {
    fn pack(&self, writer: &mut BitWriter) -> Result<(), EncodeError>;
}

pub trait Unpack: Sized {
    fn unpack(reader: &mut BitReader<'_>) -> Result<Self, DecodeError>;
}
```

- [ ] **Step 4: Create lib.rs with re-exports and global limits**

`crates/vexil-runtime/src/lib.rs`:

```rust
pub mod bit_reader;
pub mod bit_writer;
pub mod error;
pub mod leb128;
pub mod traits;
pub mod zigzag;

pub use bit_reader::BitReader;
pub use bit_writer::BitWriter;
pub use error::{DecodeError, EncodeError};
pub use traits::{Pack, Unpack};

/// Maximum string/bytes length in bytes (2^26 = 67,108,864).
pub const MAX_BYTES_LENGTH: u64 = 1 << 26;
/// Maximum array/map element count (2^24 = 16,777,216).
pub const MAX_COLLECTION_COUNT: u64 = 1 << 24;
/// Maximum LEB128 bytes for length prefixes.
pub const MAX_LENGTH_PREFIX_BYTES: u8 = 4;
/// Maximum recursion depth for nested types.
pub const MAX_RECURSION_DEPTH: u32 = 64;
```

Create empty stub files for `bit_writer.rs`, `bit_reader.rs`, `leb128.rs`, `zigzag.rs` so `lib.rs` compiles. Each stub file has a placeholder struct or is just empty with a comment.

`crates/vexil-runtime/src/bit_writer.rs`:
```rust
/// Accumulates bits LSB-first, flushes to a byte buffer.
pub struct BitWriter {
    buf: Vec<u8>,
    current_byte: u8,
    bit_offset: u8,
}
```

`crates/vexil-runtime/src/bit_reader.rs`:
```rust
/// Reads bits LSB-first from a byte slice.
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_offset: u8,
    recursion_depth: u32,
}
```

`crates/vexil-runtime/src/leb128.rs`: empty file.
`crates/vexil-runtime/src/zigzag.rs`: empty file.

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p vexil-runtime`
Expected: Compiles (stubs only).

- [ ] **Step 6: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add Cargo.toml crates/vexil-runtime/
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-runtime): scaffold crate — error types, Pack/Unpack traits, global limits"
```

---

## Task 4: `vexil-runtime` — LEB128 + ZigZag

**Files:**
- Modify: `crates/vexil-runtime/src/leb128.rs`
- Modify: `crates/vexil-runtime/src/zigzag.rs`

- [ ] **Step 1: Write LEB128 tests**

In `crates/vexil-runtime/src/leb128.rs`, add `#[cfg(test)] mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_zero() {
        let mut buf = Vec::new();
        encode(&mut buf, 0);
        assert_eq!(buf, [0x00]);
    }

    #[test]
    fn encode_127() {
        let mut buf = Vec::new();
        encode(&mut buf, 127);
        assert_eq!(buf, [0x7F]);
    }

    #[test]
    fn encode_128() {
        let mut buf = Vec::new();
        encode(&mut buf, 128);
        assert_eq!(buf, [0x80, 0x01]);
    }

    #[test]
    fn encode_300() {
        let mut buf = Vec::new();
        encode(&mut buf, 300);
        assert_eq!(buf, [0xAC, 0x02]);
    }

    #[test]
    fn round_trip_max_u64() {
        let mut buf = Vec::new();
        encode(&mut buf, u64::MAX);
        let (val, consumed) = decode(&buf, 10).unwrap();
        assert_eq!(val, u64::MAX);
        assert_eq!(consumed, 10);
    }

    #[test]
    fn decode_max_4_bytes_limit() {
        // 2^28 - 1 = max value in 4 LEB128 bytes
        let mut buf = Vec::new();
        encode(&mut buf, (1 << 28) - 1);
        assert!(buf.len() <= 4);
        let (val, _) = decode(&buf, 4).unwrap();
        assert_eq!(val, (1 << 28) - 1);
    }

    #[test]
    fn decode_exceeds_max_bytes() {
        // 2^28 requires 5 LEB128 bytes, should fail with max_bytes=4
        let mut buf = Vec::new();
        encode(&mut buf, 1 << 28);
        assert!(decode(&buf, 4).is_err());
    }

    #[test]
    fn reject_overlong_encoding() {
        // 0 encoded as two bytes: [0x80, 0x00] — overlong
        let buf = [0x80, 0x00];
        assert!(decode(&buf, 10).is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-runtime leb128`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement LEB128 encode/decode**

In `crates/vexil-runtime/src/leb128.rs`:

```rust
use crate::error::DecodeError;

/// Encode a u64 as unsigned LEB128 into the buffer.
pub fn encode(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Decode an unsigned LEB128 value from `data`.
/// Returns `(value, bytes_consumed)`.
/// Rejects sequences longer than `max_bytes` and overlong encodings.
pub fn decode(data: &[u8], max_bytes: u8) -> Result<(u64, usize), DecodeError> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    let mut last_byte = 0u8;

    for (i, &byte) in data.iter().enumerate() {
        if i >= max_bytes as usize {
            return Err(DecodeError::InvalidVarint);
        }

        result |= u64::from(byte & 0x7F) << shift;
        last_byte = byte;
        shift += 7;

        if byte & 0x80 == 0 {
            // Reject overlong: if this isn't the first byte and the
            // byte is 0, trailing zero continuation bytes are unnecessary.
            if i > 0 && byte == 0 {
                return Err(DecodeError::InvalidVarint);
            }
            return Ok((result, i + 1));
        }
    }

    // Ran out of data without finding a terminating byte.
    Err(DecodeError::UnexpectedEof)
}
```

- [ ] **Step 4: Run LEB128 tests**

Run: `cargo test -p vexil-runtime leb128`
Expected: All pass.

- [ ] **Step 5: Write ZigZag tests**

In `crates/vexil-runtime/src/zigzag.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_mapping() {
        assert_eq!(zigzag_encode(0, 64), 0);
        assert_eq!(zigzag_encode(-1, 64), 1);
        assert_eq!(zigzag_encode(1, 64), 2);
        assert_eq!(zigzag_encode(-2, 64), 3);
        assert_eq!(zigzag_encode(2, 64), 4);
        assert_eq!(zigzag_encode(i64::MIN, 64), u64::MAX);
        assert_eq!(zigzag_encode(i64::MAX, 64), u64::MAX - 1);
    }

    #[test]
    fn round_trip_i32_range() {
        for &v in &[0i64, 1, -1, 127, -128, i32::MIN as i64, i32::MAX as i64] {
            let encoded = zigzag_encode(v, 32);
            let decoded = zigzag_decode(encoded);
            assert_eq!(decoded, v, "round-trip failed for {v}");
        }
    }

    #[test]
    fn encode_32bit_width() {
        // type_bits=32: (n << 1) ^ (n >> 31)
        assert_eq!(zigzag_encode(-1, 32), 1);
        assert_eq!(zigzag_encode(1, 32), 2);
    }
}
```

- [ ] **Step 6: Implement ZigZag**

In `crates/vexil-runtime/src/zigzag.rs`:

```rust
/// ZigZag-encode a signed integer. `type_bits` is the source integer
/// width (8, 16, 32, or 64) — used for the arithmetic shift.
pub fn zigzag_encode(n: i64, type_bits: u8) -> u64 {
    ((n << 1) ^ (n >> (u32::from(type_bits) - 1))) as u64
}

/// ZigZag-decode back to signed.
pub fn zigzag_decode(n: u64) -> i64 {
    ((n >> 1) as i64) ^ -((n & 1) as i64)
}
```

- [ ] **Step 7: Run all runtime tests**

Run: `cargo test -p vexil-runtime`
Expected: All pass.

- [ ] **Step 8: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-runtime/src/leb128.rs crates/vexil-runtime/src/zigzag.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-runtime): LEB128 encode/decode + ZigZag mapping"
```

---

## Task 5: `vexil-runtime` — BitWriter

**Files:**
- Modify: `crates/vexil-runtime/src/bit_writer.rs`

- [ ] **Step 1: Write BitWriter tests**

Add `#[cfg(test)] mod tests` in `bit_writer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_single_bit_true() {
        let mut w = BitWriter::new();
        w.write_bool(true);
        let buf = w.finish();
        assert_eq!(buf, [0x01]);
    }

    #[test]
    fn write_single_bit_false() {
        let mut w = BitWriter::new();
        w.write_bool(false);
        let buf = w.finish();
        assert_eq!(buf, [0x00]);
    }

    #[test]
    fn write_bits_lsb_first() {
        // Write 3 bits (value=5 = 0b101), then 5 bits (value=19 = 0b10011).
        // LSB-first: bits go [1,0,1, 1,1,0,0,1] = byte 0b10011_101 = 0x9D.
        let mut w = BitWriter::new();
        w.write_bits(5, 3);
        w.write_bits(19, 5);
        let buf = w.finish();
        assert_eq!(buf, [0x9D]);
    }

    #[test]
    fn write_bits_cross_byte_boundary() {
        // Write u3=5, u5=19, u6=42
        // Byte 0: [1,0,1, 1,1,0,0,1] = 0x9D (3+5 bits)
        // Byte 1: [0,1,0,1,0,1, 0,0] = 0x54 >> no, let me recalculate
        // Actually: u3=5 (101), u5=19 (10011), u6=42 (101010)
        // LSB-first packing:
        // Byte 0: bits 0-7: [1,0,1] from u3 + [1,1,0,0,1] from u5 = 10011_101 = 0x9D
        // Byte 1: bits 8-13: [0,1,0,1,0,1] from u6 + 2 padding zeros = 00_101010 = 0x2A
        let mut w = BitWriter::new();
        w.write_bits(5, 3);
        w.write_bits(19, 5);
        w.write_bits(42, 6);
        let buf = w.finish();
        assert_eq!(buf, [0x9D, 0x2A]);
    }

    #[test]
    fn flush_to_byte_boundary_pads_zeros() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3);
        w.flush_to_byte_boundary();
        w.write_bits(0xFF, 8);
        let buf = w.finish();
        // First byte: 3 bits + 5 zero-pad = 0b00000_101 = 0x05
        // Second byte: 0xFF
        assert_eq!(buf, [0x05, 0xFF]);
    }

    #[test]
    fn write_u8_flushes_first() {
        let mut w = BitWriter::new();
        w.write_bool(true); // 1 bit
        w.write_u8(0xAB);
        let buf = w.finish();
        // Byte 0: 1 bit + 7 zero-pad = 0x01
        // Byte 1: 0xAB
        assert_eq!(buf, [0x01, 0xAB]);
    }

    #[test]
    fn write_u16_le() {
        let mut w = BitWriter::new();
        w.write_u16(0x0102);
        let buf = w.finish();
        assert_eq!(buf, [0x02, 0x01]); // little-endian
    }

    #[test]
    fn write_u32_le() {
        let mut w = BitWriter::new();
        w.write_u32(0x01020304);
        let buf = w.finish();
        assert_eq!(buf, [0x04, 0x03, 0x02, 0x01]);
    }

    #[test]
    fn write_i16_negative() {
        let mut w = BitWriter::new();
        w.write_i16(-1);
        let buf = w.finish();
        assert_eq!(buf, [0xFF, 0xFF]);
    }

    #[test]
    fn write_f32_nan_canonicalized() {
        let mut w = BitWriter::new();
        w.write_f32(f32::NAN);
        let buf = w.finish();
        // Canonical NaN = 0x7FC00000 in LE = [0x00, 0x00, 0xC0, 0x7F]
        assert_eq!(buf, [0x00, 0x00, 0xC0, 0x7F]);
    }

    #[test]
    fn write_f64_nan_canonicalized() {
        let mut w = BitWriter::new();
        w.write_f64(f64::NAN);
        let buf = w.finish();
        let expected = 0x7FF8000000000000u64.to_le_bytes();
        assert_eq!(buf, expected);
    }

    #[test]
    fn write_f32_negative_zero_preserved() {
        let mut w = BitWriter::new();
        w.write_f32(-0.0f32);
        let buf = w.finish();
        assert_eq!(buf, (-0.0f32).to_le_bytes());
        assert_ne!(buf, 0.0f32.to_le_bytes());
    }

    #[test]
    fn write_leb128() {
        let mut w = BitWriter::new();
        w.write_leb128(300);
        let buf = w.finish();
        assert_eq!(buf, [0xAC, 0x02]);
    }

    #[test]
    fn write_zigzag_negative_one() {
        let mut w = BitWriter::new();
        w.write_zigzag(-1, 64);
        let buf = w.finish();
        // ZigZag(-1) = 1, LEB128(1) = [0x01]
        assert_eq!(buf, [0x01]);
    }

    #[test]
    fn write_string() {
        let mut w = BitWriter::new();
        w.write_string("hi");
        let buf = w.finish();
        // LEB128(2) = [0x02], then b"hi" = [0x68, 0x69]
        assert_eq!(buf, [0x02, 0x68, 0x69]);
    }

    #[test]
    fn write_bytes() {
        let mut w = BitWriter::new();
        w.write_bytes(&[0xDE, 0xAD]);
        let buf = w.finish();
        assert_eq!(buf, [0x02, 0xDE, 0xAD]);
    }

    #[test]
    fn write_raw_bytes() {
        let mut w = BitWriter::new();
        w.write_raw_bytes(&[0xCA, 0xFE]);
        let buf = w.finish();
        assert_eq!(buf, [0xCA, 0xFE]); // no length prefix
    }

    #[test]
    fn empty_flush_produces_zero_byte() {
        // Per spec §4.1: empty message produces a single 0x00 byte.
        // flush_to_byte_boundary on a fresh writer with no bits written
        // must emit one zero byte (not be a no-op).
        let mut w = BitWriter::new();
        w.flush_to_byte_boundary();
        let buf = w.finish();
        assert_eq!(buf, [0x00]);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-runtime bit_writer`
Expected: FAIL — methods not defined.

- [ ] **Step 3: Implement BitWriter**

Implement `BitWriter` in `crates/vexil-runtime/src/bit_writer.rs`. Key implementation notes:
- `new()` initializes `buf: Vec::new()`, `current_byte: 0`, `bit_offset: 0`.
- `write_bits(value, count)` — loop: extract LSB from value, set bit in `current_byte` at `bit_offset`, increment offset. When offset hits 8, push byte and reset.
- `flush_to_byte_boundary()` — push `current_byte` and reset. Even if `bit_offset == 0` and no bits have been written, push a zero byte (spec §4.1: empty message = 1 zero byte). After the first flush, subsequent no-op flushes at offset 0 should NOT emit extra bytes — track whether any content has been written (e.g., `buf` is non-empty or `bit_offset > 0`).
- Multi-byte integer writes: `flush_to_byte_boundary()` then `buf.extend_from_slice(&v.to_le_bytes())`.
- `write_f32`/`write_f64`: check `.is_nan()`, if so substitute canonical NaN bits. Then write LE bytes.
- `write_leb128`: flush, then call `crate::leb128::encode(&mut self.buf, v)`.
- `write_zigzag`: flush, then `crate::zigzag::zigzag_encode(v, type_bits)` → `write_leb128`.
- `write_string`/`write_bytes`: flush, LEB128 length prefix, extend with raw bytes.
- `write_raw_bytes`: flush, extend with raw bytes (no prefix).
- `finish(self)`: flush + return `self.buf`.

- [ ] **Step 4: Run all BitWriter tests**

Run: `cargo test -p vexil-runtime bit_writer`
Expected: All pass.

- [ ] **Step 5: Clippy check**

Run: `cargo clippy -p vexil-runtime --all-targets -- -D warnings`

- [ ] **Step 6: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-runtime/src/bit_writer.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-runtime): BitWriter — LSB-first bit accumulator with LE integers, NaN canonicalization, LEB128"
```

---

## Task 6: `vexil-runtime` — BitReader

**Files:**
- Modify: `crates/vexil-runtime/src/bit_reader.rs`

- [ ] **Step 1: Write BitReader tests**

Add `#[cfg(test)] mod tests` in `bit_reader.rs`. Tests should mirror BitWriter tests as round-trips:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::BitWriter;

    #[test]
    fn read_single_bit() {
        let mut r = BitReader::new(&[0x01]);
        assert_eq!(r.read_bool().unwrap(), true);
    }

    #[test]
    fn round_trip_sub_byte() {
        let mut w = BitWriter::new();
        w.write_bits(5, 3);
        w.write_bits(19, 5);
        w.write_bits(42, 6);
        let buf = w.finish();

        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_bits(3).unwrap(), 5);
        assert_eq!(r.read_bits(5).unwrap(), 19);
        assert_eq!(r.read_bits(6).unwrap(), 42);
    }

    #[test]
    fn round_trip_u16() {
        let mut w = BitWriter::new();
        w.write_u16(0x1234);
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_u16().unwrap(), 0x1234);
    }

    #[test]
    fn round_trip_i32_negative() {
        let mut w = BitWriter::new();
        w.write_i32(-42);
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_i32().unwrap(), -42);
    }

    #[test]
    fn round_trip_f32() {
        let mut w = BitWriter::new();
        w.write_f32(3.14);
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_f32().unwrap(), 3.14);
    }

    #[test]
    fn round_trip_f64_nan_canonical() {
        let mut w = BitWriter::new();
        w.write_f64(f64::NAN);
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let val = r.read_f64().unwrap();
        assert!(val.is_nan());
        assert_eq!(val.to_bits(), 0x7FF8000000000000);
    }

    #[test]
    fn round_trip_string() {
        let mut w = BitWriter::new();
        w.write_string("hello");
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_string().unwrap(), "hello");
    }

    #[test]
    fn round_trip_leb128() {
        let mut w = BitWriter::new();
        w.write_leb128(300);
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_leb128(4).unwrap(), 300);
    }

    #[test]
    fn round_trip_zigzag() {
        let mut w = BitWriter::new();
        w.write_zigzag(-42, 64);
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_zigzag(64, 10).unwrap(), -42);
    }

    #[test]
    fn unexpected_eof_on_empty() {
        let mut r = BitReader::new(&[]);
        assert_eq!(r.read_u8().unwrap_err(), DecodeError::UnexpectedEof);
    }

    #[test]
    fn invalid_utf8() {
        let mut w = BitWriter::new();
        // Write a "string" with invalid UTF-8 bytes
        w.write_leb128(2);
        w.write_raw_bytes(&[0xFF, 0xFE]);
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_string().unwrap_err(), DecodeError::InvalidUtf8);
    }

    #[test]
    fn recursion_depth_limit() {
        let mut r = BitReader::new(&[]);
        for _ in 0..64 {
            r.enter_recursive().unwrap();
        }
        assert_eq!(
            r.enter_recursive().unwrap_err(),
            DecodeError::RecursionLimitExceeded
        );
    }

    #[test]
    fn recursion_depth_leave() {
        let mut r = BitReader::new(&[]);
        for _ in 0..64 {
            r.enter_recursive().unwrap();
        }
        r.leave_recursive();
        r.enter_recursive().unwrap(); // should succeed after leave
    }

    #[test]
    fn flush_to_byte_boundary_reader() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3);
        w.flush_to_byte_boundary();
        w.write_u8(0xAB);
        let buf = w.finish();

        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_bits(3).unwrap(), 0b101);
        r.flush_to_byte_boundary();
        assert_eq!(r.read_u8().unwrap(), 0xAB);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-runtime bit_reader`
Expected: FAIL — methods not defined.

- [ ] **Step 3: Implement BitReader**

Implement `BitReader` in `crates/vexil-runtime/src/bit_reader.rs`. Key notes:
- `new(data)` initializes `byte_pos: 0`, `bit_offset: 0`, `recursion_depth: 0`.
- `read_bits(count)` — extract bits LSB-first from current byte, advance. Check EOF.
- `flush_to_byte_boundary()` — if `bit_offset > 0`, advance `byte_pos` by 1, reset `bit_offset`.
- Multi-byte integer reads: flush, check remaining bytes, read LE slice.
- `read_f32`/`read_f64`: read LE bytes, `from_le_bytes`. NaN is already canonical from writer.
- `read_leb128(max_bytes)`: flush, call `crate::leb128::decode(&self.data[self.byte_pos..], max_bytes)`, advance `byte_pos`.
- `read_zigzag(type_bits, max_bytes)`: `read_leb128(max_bytes)` then `crate::zigzag::zigzag_decode`.
- `read_string()`: flush, read_leb128(4) for length, check ≤ MAX_BYTES_LENGTH, read raw bytes, `String::from_utf8`.
- `read_bytes()`: like string but no UTF-8 check.
- `read_raw_bytes(len)`: flush, check remaining, copy slice.
- `enter_recursive()` / `leave_recursive()`: increment/decrement `recursion_depth`, check ≤ 64.

- [ ] **Step 4: Run all BitReader tests**

Run: `cargo test -p vexil-runtime bit_reader`
Expected: All pass.

- [ ] **Step 5: Run all runtime tests + clippy**

Run: `cargo test -p vexil-runtime && cargo clippy -p vexil-runtime --all-targets -- -D warnings`

- [ ] **Step 6: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-runtime/src/bit_reader.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-runtime): BitReader — LSB-first reader with LEB128, ZigZag, recursion depth tracking"
```

---

## Task 7: `vexil-runtime` — Wire format round-trip tests

**Files:**
- Create: `crates/vexil-runtime/tests/wire_roundtrip.rs`

These are hand-written types that mirror codegen output, proving the runtime handles all wire format patterns correctly.

- [ ] **Step 1: Write round-trip test file**

`crates/vexil-runtime/tests/wire_roundtrip.rs`:

```rust
//! Hand-written types mirroring codegen output.
//! Proves wire format correctness without compiling generated code.

use std::collections::BTreeMap;
use vexil_runtime::*;

// ── Simple Message ──

struct Hello {
    name: String,
    age: u8,
}

impl Pack for Hello {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_string(&self.name);
        w.write_u8(self.age);
        w.flush_to_byte_boundary();
        Ok(())
    }
}

impl Unpack for Hello {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        let name = r.read_string()?;
        let age = r.read_u8()?;
        r.flush_to_byte_boundary();
        Ok(Self { name, age })
    }
}

#[test]
fn hello_round_trip() {
    let val = Hello { name: "Alice".into(), age: 30 };
    let mut w = BitWriter::new();
    val.pack(&mut w).unwrap();
    let buf = w.finish();
    let mut r = BitReader::new(&buf);
    let decoded = Hello::unpack(&mut r).unwrap();
    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 30);
}

#[test]
fn hello_exact_bytes() {
    let val = Hello { name: "hi".into(), age: 7 };
    let mut w = BitWriter::new();
    val.pack(&mut w).unwrap();
    let buf = w.finish();
    // LEB128(2) + "hi" + u8(7) = [0x02, 0x68, 0x69, 0x07]
    assert_eq!(buf, [0x02, 0x68, 0x69, 0x07]);
}

// ── Sub-byte packing ──

// Message with u3 + u5 + u6 fields
struct SubByte {
    a: u8, // u3
    b: u8, // u5
    c: u8, // u6
}

impl Pack for SubByte {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_bits(self.a as u64, 3);
        w.write_bits(self.b as u64, 5);
        w.write_bits(self.c as u64, 6);
        w.flush_to_byte_boundary();
        Ok(())
    }
}

impl Unpack for SubByte {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        let a = r.read_bits(3)? as u8;
        let b = r.read_bits(5)? as u8;
        let c = r.read_bits(6)? as u8;
        r.flush_to_byte_boundary();
        Ok(Self { a, b, c })
    }
}

#[test]
fn sub_byte_round_trip() {
    let val = SubByte { a: 5, b: 19, c: 42 };
    let mut w = BitWriter::new();
    val.pack(&mut w).unwrap();
    let buf = w.finish();
    assert_eq!(buf, [0x9D, 0x2A]); // spec worked example
    let mut r = BitReader::new(&buf);
    let decoded = SubByte::unpack(&mut r).unwrap();
    assert_eq!((decoded.a, decoded.b, decoded.c), (5, 19, 42));
}

// ── Optional (byte-aligned T) ──

#[test]
fn optional_present_byte_aligned() {
    let mut w = BitWriter::new();
    let val: Option<String> = Some("hello".into());
    w.write_bool(val.is_some());
    if let Some(ref s) = val {
        w.flush_to_byte_boundary();
        w.write_string(s);
    }
    w.flush_to_byte_boundary();
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let present = r.read_bool().unwrap();
    assert!(present);
    r.flush_to_byte_boundary();
    let s = r.read_string().unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn optional_absent() {
    let mut w = BitWriter::new();
    w.write_bool(false);
    w.flush_to_byte_boundary();
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    assert!(!r.read_bool().unwrap());
}

// ── Optional (sub-byte T) ──

#[test]
fn optional_sub_byte_present() {
    let mut w = BitWriter::new();
    w.write_bool(true);   // present
    w.write_bits(5, 3);   // u3 value, no flush
    w.flush_to_byte_boundary();
    let buf = w.finish();
    // 1 bit (true) + 3 bits (101) = 4 bits → 0b0000_1011 = 0x0B?
    // LSB-first: bit0=1(present), bit1=1, bit2=0, bit3=1 from u3=5(101)
    // = 0b0000_1011 = 0x0B
    // Wait: u3=5 = 0b101, LSB-first packing after the bool bit:
    // bit0=1 (present), bit1=1 (lsb of 5), bit2=0, bit3=1 = 0b...1011
    assert_eq!(buf, [0x0B]);

    let mut r = BitReader::new(&buf);
    assert!(r.read_bool().unwrap());
    assert_eq!(r.read_bits(3).unwrap(), 5);
}

// ── Result ──

#[test]
fn result_ok_round_trip() {
    let mut w = BitWriter::new();
    w.write_bool(false); // 0 = Ok
    w.write_u32(42);
    w.flush_to_byte_boundary();
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let is_err = r.read_bool().unwrap();
    assert!(!is_err);
    assert_eq!(r.read_u32().unwrap(), 42);
}

#[test]
fn result_err_round_trip() {
    let mut w = BitWriter::new();
    w.write_bool(true); // 1 = Err
    w.write_string("oops");
    w.flush_to_byte_boundary();
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let is_err = r.read_bool().unwrap();
    assert!(is_err);
    assert_eq!(r.read_string().unwrap(), "oops");
}

// ── Empty message ──

#[test]
fn empty_message_produces_one_zero_byte() {
    let mut w = BitWriter::new();
    w.flush_to_byte_boundary();
    let buf = w.finish();
    assert_eq!(buf, [0x00]);
}

// ── Delta encoding (integer) ──

#[test]
fn delta_integer_round_trip() {
    // Simulate two sequential writes of a timestamp field with @delta
    let values: Vec<i64> = vec![1000, 1005, 1003];

    // Encode
    let mut w = BitWriter::new();
    let mut prev: i64 = 0;
    for &val in &values {
        let delta = val.wrapping_sub(prev);
        w.write_i64(delta);
        prev = val;
    }
    let buf = w.finish();

    // Decode
    let mut r = BitReader::new(&buf);
    let mut prev: i64 = 0;
    for &expected in &values {
        let delta = r.read_i64().unwrap();
        let val = prev.wrapping_add(delta);
        assert_eq!(val, expected);
        prev = val;
    }
}

// ── Delta + varint composition ──

#[test]
fn delta_varint_round_trip() {
    let values: Vec<u32> = vec![100, 200, 150];

    let mut w = BitWriter::new();
    let mut prev: u32 = 0;
    for &val in &values {
        let delta = val.wrapping_sub(prev);
        w.write_leb128(delta as u64); // varint-encoded delta
        prev = val;
    }
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let mut prev: u32 = 0;
    for &expected in &values {
        let delta = r.read_leb128(5).unwrap() as u32;
        let val = prev.wrapping_add(delta);
        assert_eq!(val, expected);
        prev = val;
    }
}

// ── Delta + zigzag composition ──

#[test]
fn delta_zigzag_round_trip() {
    let values: Vec<i32> = vec![10, 15, 8, -5];

    let mut w = BitWriter::new();
    let mut prev: i32 = 0;
    for &val in &values {
        let delta = val.wrapping_sub(prev);
        w.write_zigzag(delta as i64, 32);
        prev = val;
    }
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let mut prev: i32 = 0;
    for &expected in &values {
        let delta = r.read_zigzag(32, 5).unwrap() as i32;
        let val = prev.wrapping_add(delta);
        assert_eq!(val, expected);
        prev = val;
    }
}

// ── Delta on float ──

#[test]
fn delta_float_round_trip() {
    let values: Vec<f64> = vec![1.0, 1.5, 1.25];

    let mut w = BitWriter::new();
    let mut prev: f64 = 0.0;
    for &val in &values {
        let delta = val - prev;
        w.write_f64(delta);
        prev = val;
    }
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let mut prev: f64 = 0.0;
    for &expected in &values {
        let delta = r.read_f64().unwrap();
        let val = prev + delta;
        assert!((val - expected).abs() < f64::EPSILON);
        prev = val;
    }
}

// ── Union wire format ──

#[test]
fn union_wire_format() {
    // Encode: discriminant=1, payload = [3 bytes: u8, u8, u8]
    let mut w = BitWriter::new();
    w.flush_to_byte_boundary();
    w.write_leb128(1); // discriminant
    let mut payload = BitWriter::new();
    payload.write_u8(0xFF);
    payload.write_u8(0x00);
    payload.write_u8(0xAB);
    payload.flush_to_byte_boundary();
    let payload_bytes = payload.finish();
    w.write_leb128(payload_bytes.len() as u64); // byte length
    w.write_raw_bytes(&payload_bytes);
    let buf = w.finish();

    // Decode
    let mut r = BitReader::new(&buf);
    r.flush_to_byte_boundary();
    let disc = r.read_leb128(4).unwrap();
    assert_eq!(disc, 1);
    let len = r.read_leb128(4).unwrap() as usize;
    assert_eq!(len, 3);
    let payload = r.read_raw_bytes(len).unwrap();
    assert_eq!(payload, [0xFF, 0x00, 0xAB]);
}

// ── Overlong LEB128 rejection ──

#[test]
fn overlong_leb128_rejected() {
    // 0 encoded as [0x80, 0x00] — overlong
    let buf = [0x80, 0x00];
    let mut r = BitReader::new(&buf);
    assert_eq!(r.read_leb128(10).unwrap_err(), DecodeError::InvalidVarint);
}

// ── @limit enforcement ──

#[test]
fn encode_limit_exceeded() {
    // Simulate: field with @limit(2) on a string of length 3
    let err = EncodeError::LimitExceeded {
        field: "name",
        limit: 2,
        actual: 3,
    };
    // Just verify the error type is constructable and correct
    assert_eq!(err, EncodeError::LimitExceeded {
        field: "name",
        limit: 2,
        actual: 3,
    });
}
```

- [ ] **Step 2: Run wire round-trip tests**

Run: `cargo test -p vexil-runtime --test wire_roundtrip`
Expected: All pass (these use the runtime built in Tasks 4-6).

- [ ] **Step 3: Fix any failures, iterate**

If tests fail, fix runtime bugs and re-run.

- [ ] **Step 4: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-runtime/tests/wire_roundtrip.rs
VEXIL_COMMIT_TASK=1 git commit -m "test(vexil-runtime): wire format round-trip tests — sub-byte, optional, result, delta, union, LEB128"
```

---

## Task 8: Scaffold `vexil-codegen` crate + emit helpers + type mapping

**Files:**
- Create: `crates/vexil-codegen/Cargo.toml`
- Create: `crates/vexil-codegen/src/lib.rs`
- Create: `crates/vexil-codegen/src/emit.rs`
- Create: `crates/vexil-codegen/src/types.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Create crate and Cargo.toml**

```toml
[package]
name = "vexil-codegen"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Rust code generation backend for the Vexil schema compiler"
keywords = ["schema", "codegen", "serialization"]
categories = ["compilers", "development-tools"]

[dependencies]
vexil-lang = { path = "../vexil-lang" }
thiserror = "2"

[dev-dependencies]
test-case = "3"
```

Add `"crates/vexil-codegen"` to workspace members.

- [ ] **Step 2: Create `emit.rs` — CodeWriter helper**

`crates/vexil-codegen/src/emit.rs`:

```rust
/// Helper for emitting formatted Rust source code with indentation management.
pub struct CodeWriter {
    buf: String,
    indent: usize,
}

impl CodeWriter {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            indent: 0,
        }
    }

    /// Write a line with current indentation.
    pub fn line(&mut self, text: &str) {
        if text.is_empty() {
            self.buf.push('\n');
        } else {
            for _ in 0..self.indent {
                self.buf.push_str("    ");
            }
            self.buf.push_str(text);
            self.buf.push('\n');
        }
    }

    /// Write text without trailing newline (for partial lines).
    pub fn write(&mut self, text: &str) {
        for _ in 0..self.indent {
            self.buf.push_str("    ");
        }
        self.buf.push_str(text);
    }

    /// Append text to current line (no indentation).
    pub fn append(&mut self, text: &str) {
        self.buf.push_str(text);
    }

    /// Increase indentation.
    pub fn indent(&mut self) {
        self.indent += 1;
    }

    /// Decrease indentation.
    pub fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    /// Write an opening brace line and indent.
    pub fn open_block(&mut self, prefix: &str) {
        self.line(&format!("{prefix} {{"));
        self.indent();
    }

    /// Dedent and write closing brace.
    pub fn close_block(&mut self) {
        self.dedent();
        self.line("}");
    }

    /// Emit an empty line.
    pub fn blank(&mut self) {
        self.buf.push('\n');
    }

    /// Consume and return the built string.
    pub fn finish(self) -> String {
        self.buf
    }
}
```

- [ ] **Step 3: Create `types.rs` — ResolvedType → Rust type string**

`crates/vexil-codegen/src/types.rs`:

```rust
use std::collections::HashSet;
use vexil_lang::ast::{PrimitiveType, SemanticType, SubByteType};
use vexil_lang::ir::{ResolvedType, TypeDef, TypeId, TypeRegistry};

/// Convert a ResolvedType to its Rust type string.
/// `needs_box` contains `(type_id, field_index)` pairs that need Box wrapping.
/// `current_context` is `Some((type_id, field_index))` when we're in a boxed context.
pub fn rust_type(
    ty: &ResolvedType,
    registry: &TypeRegistry,
    needs_box: &HashSet<(TypeId, usize)>,
    context: Option<(TypeId, usize)>,
) -> String {
    match ty {
        ResolvedType::Primitive(p) => primitive_type(p).to_string(),
        ResolvedType::SubByte(s) => sub_byte_type(s).to_string(),
        ResolvedType::Semantic(s) => semantic_type(s).to_string(),
        ResolvedType::Named(id) => {
            let name = match registry.get(*id) {
                Some(def) => type_def_name(def),
                None => "UnresolvedType".to_string(),
            };
            if context.is_some_and(|ctx| needs_box.contains(&ctx)) {
                format!("Box<{name}>")
            } else {
                name
            }
        }
        ResolvedType::Optional(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, context);
            format!("Option<{inner_str}>")
        }
        ResolvedType::Array(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("Vec<{inner_str}>")
        }
        ResolvedType::Map(k, v) => {
            let k_str = rust_type(k, registry, needs_box, None);
            let v_str = rust_type(v, registry, needs_box, None);
            format!("BTreeMap<{k_str}, {v_str}>")
        }
        ResolvedType::Result(ok, err) => {
            let ok_str = rust_type(ok, registry, needs_box, context);
            let err_str = rust_type(err, registry, needs_box, context);
            format!("Result<{ok_str}, {err_str}>")
        }
    }
}

fn primitive_type(p: &PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::Bool => "bool",
        PrimitiveType::U8 => "u8",
        PrimitiveType::U16 => "u16",
        PrimitiveType::U32 => "u32",
        PrimitiveType::U64 => "u64",
        PrimitiveType::I8 => "i8",
        PrimitiveType::I16 => "i16",
        PrimitiveType::I32 => "i32",
        PrimitiveType::I64 => "i64",
        PrimitiveType::F32 => "f32",
        PrimitiveType::F64 => "f64",
        PrimitiveType::Void => "()",
    }
}

fn sub_byte_type(s: &SubByteType) -> &'static str {
    if s.signed { "i8" } else { "u8" }
}

fn semantic_type(s: &SemanticType) -> &'static str {
    match s {
        SemanticType::String => "String",
        SemanticType::Bytes => "Vec<u8>",
        SemanticType::Rgb => "(u8, u8, u8)",
        SemanticType::Uuid => "[u8; 16]",
        SemanticType::Timestamp => "i64",
        SemanticType::Hash => "[u8; 32]",
    }
}

fn type_def_name(def: &TypeDef) -> String {
    match def {
        TypeDef::Message(m) => m.name.to_string(),
        TypeDef::Enum(e) => e.name.to_string(),
        TypeDef::Flags(f) => f.name.to_string(),
        TypeDef::Union(u) => u.name.to_string(),
        TypeDef::Newtype(n) => n.name.to_string(),
        TypeDef::Config(c) => c.name.to_string(),
    }
}
```

- [ ] **Step 4: Create `lib.rs` with generate() stub**

`crates/vexil-codegen/src/lib.rs`:

```rust
pub mod annotations;
pub mod boxing;
pub mod config;
pub mod delta;
pub mod emit;
pub mod enum_gen;
pub mod flags;
pub mod message;
pub mod newtype;
pub mod types;
pub mod union_gen;

use vexil_lang::ir::{CompiledSchema, TypeId};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CodegenError {
    #[error("unresolved type {type_id:?} referenced by {referenced_by}")]
    UnresolvedType {
        type_id: TypeId,
        referenced_by: String,
    },
}

pub fn generate(compiled: &CompiledSchema) -> Result<String, CodegenError> {
    todo!("implemented in Task 10+")
}
```

Create empty stub modules: `annotations.rs`, `boxing.rs`, `config.rs`, `delta.rs`, `enum_gen.rs`, `flags.rs`, `message.rs`, `newtype.rs`, `union_gen.rs` — each as empty files.

**Note:** Add `thiserror = "2"` to `[dependencies]` in `Cargo.toml` since `CodegenError` derives from it.

- [ ] **Step 5: Verify workspace compiles**

Run: `cargo check --workspace --all-targets`
Expected: Compiles.

- [ ] **Step 6: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add Cargo.toml crates/vexil-codegen/
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): scaffold crate — CodeWriter, type mapping, generate() stub"
```

---

## Task 9: `vexil-codegen` — Boxing analysis

**Files:**
- Modify: `crates/vexil-codegen/src/boxing.rs`

- [ ] **Step 1: Write boxing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Tests will compile Vexil source and check which fields need boxing.
    // We need helper to compile and run analysis.

    fn analyze(src: &str) -> HashSet<(TypeId, usize)> {
        let result = vexil_lang::compile(src);
        let compiled = result.compiled.unwrap();
        detect_boxing(&compiled)
    }

    #[test]
    fn no_recursion_no_boxing() {
        let needs = analyze(r#"
            namespace test.box
            message Simple { name @0 : string }
        "#);
        assert!(needs.is_empty());
    }

    #[test]
    fn optional_self_reference_needs_box() {
        let needs = analyze(r#"
            namespace test.box
            message Node {
                value @0 : i32
                next  @1 : optional<Node>
            }
        "#);
        // Field index 1 (next) of the message type should need Box
        assert!(!needs.is_empty());
    }

    #[test]
    fn array_self_reference_no_box() {
        let needs = analyze(r#"
            namespace test.box
            message Tree {
                value    @0 : i32
                children @1 : array<Tree>
            }
        "#);
        // Vec<T> is already heap-allocated, no Box needed
        assert!(needs.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-codegen boxing`
Expected: FAIL — `detect_boxing` not defined.

- [ ] **Step 3: Implement boxing analysis**

In `crates/vexil-codegen/src/boxing.rs`:

Walk the type graph from each declaration. Track a stack of `TypeId`s. When reaching `Named(id)` through `Optional<T>` or `Result<T,E>` where `id` is already on the stack, mark that `(parent_type_id, field_index)` for boxing. `Vec<T>` and `BTreeMap<K,V>` are heap-allocated so no boxing needed for references through them.

```rust
use std::collections::HashSet;
use vexil_lang::ir::{CompiledSchema, ResolvedType, TypeDef, TypeId};

/// Returns set of (type_id, field_index) pairs that need Box<T> wrapping.
pub fn detect_boxing(compiled: &CompiledSchema) -> HashSet<(TypeId, usize)> {
    let mut needs_box = HashSet::new();
    for &id in &compiled.declarations {
        let mut path = Vec::new();
        path.push(id);
        match compiled.registry.get(id) {
            Some(TypeDef::Message(msg)) => {
                for (fi, field) in msg.fields.iter().enumerate() {
                    walk_for_boxing(&field.resolved_type, id, fi, &path, compiled, &mut needs_box);
                }
            }
            Some(TypeDef::Union(un)) => {
                for variant in &un.variants {
                    for (fi, field) in variant.fields.iter().enumerate() {
                        walk_for_boxing(&field.resolved_type, id, fi, &path, compiled, &mut needs_box);
                    }
                }
            }
            _ => {}
        }
    }
    needs_box
}

fn walk_for_boxing(
    ty: &ResolvedType,
    parent_id: TypeId,
    field_index: usize,
    path: &[TypeId],
    compiled: &CompiledSchema,
    needs_box: &mut HashSet<(TypeId, usize)>,
) {
    match ty {
        ResolvedType::Optional(inner) | ResolvedType::Result(inner, _) => {
            // Check if inner contains a back-reference to something on the path
            check_inner_for_cycle(inner, parent_id, field_index, path, compiled, needs_box);
            if let ResolvedType::Result(_, err) = ty {
                check_inner_for_cycle(err, parent_id, field_index, path, compiled, needs_box);
            }
        }
        ResolvedType::Named(id) => {
            // Direct named reference — descend but don't box (direct cycles are
            // caught by typeck as infinite recursion).
            if path.contains(id) {
                return; // Already on path, but not through Optional/Result — skip
            }
            let mut new_path = path.to_vec();
            new_path.push(*id);
            match compiled.registry.get(*id) {
                Some(TypeDef::Message(msg)) => {
                    for (fi, field) in msg.fields.iter().enumerate() {
                        walk_for_boxing(&field.resolved_type, *id, fi, &new_path, compiled, needs_box);
                    }
                }
                Some(TypeDef::Union(un)) => {
                    for variant in &un.variants {
                        for (fi, field) in variant.fields.iter().enumerate() {
                            walk_for_boxing(&field.resolved_type, *id, fi, &new_path, compiled, needs_box);
                        }
                    }
                }
                _ => {}
            }
        }
        ResolvedType::Array(_) | ResolvedType::Map(_, _) => {
            // Heap-allocated containers — no boxing needed
        }
        _ => {} // Primitive, SubByte, Semantic — terminal
    }
}

fn check_inner_for_cycle(
    ty: &ResolvedType,
    parent_id: TypeId,
    field_index: usize,
    path: &[TypeId],
    compiled: &CompiledSchema,
    needs_box: &mut HashSet<(TypeId, usize)>,
) {
    if let ResolvedType::Named(id) = ty {
        if path.contains(id) {
            needs_box.insert((parent_id, field_index));
            return;
        }
    }
    // Descend further for nested types
    walk_for_boxing(ty, parent_id, field_index, path, compiled, needs_box);
}
```

- [ ] **Step 4: Run boxing tests**

Run: `cargo test -p vexil-codegen boxing`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/boxing.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): boxing analysis — detect recursive types needing Box"
```

---

## Task 10: `vexil-codegen` — Annotations helper

**Files:**
- Modify: `crates/vexil-codegen/src/annotations.rs`

- [ ] **Step 1: Implement annotations emission**

`crates/vexil-codegen/src/annotations.rs`:

```rust
use crate::emit::CodeWriter;
use vexil_lang::ir::{ResolvedAnnotations, TombstoneDef};

/// Emit `@doc` as `///` doc comments.
pub fn emit_doc(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    for doc in &annotations.doc {
        w.line(&format!("/// {doc}"));
    }
}

/// Emit `@since` as a doc comment.
pub fn emit_since(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    if let Some(ref since) = annotations.since {
        w.line(&format!("/// @since {since}"));
    }
}

/// Emit `@deprecated` as `#[deprecated(...)]`.
pub fn emit_deprecated(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    if let Some(ref dep) = annotations.deprecated {
        match &dep.since {
            Some(since) => w.line(&format!(
                "#[deprecated(since = \"{since}\", note = \"{}\")]",
                dep.reason
            )),
            None => w.line(&format!("#[deprecated(note = \"{}\")]", dep.reason)),
        }
    }
}

/// Emit `@non_exhaustive` as `#[non_exhaustive]`.
pub fn emit_non_exhaustive(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    if annotations.non_exhaustive {
        w.line("#[non_exhaustive]");
    }
}

/// Emit all type-level annotations in standard order.
pub fn emit_type_annotations(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    emit_doc(w, annotations);
    emit_since(w, annotations);
    emit_deprecated(w, annotations);
    emit_non_exhaustive(w, annotations);
}

/// Emit field-level annotations.
pub fn emit_field_annotations(w: &mut CodeWriter, annotations: &ResolvedAnnotations) {
    emit_doc(w, annotations);
    emit_since(w, annotations);
    emit_deprecated(w, annotations);
}

/// Emit tombstone comments and REMOVED_ORDINALS constant.
/// Emit `@type`, `@domain`, `@revision` as doc comments (VNP protocol-level).
/// These are not in ResolvedAnnotations — pass raw values if available from
/// the AST's annotation list. Codegen reads from IR only, so these are
/// emitted only if the IR carries them (revision is in ResolvedAnnotations).
pub fn emit_protocol_annotations(
    w: &mut CodeWriter,
    annotations: &ResolvedAnnotations,
) {
    // @revision is stored in ResolvedAnnotations.
    // @type and @domain are NOT in the IR — they must be extracted from
    // the AST if needed. For now, emit @revision as a doc comment.
    if let Some(rev) = annotations.revision {
        w.line(&format!("/// @revision({rev})"));
    }
    // Note: @type(0xNN) and @domain(Name) are VNP protocol annotations
    // stored only in the AST, not the IR. The spec says codegen reads IR
    // only (line 704). If needed, a future change can propagate these
    // through ResolvedAnnotations or accept the AST alongside the IR.
}

pub fn emit_tombstones(w: &mut CodeWriter, type_name: &str, tombstones: &[TombstoneDef]) {
    if tombstones.is_empty() {
        return;
    }
    for t in tombstones {
        let since_str = t.since.as_deref().unwrap_or("unknown");
        w.line(&format!(
            "// REMOVED @{} (since {}): {}",
            t.ordinal, since_str, t.reason
        ));
    }
    w.write(&format!(
        "pub const {}_REMOVED_ORDINALS: &[(u16, &str, &str)] = &[",
        type_name.to_uppercase()
    ));
    w.append("\n");
    w.indent();
    for t in tombstones {
        let since_str = t.since.as_deref().unwrap_or("unknown");
        w.line(&format!(
            "({}, \"{}\", \"{}\"),",
            t.ordinal, since_str, t.reason
        ));
    }
    w.dedent();
    w.line("];");
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vexil-codegen`

- [ ] **Step 3: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/annotations.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): annotation emission — doc, deprecated, since, non_exhaustive, tombstones"
```

---

## Task 11: `vexil-codegen` — Field write/read dispatch + Message generation

**Files:**
- Modify: `crates/vexil-codegen/src/message.rs`

This is the core codegen module. It has two responsibilities: (a) a reusable dispatch layer that maps `ResolvedType + FieldEncoding` to `write_*`/`read_*` calls (used by message, union, and delta modules), and (b) message-specific struct + Pack/Unpack emission.

- [ ] **Step 1: Implement the field write/read dispatch functions**

These are the workhorses used by all type generators. Put them in `message.rs` (they're tightly coupled to message Pack/Unpack but exported for union/delta reuse).

```rust
/// Emit the write call for a field value.
/// `access` is the expression to write, e.g., "self.name" or "&self.name".
pub fn emit_write(
    w: &mut CodeWriter,
    access: &str,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    field_name: &str,  // for error messages in @limit checks
)
```

Dispatch rules for `emit_write`:
- **Encoding check first:**
  - `@varint` → `w.write_leb128({access} as u64);` with max_bytes per type (u16→3, u32→5, u64→10)
  - `@zigzag` → `w.write_zigzag({access} as i64, {type_bits});`
  - `@limit` → emit `if {access}.len() as u64 > {limit} { return Err(EncodeError::LimitExceeded { ... }) }`
  - `@delta` → NOT handled here (delta module wraps this)
- **Type dispatch (for `Encoding::Default`):**
  - `Primitive(Bool)` → `w.write_bool({access});`
  - `Primitive(U8)` → `w.write_u8({access});` (similarly for U16-U64, I8-I64)
  - `Primitive(F32)` → `w.write_f32({access});`
  - `Primitive(Void)` → nothing (0 bits)
  - `SubByte { bits, signed: false }` → `w.write_bits({access} as u64, {bits});`
  - `SubByte { bits, signed: true }` → `w.write_bits({access} as u8 as u64, {bits});` (truncate to N bits)
  - `Semantic(String)` → `w.write_string(&{access});`
  - `Semantic(Bytes)` → `w.write_bytes(&{access});`
  - `Semantic(Rgb)` → `w.write_u8({access}.0); w.write_u8({access}.1); w.write_u8({access}.2);`
  - `Semantic(Uuid)` → `w.write_raw_bytes(&{access});` (16 bytes, no prefix, at byte boundary)
  - `Semantic(Timestamp)` → `w.write_i64({access});`
  - `Semantic(Hash)` → `w.write_raw_bytes(&{access});` (32 bytes)
  - `Named(id)` → `{access}.pack(w)?;`
  - `Optional(inner)` → emit presence bit + conditional (flush rules depend on inner type's byte alignment)
  - `Array(inner)` → `w.write_leb128({access}.len() as u64);` then loop + emit_write for each element
  - `Map(k, v)` → `w.write_leb128({access}.len() as u64);` then loop over entries
  - `Result(ok, err)` → discriminant bit + conditional (same flush rules as optional)

```rust
/// Emit the read expression for a field.
/// Returns a String expression that evaluates to the field value.
pub fn emit_read(
    w: &mut CodeWriter,
    ty: &ResolvedType,
    enc: &FieldEncoding,
    registry: &TypeRegistry,
    field_name: &str,
    var_name: &str,  // variable name to bind the result
)
```

Mirror dispatch — `read_*` calls return `Result`, so chain `?`.

**Byte-alignment helper:** Add `fn is_byte_aligned(ty: &ResolvedType, registry: &TypeRegistry) -> bool` to determine whether a type needs `flush_to_byte_boundary()` before read/write in optional/result contexts. Sub-byte types (bool, SubByte, exhaustive enum with wire_bits < 8) are NOT byte-aligned; everything else is.

- [ ] **Step 2: Implement message struct emission**

```rust
pub fn emit_message(
    w: &mut CodeWriter,
    msg: &MessageDef,
    registry: &TypeRegistry,
    needs_box: &HashSet<(TypeId, usize)>,
    type_id: TypeId,
)
```

1. Emit `annotations::emit_type_annotations(w, &msg.annotations)`
2. Emit `#[derive(Debug, Clone, PartialEq)]`
3. Emit `pub struct {Name} {`
4. For each field: emit field annotations, `pub {name}: {rust_type},`
5. Close struct
6. Emit `impl Pack for {Name} {` → for each field call `emit_write`  → `w.flush_to_byte_boundary(); Ok(())`
7. Emit `impl Unpack for {Name} {` → for each field call `emit_read` → `r.flush_to_byte_boundary(); Ok(Self { ... })`

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p vexil-codegen`

- [ ] **Step 4: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/message.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): field dispatch + message struct/Pack/Unpack generation"
```

---

## Task 12: `vexil-codegen` — Enum generation

**Files:**
- Modify: `crates/vexil-codegen/src/enum_gen.rs`

- [ ] **Step 1: Implement enum code generation**

Generate: Rust enum with discriminant values, `impl Pack` using `write_bits(wire_bits)`, `impl Unpack` using `read_bits(wire_bits)` + match. Handle:
- Exhaustive (no `Unknown` variant)
- `@non_exhaustive` (add `Unknown(u64)` variant, `#[non_exhaustive]` attribute)
- Explicit backing type (use backing width for wire_bits)

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vexil-codegen`

- [ ] **Step 3: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/enum_gen.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): enum type + Pack/Unpack generation"
```

---

## Task 13: `vexil-codegen` — Flags generation

**Files:**
- Modify: `crates/vexil-codegen/src/flags.rs`

- [ ] **Step 1: Implement flags code generation**

Generate: newtype struct `F(pub u64)`, bit constants, `contains()`, `is_empty()`, `BitOr`, `BitAnd`, `Not` impls, `impl Pack` using `write_bits` or `write_u16/u32/u64` depending on `wire_bytes`, `impl Unpack` matching.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vexil-codegen`

- [ ] **Step 3: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/flags.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): flags struct + bitwise ops + Pack/Unpack generation"
```

---

## Task 14: `vexil-codegen` — Union generation

**Files:**
- Modify: `crates/vexil-codegen/src/union_gen.rs`

- [ ] **Step 1: Implement union generation**

Generate: Rust enum with struct variants. Wire format per spec §4.4:
- `impl Pack`: flush to byte boundary, LEB128 discriminant, create inner `BitWriter` for payload, encode variant fields into it, flush inner writer, LEB128 byte length of payload, `write_raw_bytes` for payload.
- `impl Unpack`: flush, read LEB128 discriminant, read LEB128 byte length, match on discriminant → create `BitReader` from `read_raw_bytes(len)`, decode variant fields, else if unknown → skip payload.
- Handle `@non_exhaustive` with `Unknown { discriminant: u64, data: Vec<u8> }` variant and `#[non_exhaustive]` attribute.
- Empty variant (e.g., `Reset @2 {}`) → discriminant + zero-length payload.
- Reuse `message::emit_write`/`emit_read` for variant field encoding.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vexil-codegen`

- [ ] **Step 3: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/union_gen.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): union enum + Pack/Unpack generation"
```

---

## Task 15: `vexil-codegen` — Newtype + Config generation

**Files:**
- Modify: `crates/vexil-codegen/src/newtype.rs`
- Modify: `crates/vexil-codegen/src/config.rs`

- [ ] **Step 1: Implement newtype generation**

Generate: `pub struct Foo(pub T)`, delegate `Pack`/`Unpack` to inner type. Use `terminal_type` from `NewtypeDef` for the wire encoding (not `inner_type`, which may be another newtype).

- [ ] **Step 2: Implement config generation**

Generate: `pub struct Config { pub fields... }`, `impl Default` with values from `ConfigFieldDef.default_value`. Map `DefaultValue` variants to Rust expressions:
- `DefaultValue::Bool(b)` → `true`/`false`
- `DefaultValue::Integer(n)` → `{n}`
- `DefaultValue::Float(f)` → `{f}`
- `DefaultValue::String(s)` → `String::from("{s}")`
- `DefaultValue::None` → `None`
- `DefaultValue::Array(items)` → `vec![...]`
- `DefaultValue::EnumVariant(name)` → `{EnumType}::{name}`

No `Pack`/`Unpack` generated for config types.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p vexil-codegen`

- [ ] **Step 4: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/newtype.rs crates/vexil-codegen/src/config.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): newtype + config generation"
```

---

## Task 16: `vexil-codegen` — Delta encoding generation

**Files:**
- Modify: `crates/vexil-codegen/src/delta.rs`

- [ ] **Step 1: Implement delta encoder/decoder generation**

For messages with any `@delta` fields, generate `TypeEncoder` and `TypeDecoder` structs. Each has:
- `prev_<field>: <type>` for each delta field (initialized to 0 / 0.0)
- `pub fn pack(&mut self, val: &Type, w: &mut BitWriter)` — compute delta, encode using base encoding
- `pub fn unpack(&mut self, r: &mut BitReader) -> Result<Type, DecodeError>`
- `pub fn reset(&mut self)` — reset all prev values

Handle compositions: `@delta @varint` → delta then LEB128, `@delta @zigzag` → delta then ZigZag.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vexil-codegen`

- [ ] **Step 3: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/delta.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): delta encoding — stateful Encoder/Decoder struct generation"
```

---

## Task 17: `vexil-codegen` — Wire `generate()` orchestrator

**Files:**
- Modify: `crates/vexil-codegen/src/lib.rs`

- [ ] **Step 1: Implement `generate()` orchestrator**

Replace the `todo!()` in `generate()` with the full pipeline:

1. Run `boxing::detect_boxing(compiled)` → `needs_box` set
2. Create `CodeWriter`
3. Emit header comment: `// Code generated by vexilc. DO NOT EDIT.`
4. Emit `// Source: <schema namespace>`
5. Emit blank line + `use std::collections::BTreeMap;` (if any maps) + `use vexil_runtime::*;`
6. Emit `SCHEMA_VERSION` constant if `compiled.annotations.version` is set
7. For each `type_id` in `compiled.declarations`:
   - Look up `TypeDef` from registry
   - Emit section separator comment
   - Emit tombstones
   - Dispatch to `message::emit`, `enum_gen::emit`, `flags::emit`, etc.
   - Emit delta encoder/decoder if message has `@delta` fields
8. Return `w.finish()`

Check for unresolved import stubs → `CodegenError::UnresolvedType`.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vexil-codegen`

- [ ] **Step 3: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/lib.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): generate() orchestrator — full pipeline wiring"
```

---

## Task 18: `vexil-codegen` — Golden file tests

**Files:**
- Create: `crates/vexil-codegen/tests/golden.rs`
- Create: `crates/vexil-codegen/tests/golden/006_message.rs`
- Create: `crates/vexil-codegen/tests/golden/007_enum.rs`
- Create: `crates/vexil-codegen/tests/golden/008_flags.rs`
- Create: `crates/vexil-codegen/tests/golden/009_union.rs`
- Create: `crates/vexil-codegen/tests/golden/010_newtype.rs`
- Create: `crates/vexil-codegen/tests/golden/011_config.rs`
- Create: `crates/vexil-codegen/tests/golden/016_recursive.rs`

- [ ] **Step 1: Create golden test runner**

`crates/vexil-codegen/tests/golden.rs`:

```rust
use std::fs;
use std::path::Path;
use vexil_lang::diagnostic::Severity;

fn golden_test(corpus_name: &str) {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("corpus/valid");
    let golden_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");

    let source_path = corpus_dir.join(format!("{corpus_name}.vexil"));
    let golden_path = golden_dir.join(format!("{corpus_name}.rs"));

    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", source_path.display()));
    let result = vexil_lang::compile(&source);
    assert!(
        !result.diagnostics.iter().any(|d| d.severity == Severity::Error),
        "compilation errors: {:?}", result.diagnostics
    );
    let compiled = result.compiled.expect("no compiled schema");
    let generated = vexil_codegen::generate(&compiled).expect("codegen failed");

    if std::env::var("UPDATE_GOLDEN").is_ok() {
        fs::create_dir_all(&golden_dir).ok();
        fs::write(&golden_path, &generated).unwrap();
        eprintln!("Updated golden file: {}", golden_path.display());
        return;
    }

    let expected = fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("cannot read golden {}: {e}\nRun with UPDATE_GOLDEN=1 to create", golden_path.display()));

    if generated != expected {
        // Show diff
        let diff = simple_diff(&expected, &generated);
        panic!("Golden file mismatch for {corpus_name}:\n{diff}");
    }
}

fn simple_diff(expected: &str, actual: &str) -> String {
    let mut out = String::new();
    for (i, (e, a)) in expected.lines().zip(actual.lines()).enumerate() {
        if e != a {
            out.push_str(&format!("Line {}:  expected: {e}\n", i + 1));
            out.push_str(&format!("Line {}:    actual: {a}\n", i + 1));
        }
    }
    let exp_lines = expected.lines().count();
    let act_lines = actual.lines().count();
    if exp_lines != act_lines {
        out.push_str(&format!("Line count: expected {exp_lines}, actual {act_lines}\n"));
    }
    out
}

#[test] fn golden_006_message() { golden_test("006_message"); }
#[test] fn golden_007_enum() { golden_test("007_enum"); }
#[test] fn golden_008_flags() { golden_test("008_flags"); }
#[test] fn golden_009_union() { golden_test("009_union"); }
#[test] fn golden_010_newtype() { golden_test("010_newtype"); }
#[test] fn golden_011_config() { golden_test("011_config"); }
#[test] fn golden_016_recursive() { golden_test("016_recursive"); }
```

- [ ] **Step 2: Generate golden files**

Run: `UPDATE_GOLDEN=1 cargo test -p vexil-codegen --test golden`

This creates all `.rs` golden files from current codegen output.

- [ ] **Step 3: Review golden files manually**

Read each generated `.rs` file. Verify it matches the expected patterns from the spec:
- Correct struct/enum shapes
- Correct `Pack`/`Unpack` implementations
- Correct `write_*`/`read_*` calls
- Correct annotations, tombstones
- Box wrapping on recursive types

Fix any codegen bugs found during review.

- [ ] **Step 4: Run golden tests (non-update mode)**

Run: `cargo test -p vexil-codegen --test golden`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/tests/
VEXIL_COMMIT_TASK=1 git commit -m "test(vexil-codegen): golden file tests for all 6 type kinds + recursive"
```

---

## Task 19: `vexilc` — Codegen subcommand

**Files:**
- Modify: `crates/vexilc/Cargo.toml`
- Modify: `crates/vexilc/src/main.rs`

- [ ] **Step 1: Add vexil-codegen dependency**

In `crates/vexilc/Cargo.toml`, add:
```toml
vexil-codegen = { path = "../vexil-codegen" }
```

- [ ] **Step 2: Add codegen subcommand to CLI**

Modify `crates/vexilc/src/main.rs` to support two modes:
- `vexilc check <file.vexil>` — current behavior (parse + validate)
- `vexilc codegen <file.vexil> [--output <path>]` — compile + generate Rust

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: vexilc <check|codegen> <file.vexil> [--output <path>]");
        std::process::exit(1);
    }

    let command = &args[1];
    let file = &args[2];

    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {file}: {e}");
            std::process::exit(1);
        }
    };

    match command.as_str() {
        "check" => run_check(file, &source),
        "codegen" => {
            let output = args.iter()
                .position(|a| a == "--output")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.as_str());
            run_codegen(file, &source, output);
        }
        _ => {
            eprintln!("Unknown command: {command}");
            eprintln!("Usage: vexilc <check|codegen> <file.vexil>");
            std::process::exit(1);
        }
    }
}
```

Keep `render_diagnostic` as is. `run_check` is the old `main` logic. `run_codegen` calls `vexil_lang::compile()`, checks for errors, then `vexil_codegen::generate()`, writes to output file or stdout.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p vexilc`

- [ ] **Step 4: Smoke test the CLI**

```bash
# Check mode (existing behavior)
cargo run -p vexilc -- check corpus/valid/006_message.vexil

# Codegen mode
cargo run -p vexilc -- codegen corpus/valid/006_message.vexil
# Should print generated Rust to stdout

# Codegen with output file
cargo run -p vexilc -- codegen corpus/valid/006_message.vexil --output /tmp/test_output.rs
cat /tmp/test_output.rs
```

- [ ] **Step 5: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexilc/
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexilc): codegen subcommand — compile + generate Rust output"
```

---

## Task 20: Quality gate — full workspace check

- [ ] **Step 1: Format check**

Run: `cargo fmt --all -- --check`

- [ ] **Step 2: Clippy (all crates)**

Run: `cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 3: All tests**

Run: `cargo test --workspace`

- [ ] **Step 4: Fix any issues found, iterate**

- [ ] **Step 5: Final commit (if any fixes)**

```bash
VEXIL_COMMIT_TASK=1 git add -A
VEXIL_COMMIT_TASK=1 git commit -m "chore: quality gate fixes for Milestone D"
```
