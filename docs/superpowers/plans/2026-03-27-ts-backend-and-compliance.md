# TypeScript Backend and Compliance Infrastructure — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a TypeScript codegen backend, `@vexil/runtime` npm package, compliance test vectors, benchmark suite, and encoding edge-case spec additions — validating the wire format for downstream consumers.

**Architecture:** One feature branch (`feature/ts-backend-and-compliance`) with 8 task groups in dependency order. Spec-first: normative additions land before any implementation. Compliance vectors are the shared contract between Rust and TypeScript. All TS work at `packages/runtime-ts/`.

**Tech Stack:** Rust (workspace), TypeScript (ES2022, Vitest), npm (`@vexil/runtime`), Criterion (benchmarks).

**Spec reference:** `docs/superpowers/specs/2026-03-27-ts-backend-and-compliance-design.md`

---

## File Structure

```
spec/
  vexil-spec.md                            # MODIFY: insert new §11 Encoding Edge Cases, renumber §11→§12 through §13→§14
corpus/
  valid/
    019_evolution_append_field.vexil        # NEW
    020_evolution_add_variant.vexil         # NEW
    021_empty_optionals.vexil              # NEW
    022_nested_schemas.vexil               # NEW
    023_recursive_depth.vexil              # NEW
    024_zero_length_payload.vexil          # NEW
    025_evolution_deprecate.vexil          # NEW
    026_required_to_optional.vexil         # NEW
  MANIFEST.md                              # MODIFY: add 8 new entries
crates/
  vexil-runtime/
    src/
      bit_writer.rs                        # MODIFY: add recursion depth tracking
      error.rs                             # MODIFY: add EncodeError::RecursionLimitExceeded
  vexil-codegen-rust/
    src/
      message.rs                           # MODIFY: emit enter_recursive/leave_recursive in Pack
    tests/
      golden/
        016_recursive.rs                   # MODIFY: updated golden file (depth tracking in Pack)
      golden_bytes.rs                      # NEW: compliance vector validator
      evolution_roundtrip.rs               # NEW: schema evolution roundtrip tests
  vexil-codegen-ts/                        # NEW crate
    Cargo.toml
    src/
      lib.rs                               # Entry point, generate(), generate_with_imports()
      backend.rs                           # TypeScriptBackend impl of CodegenBackend
      types.rs                             # Vexil→TypeScript type mapping
      message.rs                           # Message/Config codegen (interfaces + encode/decode)
      enum_gen.rs                          # Enum codegen (string literal union + const object)
      flags.rs                             # Flags codegen (number + named constants)
      union_gen.rs                         # Union codegen (discriminated union)
      newtype.rs                           # Newtype codegen (type alias)
      emit.rs                              # CodeWriter utility (indentation, blocks)
    tests/
      golden.rs                            # Golden output tests
      golden/
        006_message.ts                     # Golden files for each corpus schema
        007_enum.ts
        008_flags.ts
        009_union.ts
        010_newtype.ts
        011_config.ts
        016_recursive.ts
      compile_check.rs                     # tsc --noEmit on generated code
      project_compile_check.rs             # Multi-file TS project compilation
  vexil-bench/                             # NEW crate
    Cargo.toml
    src/
      lib.rs
      messages.rs                          # Hand-written VNP-representative message types
    benches/
      encode_decode.rs                     # Criterion benchmarks
  vexilc/
    src/
      main.rs                              # MODIFY: add "typescript" target dispatch
    Cargo.toml                             # MODIFY: add vexil-codegen-ts dependency
compliance/
  vectors/
    README.md                              # NEW: vector format documentation
    primitives.json                        # NEW
    sub_byte.json                          # NEW
    messages.json                          # NEW
    enums.json                             # NEW
    unions.json                            # NEW
    optionals.json                         # NEW
    arrays_maps.json                       # NEW
    evolution.json                         # NEW
packages/
  runtime-ts/
    package.json                           # NEW
    tsconfig.json                          # NEW
    vitest.config.ts                       # NEW
    src/
      index.ts                             # NEW: re-exports
      bit-reader.ts                        # NEW: BitReader class
      bit-writer.ts                        # NEW: BitWriter class
      leb128.ts                            # NEW: LEB128 encode/decode helpers
    tests/
      bit-reader.test.ts                   # NEW: unit tests
      bit-writer.test.ts                   # NEW: unit tests
      compliance.test.ts                   # NEW: golden vector compliance
Cargo.toml                                 # MODIFY: add vexil-codegen-ts and vexil-bench to workspace members
docs/
  limitations-and-gaps.md                  # NEW: living document
```

---

## Task 1: Spec §11 — Encoding Edge Cases

**Files:**
- Modify: `spec/vexil-spec.md` (insert new §11 between current §10 and §11, renumber §11→§12, §12→§13, §13→§14)

The current spec has §10 Breaking Change Rules → §11 Codegen Contract → §12 Standard Annotations → §13 Security Considerations. We insert a new §11 Encoding Edge Cases and shift the rest.

- [ ] **Step 1: Read the insertion point and existing section boundaries**

Read: `spec/vexil-spec.md` lines 810–820 (end of §10, start of current §11).

- [ ] **Step 2: Insert the new §11 section**

Insert after line 812 (the `---` separator after §10), before the current §11 heading. The new section:

```markdown
## §11  Encoding Edge Cases

These rules are normative.  A conformant implementation MUST handle each
case exactly as specified.  Behaviour on violation is "decode error" or
"encode error" unless stated otherwise.

### 11.1  Empty optionals

An `optional<T>` with no value encodes as a single `0` bit.  No payload
follows.  An `optional<T>` with a value encodes as a `1` bit followed by
`T`'s encoding.

For nested optionals (`optional<optional<T>>`):
- None → `0` (1 bit)
- Some(None) → `1 0` (2 bits)
- Some(Some(v)) → `1 1` followed by v's encoding

### 11.2  Zero-length payloads

A message with zero fields encodes as zero bytes (empty payload).
A union variant with no fields encodes as: discriminant (LEB128) +
length `0` (LEB128).

### 11.3  Maximum recursion depth

Recursive types (self-referencing messages via `optional` or `array`)
have a maximum nesting depth of **64** at encode and decode time.

- Encoder: returns `EncodeError::RecursionLimitExceeded`.
- Decoder: returns `DecodeError::RecursionLimitExceeded`.

Implementations MUST NOT rely on stack overflow for enforcement.

### 11.4  Trailing bytes

When a decoder has consumed all declared fields of a message, any
remaining bytes in the payload are **ignored**.  This enables forward
compatibility — a v2 encoder may append new fields that a v1 decoder
simply skips.

Decoders MUST NOT reject messages with trailing bytes after the last
known field.  Decoders MUST NOT interpret trailing bytes.

### 11.5  Sub-byte boundary at message end

After encoding all fields, the encoder calls `flush_to_byte_boundary()`.
Padding bits MUST be zero.  The decoder calls `flush_to_byte_boundary()`
after reading all known fields, before checking for trailing bytes.

### 11.6  Union discriminant overflow

If a decoder encounters a union discriminant value that does not match
any known variant:

- If the union is `@non_exhaustive`: skip the length-prefixed payload.
  The application receives an opaque discriminant + raw bytes.
- If the union is exhaustive: return `DecodeError::UnknownUnionVariant`.

The length-prefixed payload enables skipping unknown variants without
knowing their structure.

### 11.7  NaN canonicalization

All `f32` NaN values encode as `0x7FC00000` (canonical quiet NaN).
All `f64` NaN values encode as `0x7FF8000000000000` (canonical quiet NaN).

Signaling NaN, negative NaN, and NaN with payload are all mapped to
the canonical quiet NaN **before** encoding.  This ensures bit-identical
encoding for any NaN input.

### 11.8  Negative zero

`-0.0` is preserved on the wire (distinct from `+0.0`).  IEEE 754
defines `-0.0 == +0.0`, but their bit patterns differ.  Vexil preserves
the bit pattern for deterministic encoding.

### 11.9  String encoding errors

String fields use UTF-8.  An encoder receiving non-UTF-8 data returns
`EncodeError::InvalidUtf8`.  A decoder encountering invalid UTF-8 in a
string field returns `DecodeError::InvalidUtf8`.

`bytes` fields have no encoding restriction.

### 11.10  Schema evolution compatibility rules

**Adding a field** (new ordinal, appended in declaration order):
- v1 encoder → v2 decoder: v2 decoder reads known fields, new field gets
  its default value (zero / empty / None depending on type).
- v2 encoder → v1 decoder: v1 decoder reads its known fields, ignores
  trailing bytes (§11.4).

**Adding a variant** to a `@non_exhaustive` union:
- v2 encoder → v1 decoder: v1 decoder reads discriminant, does not
  recognise it, skips length-prefixed payload (§11.6).
- v1 encoder → v2 decoder: works unchanged (old variants still valid).

**Deprecating a field** (marking `@deprecated`):
- No wire change.  `@deprecated` is a source-level annotation only — the
  field is still encoded and decoded normally.  The ordinal remains
  reserved and MUST NOT be reused.

**Changing a required field to `optional<T>`** (**BREAKING**):
- This changes the wire encoding (a 1-bit presence flag is inserted
  before `T`'s encoding).  A v1 decoder reading v2-encoded data would
  misinterpret the presence flag as part of the field value.
- This is classified as a breaking change in §10.
```

- [ ] **Step 3: Renumber subsequent sections**

- Current `## §11  Codegen Contract` → `## §12  Codegen Contract`
- All subsection references: `### 11.1` → `### 12.1`, `### 11.2` → `### 12.2`, `### 11.3` → `### 12.3`
- Current `## §12  Standard Annotations` → `## §13  Standard Annotations`
- All subsection references: `### 12.1` → `### 13.1` through `### 12.6` → `### 13.6`
- Current `## §13  Security Considerations` → `## §14  Security Considerations`
- All subsection references: `### 13.1` → `### 14.1` through `### 13.4` → `### 14.4`

- [ ] **Step 4: Update any cross-references within the spec**

Search for `§11`, `§12`, `§13` in the body text and update references. Key references to check:
- The breaking changes table in §10 may reference `§11` or `§12`
- Security section may reference annotation sections
- Appendices may reference main sections

Run: `grep -n "§11\|§12\|§13" spec/vexil-spec.md` to find all cross-references.

- [ ] **Step 5: Verify the spec is well-formed**

Read the spec file and verify:
- Section numbering is sequential (§1 through §14 + Appendices)
- No duplicate section numbers
- Cross-references updated

- [ ] **Step 6: Commit**

```bash
git add spec/vexil-spec.md
git commit -m "spec: add §11 encoding edge cases (normative), renumber §11-§13 → §12-§14"
```

---

## Task 2: Edge Case Corpus Schemas

**Files:**
- Create: `corpus/valid/019_evolution_append_field.vexil` through `corpus/valid/026_required_to_optional.vexil`
- Modify: `corpus/MANIFEST.md`

- [ ] **Step 1: Write 019_evolution_append_field.vexil**

```vexil
namespace test.evolution.append

@version "1.0.0"

// V1 message — baseline
message HeaderV1 {
    kind   @0 : u8
    status @1 : u8
}

// V2 message — field appended (compatible change)
message HeaderV2 {
    kind   @0 : u8
    status @1 : u8
    flags  @2 : u16
}
```

- [ ] **Step 2: Write 020_evolution_add_variant.vexil**

```vexil
namespace test.evolution.variant

@version "1.0.0"

@non_exhaustive
union ShapeV1 {
    Circle @0 { radius @0 : f32 }
    Rect   @1 { w @0 : f32  h @1 : f32 }
}

@non_exhaustive
union ShapeV2 {
    Circle   @0 { radius @0 : f32 }
    Rect     @1 { w @0 : f32  h @1 : f32 }
    Triangle @2 { base @0 : f32  height @1 : f32 }
}
```

- [ ] **Step 3: Write 021_empty_optionals.vexil**

```vexil
namespace test.empty.optionals

message WithOptionals {
    name  @0 : optional<string>
    value @1 : optional<u32>
    flag  @2 : optional<bool>
}

message NestedOptional {
    inner @0 : optional<optional<u32>>
}

message AllEmpty {
    a @0 : optional<string>
    b @1 : optional<u32>
    c @2 : optional<bool>
}
```

- [ ] **Step 4: Write 022_nested_schemas.vexil**

```vexil
namespace test.nested.schemas

message Coord {
    x @0 : i32
    y @1 : i32
}

message Rect {
    origin @0 : Coord
    size   @1 : Coord
}

message Canvas {
    bounds @0 : Rect
    name   @1 : string
    layers @2 : array<Rect>
}
```

- [ ] **Step 5: Write 023_recursive_depth.vexil**

```vexil
namespace test.recursive.depth

message TreeNode {
    value    @0 : u32
    children @1 : array<TreeNode>
}

message LinkedList {
    value @0 : i64
    next  @1 : optional<LinkedList>
}
```

- [ ] **Step 6: Write 024_zero_length_payload.vexil**

```vexil
namespace test.zero.payload

message Empty {}

message Wrapper {
    inner @0 : Empty
    tag   @1 : u8
}

union Event {
    Ping @0 {}
    Pong @1 {}
    Data @2 { payload @0 : bytes }
}
```

- [ ] **Step 7: Write 025_evolution_deprecate.vexil**

```vexil
namespace test.evolution.deprecate

@version "2.0.0"

message Config {
    name     @0 : string
    @deprecated "use name_v2 instead"
    old_name @1 : string
    timeout  @2 : u32
}
```

- [ ] **Step 8: Write 026_required_to_optional.vexil**

```vexil
namespace test.evolution.reqopt

// V1: required field
message SettingsV1 {
    timeout @0 : u32
    name    @1 : string
}

// V2: timeout becomes optional (BREAKING — wire layout changes)
message SettingsV2 {
    timeout @0 : optional<u32>
    name    @1 : string
}
```

- [ ] **Step 9: Update MANIFEST.md**

Append to the valid corpus table:

```markdown
| 019_evolution_append_field.vexil | §11.10 | Schema evolution: field appended to message |
| 020_evolution_add_variant.vexil  | §11.10 | Schema evolution: variant added to `@non_exhaustive` union |
| 021_empty_optionals.vexil        | §11.1  | Empty and nested optional encoding |
| 022_nested_schemas.vexil         | §4.1   | Nested message references, arrays of messages |
| 023_recursive_depth.vexil        | §11.3  | Self-recursive and mutual recursive types |
| 024_zero_length_payload.vexil    | §11.2  | Empty messages, empty union variants |
| 025_evolution_deprecate.vexil    | §11.10 | Schema evolution: deprecated field (no wire change) |
| 026_required_to_optional.vexil   | §11.10 | Breaking change: required→optional wire layout difference |
```

- [ ] **Step 10: Run tests to verify new schemas compile**

Run: `cargo test --workspace`
Expected: All existing tests pass. New corpus files picked up by glob-based test harness.

- [ ] **Step 11: Commit**

```bash
git add corpus/valid/019_*.vexil corpus/valid/020_*.vexil corpus/valid/021_*.vexil \
        corpus/valid/022_*.vexil corpus/valid/023_*.vexil corpus/valid/024_*.vexil \
        corpus/valid/025_*.vexil corpus/valid/026_*.vexil \
        corpus/MANIFEST.md
git commit -m "corpus: add 8 schemas for encoding edge cases and evolution"
```

---

## Task 3: Rust Runtime — Recursion Depth in BitWriter

**Files:**
- Modify: `crates/vexil-runtime/src/bit_writer.rs`
- Modify: `crates/vexil-runtime/src/error.rs`

BitReader already has `enter_recursive()`/`leave_recursive()` with `recursion_depth` tracking and `DecodeError::RecursionLimitExceeded`. BitWriter does NOT — it needs the same for encode-side enforcement.

- [ ] **Step 1: Add RecursionLimitExceeded to EncodeError**

In `crates/vexil-runtime/src/error.rs`, add to the `EncodeError` enum:

```rust
    /// Recursive type nesting exceeded [`MAX_RECURSION_DEPTH`](crate::MAX_RECURSION_DEPTH).
    #[error("recursive type nesting exceeded 64 levels")]
    RecursionLimitExceeded,
```

- [ ] **Step 2: Add depth field and methods to BitWriter**

In `crates/vexil-runtime/src/bit_writer.rs`, add `recursion_depth: u32` to the struct:

```rust
pub struct BitWriter {
    buf: Vec<u8>,
    current_byte: u8,
    bit_offset: u8,
    recursion_depth: u32,
}
```

Update `new()` to initialize `recursion_depth: 0`.

Add methods after the existing methods (before the `Default` impl):

```rust
    /// Increment recursion depth; return error if limit exceeded.
    pub fn enter_recursive(&mut self) -> Result<(), EncodeError> {
        self.recursion_depth += 1;
        if self.recursion_depth > crate::MAX_RECURSION_DEPTH {
            return Err(EncodeError::RecursionLimitExceeded);
        }
        Ok(())
    }

    /// Decrement recursion depth.
    pub fn leave_recursive(&mut self) {
        self.recursion_depth = self.recursion_depth.saturating_sub(1);
    }
```

- [ ] **Step 3: Write tests for depth tracking**

Add to the test module in `crates/vexil-runtime/src/bit_writer.rs` (or create one if it doesn't exist):

```rust
#[cfg(test)]
mod depth_tests {
    use super::*;
    use crate::EncodeError;

    #[test]
    fn depth_increment_decrement() {
        let mut w = BitWriter::new();
        w.enter_recursive().unwrap();
        w.enter_recursive().unwrap();
        w.leave_recursive();
        w.leave_recursive();
        // No panic, depth back to 0
    }

    #[test]
    fn depth_max_64_succeeds() {
        let mut w = BitWriter::new();
        for _ in 0..64 {
            w.enter_recursive().unwrap();
        }
        // 64 is exactly the limit — should succeed
    }

    #[test]
    fn depth_65_exceeds_limit() {
        let mut w = BitWriter::new();
        for _ in 0..64 {
            w.enter_recursive().unwrap();
        }
        assert_eq!(
            w.enter_recursive().unwrap_err(),
            EncodeError::RecursionLimitExceeded
        );
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p vexil-runtime`
Expected: All pass, including new depth tests.

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-runtime/
git commit -m "feat(vexil-runtime): add recursion depth tracking to BitWriter"
```

---

## Task 4: Trailing Bytes Tolerance Test

**Files:**
- Modify: `crates/vexil-runtime/src/bit_reader.rs` (add test only — BitReader already tolerates trailing bytes)

- [ ] **Step 1: Write test verifying trailing bytes are tolerated**

Add to the test module in `crates/vexil-runtime/src/bit_reader.rs`:

```rust
    #[test]
    fn trailing_bytes_not_rejected() {
        // Simulate v2-encoded message read by v1 decoder:
        // v2 wrote u32(42) + u16(99), v1 only reads u32(42)
        let data = [0x2a, 0x00, 0x00, 0x00, 0x63, 0x00];
        let mut r = BitReader::new(&data);
        let x = r.read_u32().unwrap();
        assert_eq!(x, 42);
        r.flush_to_byte_boundary();
        // Remaining bytes (0x63, 0x00) must not cause error.
        // BitReader can be dropped with unread data — no panic.
    }
```

- [ ] **Step 2: Run test**

Run: `cargo test -p vexil-runtime trailing`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-runtime/src/bit_reader.rs
git commit -m "test(vexil-runtime): verify trailing bytes tolerance for schema evolution"
```

---

## Task 5: Codegen Emits Depth Tracking in Pack

**Files:**
- Modify: `crates/vexil-codegen-rust/src/message.rs`

Currently `emit_write_type` for `ResolvedType::Named(_)` emits:
```rust
"{access}.pack(w)?;"
```

It needs to also emit `enter_recursive`/`leave_recursive` on the writer, matching what `emit_read_type` already does for BitReader.

- [ ] **Step 1: Modify emit_write_type for Named types**

In `crates/vexil-codegen-rust/src/message.rs`, change the `ResolvedType::Named(_)` arm in `emit_write_type` (around line 165):

From:
```rust
        ResolvedType::Named(_) => {
            w.line(&format!("{access}.pack(w)?;"));
        }
```

To:
```rust
        ResolvedType::Named(_) => {
            w.line("w.enter_recursive()?;");
            w.line(&format!("{access}.pack(w)?;"));
            w.line("w.leave_recursive();");
        }
```

- [ ] **Step 2: Regenerate golden files**

Run: `UPDATE_GOLDEN=1 cargo test -p vexil-codegen-rust`
Expected: Golden files updated. The `016_recursive.rs` golden file should now include `w.enter_recursive()?;` and `w.leave_recursive();` in Pack impls.

- [ ] **Step 3: Verify the golden file change**

Read: `crates/vexil-codegen-rust/tests/golden/016_recursive.rs`
Verify `enter_recursive` and `leave_recursive` appear in Pack impls for TreeNode, LinkedList, and Expr/ExprKind.

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: Clean.

- [ ] **Step 6: Commit**

```bash
git add crates/vexil-codegen-rust/
git commit -m "feat(vexil-codegen-rust): emit depth tracking in Pack for recursive types"
```

---

## Task 6: Compliance Vectors — Primitives and Sub-Byte

**Files:**
- Create: `compliance/vectors/README.md`
- Create: `compliance/vectors/primitives.json`
- Create: `compliance/vectors/sub_byte.json`

- [ ] **Step 1: Write compliance/vectors/README.md**

```markdown
# Vexil Compliance Vectors

Golden byte vectors for cross-implementation testing.  Every conformant
Vexil implementation MUST produce identical bytes for the same input and
reconstruct identical values from the same bytes.

## Vector format

Standard vectors:

```json
{
  "name": "human-readable test name",
  "schema": "inline Vexil schema text",
  "type": "the message/enum/union type name to encode",
  "value": { "field": "value" },
  "expected_bytes": "hex-encoded expected output",
  "notes": "optional explanation"
}
```

Evolution vectors use a dual-schema format with `schema_v1`, `schema_v2`,
`value_v1`, `encoded_v1`, `decoded_as_v2` fields.

## Generating expected bytes

Expected bytes are generated by the Rust reference implementation.
The compliance validator in `crates/vexil-codegen-rust/tests/golden_bytes.rs`
re-encodes each vector and asserts byte-identical output.

## Cross-implementation testing

Any conformant implementation MUST:
1. Encode each vector's `value` and produce `expected_bytes`
2. Decode `expected_bytes` and reconstruct `value`

Failure on any vector is a conformance failure.
```

- [ ] **Step 2: Write compliance/vectors/primitives.json**

```json
[
  {
    "name": "bool_false",
    "schema": "namespace test.prim\nmessage M { v @0 : bool }",
    "type": "M",
    "value": { "v": false },
    "expected_bytes": "00",
    "notes": "bool false = 0 bit, flush to byte = 0x00"
  },
  {
    "name": "bool_true",
    "schema": "namespace test.prim\nmessage M { v @0 : bool }",
    "type": "M",
    "value": { "v": true },
    "expected_bytes": "01",
    "notes": "bool true = 1 bit, flush to byte = 0x01"
  },
  {
    "name": "u8_zero",
    "schema": "namespace test.prim\nmessage M { v @0 : u8 }",
    "type": "M",
    "value": { "v": 0 },
    "expected_bytes": "00"
  },
  {
    "name": "u8_max",
    "schema": "namespace test.prim\nmessage M { v @0 : u8 }",
    "type": "M",
    "value": { "v": 255 },
    "expected_bytes": "ff"
  },
  {
    "name": "u16_le",
    "schema": "namespace test.prim\nmessage M { v @0 : u16 }",
    "type": "M",
    "value": { "v": 258 },
    "expected_bytes": "0201",
    "notes": "258 = 0x0102, little-endian = 02 01"
  },
  {
    "name": "u32_le",
    "schema": "namespace test.prim\nmessage M { v @0 : u32 }",
    "type": "M",
    "value": { "v": 305419896 },
    "expected_bytes": "78563412",
    "notes": "0x12345678, little-endian = 78 56 34 12"
  },
  {
    "name": "i32_negative",
    "schema": "namespace test.prim\nmessage M { v @0 : i32 }",
    "type": "M",
    "value": { "v": -1 },
    "expected_bytes": "ffffffff",
    "notes": "-1 in two's complement i32"
  },
  {
    "name": "f32_nan_canonical",
    "schema": "namespace test.prim\nmessage M { v @0 : f32 }",
    "type": "M",
    "value": { "v": "NaN" },
    "expected_bytes": "0000c07f",
    "notes": "canonical qNaN 0x7FC00000, little-endian = 00 00 C0 7F"
  },
  {
    "name": "f64_negative_zero",
    "schema": "namespace test.prim\nmessage M { v @0 : f64 }",
    "type": "M",
    "value": { "v": "-0.0" },
    "expected_bytes": "0000000000000080",
    "notes": "-0.0 = 0x8000000000000000, little-endian"
  },
  {
    "name": "string_hello",
    "schema": "namespace test.prim\nmessage M { v @0 : string }",
    "type": "M",
    "value": { "v": "hello" },
    "expected_bytes": "0568656c6c6f",
    "notes": "LEB128 length 5 + UTF-8 'hello'"
  },
  {
    "name": "string_empty",
    "schema": "namespace test.prim\nmessage M { v @0 : string }",
    "type": "M",
    "value": { "v": "" },
    "expected_bytes": "00",
    "notes": "LEB128 length 0, no payload"
  }
]
```

- [ ] **Step 3: Write compliance/vectors/sub_byte.json**

```json
[
  {
    "name": "u3_u5_packed",
    "schema": "namespace test.sub\nmessage M { a @0 : u3  b @1 : u5 }",
    "type": "M",
    "value": { "a": 5, "b": 18 },
    "expected_bytes": "95",
    "notes": "a=101(3bit) b=10010(5bit) LSB-first → byte: 10010_101 = 0x95"
  },
  {
    "name": "u3_u5_u6_cross_byte",
    "schema": "namespace test.sub\nmessage M { a @0 : u3  b @1 : u5  c @2 : u6 }",
    "type": "M",
    "value": { "a": 7, "b": 31, "c": 63 },
    "expected_bytes": "fffc",
    "notes": "a=111 b=11111 c=111111 → bits: 11111_111 | 00_111111 → FF FC"
  },
  {
    "name": "u1_one",
    "schema": "namespace test.sub\nmessage M { v @0 : u1 }",
    "type": "M",
    "value": { "v": 1 },
    "expected_bytes": "01"
  }
]
```

- [ ] **Step 4: Commit**

```bash
git add compliance/
git commit -m "feat: add compliance vector format and primitive/sub-byte vectors"
```

---

## Task 7: Compliance Vectors — Messages, Optionals, Enums, Unions, Arrays/Maps, Evolution

**Files:**
- Create: `compliance/vectors/messages.json`
- Create: `compliance/vectors/optionals.json`
- Create: `compliance/vectors/enums.json`
- Create: `compliance/vectors/unions.json`
- Create: `compliance/vectors/arrays_maps.json`
- Create: `compliance/vectors/evolution.json`

- [ ] **Step 1: Write compliance/vectors/messages.json**

```json
[
  {
    "name": "empty_message",
    "schema": "namespace test.msg\nmessage Empty {}",
    "type": "Empty",
    "value": {},
    "expected_bytes": "",
    "notes": "Zero fields = zero bytes"
  },
  {
    "name": "two_u32_fields",
    "schema": "namespace test.msg\nmessage M { x @0 : u32  y @1 : u32 }",
    "type": "M",
    "value": { "x": 1, "y": 2 },
    "expected_bytes": "0100000002000000"
  },
  {
    "name": "mixed_bool_u16_string",
    "schema": "namespace test.msg\nmessage M { flag @0 : bool  count @1 : u16  name @2 : string }",
    "type": "M",
    "value": { "flag": true, "count": 42, "name": "test" },
    "expected_bytes": "012a000474657374",
    "notes": "bool(1 bit, flush to 0x01) + u16(LE 0x002A) + string(LEB128 len 4 + 'test')"
  }
]
```

- [ ] **Step 2: Write compliance/vectors/optionals.json**

```json
[
  {
    "name": "optional_none",
    "schema": "namespace test.opt\nmessage M { v @0 : optional<u32> }",
    "type": "M",
    "value": { "v": null },
    "expected_bytes": "00",
    "notes": "presence bit = 0, flush to byte"
  },
  {
    "name": "optional_some_u32",
    "schema": "namespace test.opt\nmessage M { v @0 : optional<u32> }",
    "type": "M",
    "value": { "v": 42 },
    "expected_bytes": "012a000000",
    "notes": "presence bit = 1, flush, then u32 LE"
  },
  {
    "name": "nested_optional_none",
    "schema": "namespace test.opt\nmessage M { v @0 : optional<optional<u32>> }",
    "type": "M",
    "value": { "v": null },
    "expected_bytes": "00",
    "notes": "outer None = 0 bit, flush"
  },
  {
    "name": "nested_optional_some_none",
    "schema": "namespace test.opt\nmessage M { v @0 : optional<optional<u32>> }",
    "type": "M",
    "value": { "v": { "inner": null } },
    "expected_bytes": "02",
    "notes": "outer Some(1) + inner None(0) = bits 10, flush = 0x02"
  },
  {
    "name": "nested_optional_some_some",
    "schema": "namespace test.opt\nmessage M { v @0 : optional<optional<u32>> }",
    "type": "M",
    "value": { "v": { "inner": 99 } },
    "expected_bytes": "0363000000",
    "notes": "outer Some(1) + inner Some(1) = bits 11, flush = 0x03, then u32 LE"
  }
]
```

- [ ] **Step 3: Write compliance/vectors/enums.json**

```json
[
  {
    "name": "enum_first_variant",
    "schema": "namespace test.enm\nenum Status { Active @0  Inactive @1 }  message M { v @0 : Status }",
    "type": "M",
    "value": { "v": "Active" },
    "expected_bytes": "00",
    "notes": "Enum with 2 variants uses 1 bit. Active = 0."
  },
  {
    "name": "enum_second_variant",
    "schema": "namespace test.enm\nenum Status { Active @0  Inactive @1 }  message M { v @0 : Status }",
    "type": "M",
    "value": { "v": "Inactive" },
    "expected_bytes": "01",
    "notes": "Inactive = 1, 1 bit, flush"
  }
]
```

- [ ] **Step 4: Write compliance/vectors/unions.json**

```json
[
  {
    "name": "union_first_variant",
    "schema": "namespace test.uni\nunion Shape { Circle @0 { radius @0 : f32 }  Rect @1 { w @0 : f32  h @1 : f32 } }\nmessage M { v @0 : Shape }",
    "type": "M",
    "value": { "v": { "variant": "Circle", "radius": 1.5 } },
    "expected_bytes": "000400000000c03f",
    "notes": "discriminant LEB128(0) + length LEB128(4) + f32 LE(1.5)"
  }
]
```

- [ ] **Step 5: Write compliance/vectors/arrays_maps.json**

```json
[
  {
    "name": "array_empty",
    "schema": "namespace test.arr\nmessage M { v @0 : array<u32> }",
    "type": "M",
    "value": { "v": [] },
    "expected_bytes": "00",
    "notes": "LEB128 count = 0"
  },
  {
    "name": "array_three_u32",
    "schema": "namespace test.arr\nmessage M { v @0 : array<u32> }",
    "type": "M",
    "value": { "v": [1, 2, 3] },
    "expected_bytes": "03010000000200000003000000",
    "notes": "LEB128 count 3 + three u32 LE values"
  },
  {
    "name": "map_one_entry",
    "schema": "namespace test.mp\nmessage M { v @0 : map<string, u32> }",
    "type": "M",
    "value": { "v": { "key": 42 } },
    "expected_bytes": "01036b65792a000000",
    "notes": "LEB128 count 1 + string('key', len 3) + u32 LE(42)"
  }
]
```

- [ ] **Step 6: Write compliance/vectors/evolution.json**

```json
[
  {
    "name": "v1_encode_v2_decode_appended_field",
    "schema_v1": "namespace test.evo\nmessage M { x @0 : u32 }",
    "schema_v2": "namespace test.evo\nmessage M { x @0 : u32  y @1 : u16 }",
    "type": "M",
    "value_v1": { "x": 42 },
    "encoded_v1": "2a000000",
    "decoded_as_v2": { "x": 42, "y": 0 },
    "notes": "v2 decoder fills y with default (0) when reading v1-encoded bytes"
  },
  {
    "name": "v2_encode_v1_decode_trailing_ignored",
    "schema_v1": "namespace test.evo\nmessage M { x @0 : u32 }",
    "schema_v2": "namespace test.evo\nmessage M { x @0 : u32  y @1 : u16 }",
    "type": "M",
    "value_v2": { "x": 42, "y": 99 },
    "encoded_v2": "2a0000006300",
    "decoded_as_v1": { "x": 42 },
    "notes": "v1 decoder reads x, ignores trailing bytes (y=99)"
  }
]
```

- [ ] **Step 7: Commit**

```bash
git add compliance/vectors/
git commit -m "feat: add compliance vectors for messages, optionals, enums, unions, arrays, evolution"
```

---

## Task 8: Rust Compliance Validator

**Files:**
- Create: `crates/vexil-codegen-rust/tests/golden_bytes.rs`
- Create: `crates/vexil-codegen-rust/tests/evolution_roundtrip.rs`

These tests validate that the Rust reference implementation produces bytes matching the compliance vectors.

- [ ] **Step 1: Write golden_bytes.rs — primitive and sub-byte validation**

```rust
//! Golden byte vector compliance tests.
//!
//! Validates that BitWriter produces bytes matching compliance/vectors/*.json.
//! This is the reference implementation — if these tests pass, the vectors are correct.

use vexil_runtime::BitWriter;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// --- Primitives ---

#[test]
fn verify_bool_false() {
    let mut w = BitWriter::new();
    w.write_bool(false);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "00");
}

#[test]
fn verify_bool_true() {
    let mut w = BitWriter::new();
    w.write_bool(true);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "01");
}

#[test]
fn verify_u8_zero() {
    let mut w = BitWriter::new();
    w.write_u8(0);
    assert_eq!(hex(&w.finish()), "00");
}

#[test]
fn verify_u8_max() {
    let mut w = BitWriter::new();
    w.write_u8(255);
    assert_eq!(hex(&w.finish()), "ff");
}

#[test]
fn verify_u16_le() {
    let mut w = BitWriter::new();
    w.write_u16(258);
    assert_eq!(hex(&w.finish()), "0201");
}

#[test]
fn verify_u32_le() {
    let mut w = BitWriter::new();
    w.write_u32(305419896);
    assert_eq!(hex(&w.finish()), "78563412");
}

#[test]
fn verify_i32_negative() {
    let mut w = BitWriter::new();
    w.write_i32(-1);
    assert_eq!(hex(&w.finish()), "ffffffff");
}

#[test]
fn verify_f32_nan_canonical() {
    let mut w = BitWriter::new();
    w.write_f32(f32::NAN);
    assert_eq!(hex(&w.finish()), "0000c07f");
}

#[test]
fn verify_f64_negative_zero() {
    let mut w = BitWriter::new();
    w.write_f64(-0.0_f64);
    assert_eq!(hex(&w.finish()), "0000000000000080");
}

#[test]
fn verify_string_hello() {
    let mut w = BitWriter::new();
    w.write_string("hello");
    assert_eq!(hex(&w.finish()), "0568656c6c6f");
}

#[test]
fn verify_string_empty() {
    let mut w = BitWriter::new();
    w.write_string("");
    assert_eq!(hex(&w.finish()), "00");
}

// --- Sub-byte ---

#[test]
fn verify_u3_u5_packed() {
    let mut w = BitWriter::new();
    w.write_bits(5, 3);  // u3 = 5
    w.write_bits(18, 5); // u5 = 18
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "95");
}

#[test]
fn verify_u3_u5_u6_cross_byte() {
    let mut w = BitWriter::new();
    w.write_bits(7, 3);  // u3 = 7
    w.write_bits(31, 5); // u5 = 31
    w.write_bits(63, 6); // u6 = 63
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "fffc");
}

// --- Messages ---

#[test]
fn verify_empty_message() {
    let w = BitWriter::new();
    assert_eq!(hex(&w.finish()), "");
}

#[test]
fn verify_two_u32_fields() {
    let mut w = BitWriter::new();
    w.write_u32(1);
    w.write_u32(2);
    assert_eq!(hex(&w.finish()), "0100000002000000");
}

#[test]
fn verify_mixed_bool_u16_string() {
    let mut w = BitWriter::new();
    w.write_bool(true);
    w.flush_to_byte_boundary();
    w.write_u16(42);
    w.write_string("test");
    assert_eq!(hex(&w.finish()), "012a000474657374");
}

// --- Optionals ---

#[test]
fn verify_optional_none() {
    let mut w = BitWriter::new();
    w.write_bool(false); // presence bit
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "00");
}

#[test]
fn verify_optional_some_u32() {
    let mut w = BitWriter::new();
    w.write_bool(true); // presence bit
    w.flush_to_byte_boundary();
    w.write_u32(42);
    assert_eq!(hex(&w.finish()), "012a000000");
}

// --- Arrays ---

#[test]
fn verify_array_empty() {
    let mut w = BitWriter::new();
    w.write_leb128(0); // count
    assert_eq!(hex(&w.finish()), "00");
}

#[test]
fn verify_array_three_u32() {
    let mut w = BitWriter::new();
    w.write_leb128(3);
    w.write_u32(1);
    w.write_u32(2);
    w.write_u32(3);
    assert_eq!(hex(&w.finish()), "03010000000200000003000000");
}
```

- [ ] **Step 2: Write evolution_roundtrip.rs**

```rust
//! Schema evolution roundtrip tests.
//!
//! Verifies forward and backward compatibility:
//! - v1 encode → v2 decode (new fields get defaults)
//! - v2 encode → v1 decode (trailing bytes ignored)

use vexil_runtime::{BitReader, BitWriter};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn encode_v1(x: u32) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.write_u32(x);
    w.finish()
}

fn encode_v2(x: u32, y: u16) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.write_u32(x);
    w.write_u16(y);
    w.finish()
}

fn decode_v1(bytes: &[u8]) -> u32 {
    let mut r = BitReader::new(bytes);
    r.read_u32().unwrap()
    // trailing bytes ignored per §11.4
}

fn decode_v2(bytes: &[u8]) -> (u32, u16) {
    let mut r = BitReader::new(bytes);
    let x = r.read_u32().unwrap();
    let y = if r.remaining() >= 2 {
        r.read_u16().unwrap()
    } else {
        0 // default for missing field
    };
    (x, y)
}

#[test]
fn v1_encode_v2_decode_field_gets_default() {
    let bytes = encode_v1(42);
    assert_eq!(hex(&bytes), "2a000000");
    let (x, y) = decode_v2(&bytes);
    assert_eq!(x, 42);
    assert_eq!(y, 0);
}

#[test]
fn v2_encode_v1_decode_trailing_ignored() {
    let bytes = encode_v2(42, 99);
    assert_eq!(hex(&bytes), "2a0000006300");
    let x = decode_v1(&bytes);
    assert_eq!(x, 42);
}

#[test]
fn v1_v2_prefix_bit_identical() {
    let v1_bytes = encode_v1(42);
    let v2_bytes = encode_v2(42, 99);
    assert_eq!(&v1_bytes[..4], &v2_bytes[..4]);
}

#[test]
fn v2_roundtrip() {
    let bytes = encode_v2(42, 99);
    let (x, y) = decode_v2(&bytes);
    assert_eq!(x, 42);
    assert_eq!(y, 99);
}

#[test]
fn deprecated_field_still_encodes() {
    // @deprecated is source-level only — no wire change
    let mut w = BitWriter::new();
    w.write_string("current");  // @0 name
    w.write_string("old");      // @1 old_name (@deprecated)
    w.write_u32(30);            // @2 timeout
    let bytes = w.finish();

    let mut r = BitReader::new(&bytes);
    assert_eq!(r.read_string().unwrap(), "current");
    assert_eq!(r.read_string().unwrap(), "old");
    assert_eq!(r.read_u32().unwrap(), 30);
}

#[test]
fn required_to_optional_is_breaking() {
    // V1: required u32 (4 bytes)
    let mut w1 = BitWriter::new();
    w1.write_u32(42);
    w1.write_string("test");
    let v1_bytes = w1.finish();

    // V2: optional<u32> with Some(42) — presence bit changes layout
    let mut w2 = BitWriter::new();
    w2.write_bool(true); // presence bit
    w2.flush_to_byte_boundary();
    w2.write_u32(42);
    w2.write_string("test");
    let v2_bytes = w2.finish();

    // Wire layouts MUST differ — this is a breaking change
    assert_ne!(v1_bytes, v2_bytes);
    assert_ne!(v1_bytes[0], v2_bytes[0]);
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p vexil-codegen-rust golden_bytes evolution_roundtrip`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add crates/vexil-codegen-rust/tests/golden_bytes.rs \
        crates/vexil-codegen-rust/tests/evolution_roundtrip.rs
git commit -m "test: golden byte compliance validator and evolution roundtrip tests"
```

---

## Task 9: `@vexil/runtime` TypeScript Package — Project Setup

**Files:**
- Create: `packages/runtime-ts/package.json`
- Create: `packages/runtime-ts/tsconfig.json`
- Create: `packages/runtime-ts/vitest.config.ts`
- Create: `packages/runtime-ts/src/index.ts`

- [ ] **Step 1: Create packages/runtime-ts/package.json**

```json
{
  "name": "@vexil/runtime",
  "version": "0.2.0",
  "description": "Runtime support for Vexil generated code — bit-level I/O and wire encoding primitives",
  "type": "module",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "files": ["dist", "README.md", "LICENSE"],
  "scripts": {
    "build": "tsc",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "devDependencies": {
    "typescript": "^5.5.0",
    "vitest": "^2.0.0"
  },
  "keywords": ["vexil", "schema", "serialization", "binary", "protocol"],
  "license": "MIT OR Apache-2.0"
}
```

- [ ] **Step 2: Create packages/runtime-ts/tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "outDir": "dist",
    "rootDir": "src",
    "declaration": true,
    "sourceMap": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

- [ ] **Step 3: Create packages/runtime-ts/vitest.config.ts**

```typescript
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['tests/**/*.test.ts'],
  },
});
```

- [ ] **Step 4: Create packages/runtime-ts/src/index.ts**

```typescript
export { BitReader } from './bit-reader.js';
export { BitWriter } from './bit-writer.js';
```

- [ ] **Step 5: Install dependencies**

Run: `cd packages/runtime-ts && npm install`
Expected: `node_modules/` created, `package-lock.json` generated.

- [ ] **Step 6: Commit**

```bash
git add packages/runtime-ts/package.json packages/runtime-ts/tsconfig.json \
        packages/runtime-ts/vitest.config.ts packages/runtime-ts/src/index.ts \
        packages/runtime-ts/package-lock.json
git commit -m "feat(@vexil/runtime): initialize TypeScript runtime package"
```

---

## Task 10: `@vexil/runtime` — BitWriter

**Files:**
- Create: `packages/runtime-ts/src/bit-writer.ts`
- Create: `packages/runtime-ts/tests/bit-writer.test.ts`

- [ ] **Step 1: Write the BitWriter test**

```typescript
import { describe, it, expect } from 'vitest';
import { BitWriter } from '../src/bit-writer.js';

function hex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

describe('BitWriter', () => {
  it('writes bool false', () => {
    const w = new BitWriter();
    w.writeBool(false);
    w.flushToByteBoundary();
    expect(hex(w.finish())).toBe('00');
  });

  it('writes bool true', () => {
    const w = new BitWriter();
    w.writeBool(true);
    w.flushToByteBoundary();
    expect(hex(w.finish())).toBe('01');
  });

  it('writes u8', () => {
    const w = new BitWriter();
    w.writeU8(255);
    expect(hex(w.finish())).toBe('ff');
  });

  it('writes u16 little-endian', () => {
    const w = new BitWriter();
    w.writeU16(258);
    expect(hex(w.finish())).toBe('0201');
  });

  it('writes u32 little-endian', () => {
    const w = new BitWriter();
    w.writeU32(305419896);
    expect(hex(w.finish())).toBe('78563412');
  });

  it('writes u64 little-endian', () => {
    const w = new BitWriter();
    w.writeU64(0x0102030405060708n);
    expect(hex(w.finish())).toBe('0807060504030201');
  });

  it('writes i32 negative', () => {
    const w = new BitWriter();
    w.writeI32(-1);
    expect(hex(w.finish())).toBe('ffffffff');
  });

  it('writes f32 with NaN canonicalization', () => {
    const w = new BitWriter();
    w.writeF32(NaN);
    expect(hex(w.finish())).toBe('0000c07f');
  });

  it('writes f64 negative zero', () => {
    const w = new BitWriter();
    w.writeF64(-0);
    expect(hex(w.finish())).toBe('0000000000000080');
  });

  it('writes string with LEB128 length', () => {
    const w = new BitWriter();
    w.writeString('hello');
    expect(hex(w.finish())).toBe('0568656c6c6f');
  });

  it('writes empty string', () => {
    const w = new BitWriter();
    w.writeString('');
    expect(hex(w.finish())).toBe('00');
  });

  it('writes sub-byte bits LSB-first', () => {
    const w = new BitWriter();
    w.writeBits(5, 3);  // u3 = 5
    w.writeBits(18, 5); // u5 = 18
    w.flushToByteBoundary();
    expect(hex(w.finish())).toBe('95');
  });

  it('writes cross-byte sub-byte fields', () => {
    const w = new BitWriter();
    w.writeBits(7, 3);
    w.writeBits(31, 5);
    w.writeBits(63, 6);
    w.flushToByteBoundary();
    expect(hex(w.finish())).toBe('fffc');
  });

  it('writes bytes with LEB128 length', () => {
    const w = new BitWriter();
    w.writeBytes(new Uint8Array([0xDE, 0xAD]));
    expect(hex(w.finish())).toBe('02dead');
  });

  it('enforces recursion depth limit', () => {
    const w = new BitWriter();
    for (let i = 0; i < 64; i++) {
      w.enterNested();
    }
    expect(() => w.enterNested()).toThrow('recursion depth exceeded');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/runtime-ts && npx vitest run`
Expected: FAIL — `bit-writer.ts` does not exist yet.

- [ ] **Step 3: Implement BitWriter**

Create `packages/runtime-ts/src/bit-writer.ts`:

```typescript
const MAX_RECURSION_DEPTH = 64;

export class BitWriter {
  private bytes: number[] = [];
  private currentByte = 0;
  private bitOffset = 0;
  private depth = 0;

  writeBits(value: number, count: number): void {
    let bitsWritten = 0;
    while (bitsWritten < count) {
      const bitsAvailable = 8 - this.bitOffset;
      const bitsToWrite = Math.min(count - bitsWritten, bitsAvailable);
      const mask = (1 << bitsToWrite) - 1;
      const bits = (value >>> bitsWritten) & mask;
      this.currentByte |= bits << this.bitOffset;
      bitsWritten += bitsToWrite;
      this.bitOffset += bitsToWrite;
      if (this.bitOffset >= 8) {
        this.bytes.push(this.currentByte);
        this.currentByte = 0;
        this.bitOffset = 0;
      }
    }
  }

  writeBool(v: boolean): void {
    this.writeBits(v ? 1 : 0, 1);
  }

  flushToByteBoundary(): void {
    if (this.bitOffset > 0) {
      this.bytes.push(this.currentByte);
      this.currentByte = 0;
      this.bitOffset = 0;
    }
  }

  private align(): void {
    this.flushToByteBoundary();
  }

  writeU8(v: number): void {
    this.align();
    this.bytes.push(v & 0xff);
  }

  writeU16(v: number): void {
    this.align();
    this.bytes.push(v & 0xff);
    this.bytes.push((v >>> 8) & 0xff);
  }

  writeU32(v: number): void {
    this.align();
    this.bytes.push(v & 0xff);
    this.bytes.push((v >>> 8) & 0xff);
    this.bytes.push((v >>> 16) & 0xff);
    this.bytes.push((v >>> 24) & 0xff);
  }

  writeU64(v: bigint): void {
    this.align();
    const lo = Number(v & 0xFFFFFFFFn);
    const hi = Number((v >> 32n) & 0xFFFFFFFFn);
    this.writeU32(lo);
    this.writeU32(hi);
  }

  writeI8(v: number): void {
    this.writeU8(v & 0xff);
  }

  writeI16(v: number): void {
    this.writeU16(v & 0xffff);
  }

  writeI32(v: number): void {
    this.align();
    const buf = new ArrayBuffer(4);
    new DataView(buf).setInt32(0, v, true);
    const arr = new Uint8Array(buf);
    for (const b of arr) this.bytes.push(b);
  }

  writeI64(v: bigint): void {
    this.writeU64(BigInt.asUintN(64, v));
  }

  writeF32(v: number): void {
    this.align();
    const buf = new ArrayBuffer(4);
    const dv = new DataView(buf);
    if (Number.isNaN(v)) {
      dv.setUint32(0, 0x7FC00000, true); // canonical qNaN
    } else {
      dv.setFloat32(0, v, true);
    }
    const arr = new Uint8Array(buf);
    for (const b of arr) this.bytes.push(b);
  }

  writeF64(v: number): void {
    this.align();
    const buf = new ArrayBuffer(8);
    const dv = new DataView(buf);
    if (Number.isNaN(v)) {
      // canonical qNaN: 0x7FF8000000000000, LE
      dv.setUint32(0, 0x00000000, true);
      dv.setUint32(4, 0x7FF80000, true);
    } else {
      dv.setFloat64(0, v, true);
    }
    const arr = new Uint8Array(buf);
    for (const b of arr) this.bytes.push(b);
  }

  writeLeb128(value: number): void {
    this.align();
    let v = value;
    do {
      let byte = v & 0x7f;
      v >>>= 7;
      if (v !== 0) byte |= 0x80;
      this.bytes.push(byte);
    } while (v !== 0);
  }

  writeString(s: string): void {
    const encoded = new TextEncoder().encode(s);
    this.writeLeb128(encoded.length);
    for (const b of encoded) this.bytes.push(b);
  }

  writeBytes(data: Uint8Array): void {
    this.writeLeb128(data.length);
    for (const b of data) this.bytes.push(b);
  }

  writeRawBytes(data: Uint8Array | number[]): void {
    this.align();
    for (const b of data) this.bytes.push(b);
  }

  enterNested(): void {
    this.depth++;
    if (this.depth > MAX_RECURSION_DEPTH) {
      throw new Error('recursive type nesting: recursion depth exceeded 64 levels');
    }
  }

  leaveNested(): void {
    this.depth = Math.max(0, this.depth - 1);
  }

  finish(): Uint8Array {
    this.flushToByteBoundary();
    return new Uint8Array(this.bytes);
  }
}
```

- [ ] **Step 4: Run tests**

Run: `cd packages/runtime-ts && npx vitest run`
Expected: All BitWriter tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/runtime-ts/src/bit-writer.ts packages/runtime-ts/tests/bit-writer.test.ts
git commit -m "feat(@vexil/runtime): implement BitWriter with LSB-first bitpack"
```

---

## Task 11: `@vexil/runtime` — BitReader

**Files:**
- Create: `packages/runtime-ts/src/bit-reader.ts`
- Create: `packages/runtime-ts/tests/bit-reader.test.ts`

- [ ] **Step 1: Write the BitReader test**

```typescript
import { describe, it, expect } from 'vitest';
import { BitReader } from '../src/bit-reader.js';

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

describe('BitReader', () => {
  it('reads bool false', () => {
    const r = new BitReader(fromHex('00'));
    expect(r.readBool()).toBe(false);
  });

  it('reads bool true', () => {
    const r = new BitReader(fromHex('01'));
    expect(r.readBool()).toBe(true);
  });

  it('reads u8', () => {
    const r = new BitReader(fromHex('ff'));
    expect(r.readU8()).toBe(255);
  });

  it('reads u16 little-endian', () => {
    const r = new BitReader(fromHex('0201'));
    expect(r.readU16()).toBe(258);
  });

  it('reads u32 little-endian', () => {
    const r = new BitReader(fromHex('78563412'));
    expect(r.readU32()).toBe(305419896);
  });

  it('reads u64 little-endian', () => {
    const r = new BitReader(fromHex('0807060504030201'));
    expect(r.readU64()).toBe(0x0102030405060708n);
  });

  it('reads i32 negative', () => {
    const r = new BitReader(fromHex('ffffffff'));
    expect(r.readI32()).toBe(-1);
  });

  it('reads f32 NaN', () => {
    const r = new BitReader(fromHex('0000c07f'));
    expect(Number.isNaN(r.readF32())).toBe(true);
  });

  it('reads f64 negative zero', () => {
    const r = new BitReader(fromHex('0000000000000080'));
    const val = r.readF64();
    expect(val).toBe(0);
    expect(Object.is(val, -0)).toBe(true);
  });

  it('reads string', () => {
    const r = new BitReader(fromHex('0568656c6c6f'));
    expect(r.readString()).toBe('hello');
  });

  it('reads empty string', () => {
    const r = new BitReader(fromHex('00'));
    expect(r.readString()).toBe('');
  });

  it('reads sub-byte bits LSB-first', () => {
    const r = new BitReader(fromHex('95'));
    expect(r.readBits(3)).toBe(5);
    expect(r.readBits(5)).toBe(18);
  });

  it('reads cross-byte sub-byte fields', () => {
    const r = new BitReader(fromHex('fffc'));
    expect(r.readBits(3)).toBe(7);
    expect(r.readBits(5)).toBe(31);
    expect(r.readBits(6)).toBe(63);
  });

  it('reads bytes with LEB128 length', () => {
    const r = new BitReader(fromHex('02dead'));
    const bytes = r.readBytes();
    expect(bytes.length).toBe(2);
    expect(bytes[0]).toBe(0xDE);
    expect(bytes[1]).toBe(0xAD);
  });

  it('tolerates trailing bytes', () => {
    const r = new BitReader(fromHex('2a0000006300'));
    expect(r.readU32()).toBe(42);
    // Remaining bytes (0x63, 0x00) not read — no error
  });

  it('enforces recursion depth limit', () => {
    const r = new BitReader(new Uint8Array(0));
    for (let i = 0; i < 64; i++) {
      r.enterNested();
    }
    expect(() => r.enterNested()).toThrow('recursion depth exceeded');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/runtime-ts && npx vitest run`
Expected: FAIL — `bit-reader.ts` does not exist yet.

- [ ] **Step 3: Implement BitReader**

Create `packages/runtime-ts/src/bit-reader.ts`:

```typescript
const MAX_RECURSION_DEPTH = 64;

export class BitReader {
  private data: Uint8Array;
  private bytePos = 0;
  private bitOffset = 0;
  private depth = 0;

  constructor(data: Uint8Array) {
    this.data = data;
  }

  readBits(count: number): number {
    let result = 0;
    let bitsRead = 0;
    while (bitsRead < count) {
      if (this.bytePos >= this.data.length) {
        throw new Error('BitReader: unexpected end of data');
      }
      const bitsAvailable = 8 - this.bitOffset;
      const bitsToRead = Math.min(count - bitsRead, bitsAvailable);
      const mask = (1 << bitsToRead) - 1;
      const bits = (this.data[this.bytePos] >>> this.bitOffset) & mask;
      result |= bits << bitsRead;
      bitsRead += bitsToRead;
      this.bitOffset += bitsToRead;
      if (this.bitOffset >= 8) {
        this.bitOffset = 0;
        this.bytePos++;
      }
    }
    return result;
  }

  readBool(): boolean {
    return this.readBits(1) === 1;
  }

  flushToByteBoundary(): void {
    if (this.bitOffset > 0) {
      this.bitOffset = 0;
      this.bytePos++;
    }
  }

  readU8(): number {
    this.flushToByteBoundary();
    const v = this.data[this.bytePos];
    this.bytePos++;
    return v;
  }

  readU16(): number {
    this.flushToByteBoundary();
    const v = this.data[this.bytePos] | (this.data[this.bytePos + 1] << 8);
    this.bytePos += 2;
    return v;
  }

  readU32(): number {
    this.flushToByteBoundary();
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 4);
    const v = dv.getUint32(0, true);
    this.bytePos += 4;
    return v;
  }

  readU64(): bigint {
    this.flushToByteBoundary();
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 8);
    const lo = BigInt(dv.getUint32(0, true));
    const hi = BigInt(dv.getUint32(4, true));
    this.bytePos += 8;
    return lo | (hi << 32n);
  }

  readI8(): number {
    this.flushToByteBoundary();
    const v = this.data[this.bytePos];
    this.bytePos++;
    return v > 127 ? v - 256 : v;
  }

  readI16(): number {
    this.flushToByteBoundary();
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 2);
    const v = dv.getInt16(0, true);
    this.bytePos += 2;
    return v;
  }

  readI32(): number {
    this.flushToByteBoundary();
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 4);
    const v = dv.getInt32(0, true);
    this.bytePos += 4;
    return v;
  }

  readI64(): bigint {
    this.flushToByteBoundary();
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 8);
    const lo = BigInt(dv.getUint32(0, true));
    const hi = BigInt(dv.getInt32(4, true));
    this.bytePos += 8;
    return (hi << 32n) | lo;
  }

  readF32(): number {
    this.flushToByteBoundary();
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 4);
    const v = dv.getFloat32(0, true);
    this.bytePos += 4;
    return v;
  }

  readF64(): number {
    this.flushToByteBoundary();
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 8);
    const v = dv.getFloat64(0, true);
    this.bytePos += 8;
    return v;
  }

  readLeb128(): number {
    this.flushToByteBoundary();
    let result = 0;
    let shift = 0;
    while (this.bytePos < this.data.length) {
      const byte = this.data[this.bytePos];
      this.bytePos++;
      result |= (byte & 0x7f) << shift;
      if ((byte & 0x80) === 0) break;
      shift += 7;
      if (shift > 28) throw new Error('LEB128 overflow for 32-bit');
    }
    return result;
  }

  readString(): string {
    const len = this.readLeb128();
    const bytes = this.data.slice(this.bytePos, this.bytePos + len);
    this.bytePos += len;
    return new TextDecoder().decode(bytes);
  }

  readBytes(): Uint8Array {
    const len = this.readLeb128();
    const bytes = this.data.slice(this.bytePos, this.bytePos + len);
    this.bytePos += len;
    return bytes;
  }

  readRawBytes(count: number): Uint8Array {
    this.flushToByteBoundary();
    const bytes = this.data.slice(this.bytePos, this.bytePos + count);
    this.bytePos += count;
    return bytes;
  }

  remaining(): number {
    return this.data.length - this.bytePos;
  }

  enterNested(): void {
    this.depth++;
    if (this.depth > MAX_RECURSION_DEPTH) {
      throw new Error('recursive type nesting: recursion depth exceeded 64 levels');
    }
  }

  leaveNested(): void {
    this.depth = Math.max(0, this.depth - 1);
  }
}
```

- [ ] **Step 4: Run tests**

Run: `cd packages/runtime-ts && npx vitest run`
Expected: All BitReader and BitWriter tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/runtime-ts/src/bit-reader.ts packages/runtime-ts/tests/bit-reader.test.ts
git commit -m "feat(@vexil/runtime): implement BitReader with LSB-first bitpack"
```

---

## Task 12: `@vexil/runtime` — Compliance Tests Against Golden Vectors

**Files:**
- Create: `packages/runtime-ts/tests/compliance.test.ts`

- [ ] **Step 1: Write compliance test**

```typescript
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { BitWriter } from '../src/bit-writer.js';
import { BitReader } from '../src/bit-reader.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const vectorsDir = join(__dirname, '../../../compliance/vectors');

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

describe('primitives compliance', () => {
  const vectors = JSON.parse(readFileSync(join(vectorsDir, 'primitives.json'), 'utf-8'));

  for (const v of vectors) {
    it(`${v.name}: encode produces expected bytes`, () => {
      const w = new BitWriter();

      switch (v.name) {
        case 'bool_false': w.writeBool(false); w.flushToByteBoundary(); break;
        case 'bool_true': w.writeBool(true); w.flushToByteBoundary(); break;
        case 'u8_zero': w.writeU8(0); break;
        case 'u8_max': w.writeU8(255); break;
        case 'u16_le': w.writeU16(258); break;
        case 'u32_le': w.writeU32(305419896); break;
        case 'i32_negative': w.writeI32(-1); break;
        case 'f32_nan_canonical': w.writeF32(NaN); break;
        case 'f64_negative_zero': w.writeF64(-0); break;
        case 'string_hello': w.writeString('hello'); break;
        case 'string_empty': w.writeString(''); break;
        default: return; // skip unknown vectors
      }

      expect(toHex(w.finish())).toBe(v.expected_bytes);
    });

    it(`${v.name}: decode matches expected value`, () => {
      if (v.expected_bytes === '') return; // skip empty vectors
      const r = new BitReader(fromHex(v.expected_bytes));

      switch (v.name) {
        case 'bool_false': expect(r.readBool()).toBe(false); break;
        case 'bool_true': expect(r.readBool()).toBe(true); break;
        case 'u8_zero': expect(r.readU8()).toBe(0); break;
        case 'u8_max': expect(r.readU8()).toBe(255); break;
        case 'u16_le': expect(r.readU16()).toBe(258); break;
        case 'u32_le': expect(r.readU32()).toBe(305419896); break;
        case 'i32_negative': expect(r.readI32()).toBe(-1); break;
        case 'f32_nan_canonical': expect(Number.isNaN(r.readF32())).toBe(true); break;
        case 'f64_negative_zero': {
          const val = r.readF64();
          expect(Object.is(val, -0)).toBe(true);
          break;
        }
        case 'string_hello': expect(r.readString()).toBe('hello'); break;
        case 'string_empty': expect(r.readString()).toBe(''); break;
      }
    });
  }
});

describe('sub-byte compliance', () => {
  it('u3_u5_packed: encode', () => {
    const w = new BitWriter();
    w.writeBits(5, 3);
    w.writeBits(18, 5);
    w.flushToByteBoundary();
    expect(toHex(w.finish())).toBe('95');
  });

  it('u3_u5_packed: decode', () => {
    const r = new BitReader(fromHex('95'));
    expect(r.readBits(3)).toBe(5);
    expect(r.readBits(5)).toBe(18);
  });

  it('u3_u5_u6 cross-byte: encode', () => {
    const w = new BitWriter();
    w.writeBits(7, 3);
    w.writeBits(31, 5);
    w.writeBits(63, 6);
    w.flushToByteBoundary();
    expect(toHex(w.finish())).toBe('fffc');
  });

  it('u3_u5_u6 cross-byte: decode', () => {
    const r = new BitReader(fromHex('fffc'));
    expect(r.readBits(3)).toBe(7);
    expect(r.readBits(5)).toBe(31);
    expect(r.readBits(6)).toBe(63);
  });
});
```

- [ ] **Step 2: Run compliance tests**

Run: `cd packages/runtime-ts && npx vitest run`
Expected: All pass — TS encode/decode matches Rust golden vectors.

- [ ] **Step 3: Commit**

```bash
git add packages/runtime-ts/tests/compliance.test.ts
git commit -m "test(@vexil/runtime): compliance tests against golden byte vectors"
```

---

## Task 13: `vexil-codegen-ts` Crate Scaffold

**Files:**
- Create: `crates/vexil-codegen-ts/Cargo.toml`
- Create: `crates/vexil-codegen-ts/src/lib.rs`
- Create: `crates/vexil-codegen-ts/src/emit.rs`
- Create: `crates/vexil-codegen-ts/src/types.rs`
- Modify: `Cargo.toml` (workspace root — add member)

This task sets up the crate structure and code-writing utility. Subsequent tasks add type generation for each declaration kind.

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "vexil-codegen-ts"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "TypeScript code generation backend for the Vexil schema compiler"
categories = ["encoding"]
keywords = ["vexil", "schema", "typescript", "codegen"]

[dependencies]
vexil-lang = { path = "../vexil-lang", version = "^0.2.0" }
```

- [ ] **Step 2: Add to workspace members**

In root `Cargo.toml`, add `"crates/vexil-codegen-ts"` to the workspace members list.

- [ ] **Step 3: Create src/emit.rs — CodeWriter utility**

```rust
/// Indentation-aware code writer for TypeScript generation.
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

    pub fn line(&mut self, text: &str) {
        if text.is_empty() {
            self.buf.push('\n');
        } else {
            for _ in 0..self.indent {
                self.buf.push_str("  ");
            }
            self.buf.push_str(text);
            self.buf.push('\n');
        }
    }

    pub fn blank(&mut self) {
        self.buf.push('\n');
    }

    pub fn open_block(&mut self, header: &str) {
        self.line(&format!("{header} {{"));
        self.indent += 1;
    }

    pub fn close_block(&mut self) {
        self.indent = self.indent.saturating_sub(1);
        self.line("}");
    }

    pub fn close_block_with(&mut self, suffix: &str) {
        self.indent = self.indent.saturating_sub(1);
        self.line(&format!("}}{suffix}"));
    }

    pub fn finish(self) -> String {
        self.buf
    }
}
```

- [ ] **Step 4: Create src/types.rs — Vexil→TypeScript type mapping**

```rust
use vexil_lang::ir::{PrimitiveType, ResolvedType, SemanticType, SubByteType, TypeId, TypeRegistry};

/// Map a Vexil ResolvedType to its TypeScript type string.
pub fn ts_type(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::Bool => "boolean".to_string(),
            PrimitiveType::U8 | PrimitiveType::U16 | PrimitiveType::U32 => "number".to_string(),
            PrimitiveType::I8 | PrimitiveType::I16 | PrimitiveType::I32 => "number".to_string(),
            PrimitiveType::U64 | PrimitiveType::I64 => "bigint".to_string(),
            PrimitiveType::F32 | PrimitiveType::F64 => "number".to_string(),
            PrimitiveType::Void => "void".to_string(),
            _ => "unknown".to_string(),
        },
        ResolvedType::SubByte(_) => "number".to_string(),
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => "string".to_string(),
            SemanticType::Bytes => "Uint8Array".to_string(),
            SemanticType::Uuid => "string".to_string(),
            SemanticType::Timestamp => "Date".to_string(),
            SemanticType::Rgb => "[number, number, number]".to_string(),
            SemanticType::Hash => "Uint8Array".to_string(),
            _ => "unknown".to_string(),
        },
        ResolvedType::Named(id) => type_name_for_id(*id, registry),
        ResolvedType::Optional(inner) => format!("{} | null", ts_type(inner, registry)),
        ResolvedType::Array(inner) => format!("{}[]", ts_type(inner, registry)),
        ResolvedType::Map(k, v) => format!("Map<{}, {}>", ts_type(k, registry), ts_type(v, registry)),
        ResolvedType::Result(ok, err) => {
            format!("{{ ok: {} }} | {{ err: {} }}", ts_type(ok, registry), ts_type(err, registry))
        }
        _ => "unknown".to_string(),
    }
}

fn type_name_for_id(id: TypeId, registry: &TypeRegistry) -> String {
    if let Some(def) = registry.get(id) {
        def.name().to_string()
    } else {
        "unknown".to_string()
    }
}
```

- [ ] **Step 5: Create src/lib.rs — initial scaffold**

```rust
pub mod emit;
pub mod types;

use std::collections::BTreeMap;
use std::path::PathBuf;

use vexil_lang::codegen::{CodegenBackend, CodegenError};
use vexil_lang::ir::CompiledSchema;
use vexil_lang::project::ProjectResult;

pub struct TypeScriptBackend;

impl CodegenBackend for TypeScriptBackend {
    fn name(&self) -> &str {
        "typescript"
    }

    fn file_extension(&self) -> &str {
        "ts"
    }

    fn generate(&self, compiled: &CompiledSchema) -> Result<String, CodegenError> {
        Ok(generate(compiled))
    }

    fn generate_project(
        &self,
        result: &ProjectResult,
    ) -> Result<BTreeMap<PathBuf, String>, CodegenError> {
        generate_project(result)
    }
}

/// Generate TypeScript code for a single compiled schema.
pub fn generate(compiled: &CompiledSchema) -> String {
    let mut w = emit::CodeWriter::new();
    w.line("import { BitReader, BitWriter } from '@vexil/runtime';");
    w.blank();

    // TODO: implemented in subsequent tasks
    for &type_id in &compiled.declarations {
        if let Some(def) = compiled.registry.get(type_id) {
            // Placeholder — filled in by Task 14-17
            let _ = def;
        }
    }

    w.finish()
}

/// Generate all files for a multi-file project.
pub fn generate_project(
    result: &ProjectResult,
) -> Result<BTreeMap<PathBuf, String>, CodegenError> {
    let mut files = BTreeMap::new();
    // TODO: implemented in Task 18
    for (namespace, compiled) in &result.schemas {
        let code = generate(compiled);
        let path = namespace_to_path(namespace);
        files.insert(path, code);
    }
    Ok(files)
}

fn namespace_to_path(namespace: &str) -> PathBuf {
    let parts: Vec<&str> = namespace.split('.').collect();
    if parts.len() > 1 {
        let dir = parts[..parts.len() - 1].join("/");
        let file = parts.last().unwrap();
        PathBuf::from(format!("{dir}/{file}.ts"))
    } else {
        PathBuf::from(format!("{namespace}.ts"))
    }
}
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check -p vexil-codegen-ts`
Expected: PASS — compiles with no errors.

- [ ] **Step 7: Commit**

```bash
git add crates/vexil-codegen-ts/ Cargo.toml
git commit -m "feat(vexil-codegen-ts): scaffold TS codegen crate with emit, types, and backend"
```

---

## Task 14: `vexil-codegen-ts` — Message and Config Generation

**Files:**
- Create: `crates/vexil-codegen-ts/src/message.rs`
- Modify: `crates/vexil-codegen-ts/src/lib.rs`

This is the largest codegen task — it generates TypeScript interfaces and encode/decode functions for message and config types. The implementation follows the same pattern as `vexil-codegen-rust/src/message.rs` but emitting TypeScript syntax.

This task is too large to include complete code inline. The implementer should:

- [ ] **Step 1: Read the Rust codegen message.rs for reference**

Read: `crates/vexil-codegen-rust/src/message.rs` (full file)
Understand the pattern: `emit_message()` generates the struct + Pack + Unpack. The TS version generates interface + `encodeFoo()` + `decodeFoo()`.

- [ ] **Step 2: Create src/message.rs with emit_message()**

The function signature:
```rust
use crate::emit::CodeWriter;
use crate::types::ts_type;
use vexil_lang::ir::{
    CompiledSchema, ConfigDef, Encoding, FieldEncoding, MessageDef, PrimitiveType,
    ResolvedType, SemanticType, TypeDef, TypeId, TypeRegistry,
};

/// Emit a TypeScript interface + encode/decode functions for a message.
pub fn emit_message(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry) {
    // 1. Emit interface
    w.open_block(&format!("export interface {}", msg.name));
    for field in &msg.fields {
        let ts = ts_type(&field.resolved_type, registry);
        w.line(&format!("{}: {};", field.name, ts));
    }
    w.close_block();
    w.blank();

    // 2. Emit encode function
    emit_encode(w, msg, registry);
    w.blank();

    // 3. Emit decode function
    emit_decode(w, msg, registry);
}

/// Emit `export function encodeFoo(v: Foo, w: BitWriter): void { ... }`
fn emit_encode(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry) {
    w.open_block(&format!(
        "export function encode{}(v: {}, w: BitWriter): void",
        msg.name, msg.name
    ));
    for field in &msg.fields {
        emit_write_field(w, &format!("v.{}", field.name), &field.resolved_type, &field.encoding, registry);
    }
    w.line("w.flushToByteBoundary();");
    w.close_block();
}

/// Emit `export function decodeFoo(r: BitReader): Foo { ... }`
fn emit_decode(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry) {
    w.open_block(&format!(
        "export function decode{}(r: BitReader): {}",
        msg.name, msg.name
    ));
    for field in &msg.fields {
        emit_read_field(w, &field.name, &field.resolved_type, &field.encoding, registry);
    }
    // Return object
    let field_names: Vec<&str> = msg.fields.iter().map(|f| f.name.as_str()).collect();
    w.line(&format!("return {{ {} }};", field_names.join(", ")));
    w.close_block();
}
```

The `emit_write_field` and `emit_read_field` functions follow the same type-dispatch pattern as Rust but emit TypeScript method calls on `BitWriter`/`BitReader`:

- `PrimitiveType::Bool` → `w.writeBool(access)` / `const name = r.readBool()`
- `PrimitiveType::U32` → `w.writeU32(access)` / `const name = r.readU32()`
- `PrimitiveType::U64` → `w.writeU64(access)` / `const name = r.readU64()`
- `SemanticType::String` → `w.writeString(access)` / `const name = r.readString()`
- `ResolvedType::Named(_)` → `w.enterNested(); encodeFoo(access, w); w.leaveNested()` / `r.enterNested(); const name = decodeFoo(r); r.leaveNested()`
- `ResolvedType::Optional(inner)` → presence bit + conditional
- `ResolvedType::Array(inner)` → LEB128 count + loop
- `ResolvedType::Map(k, v)` → LEB128 count + loop over entries

Config types generate an interface only (no encode/decode).

- [ ] **Step 3: Wire message.rs into lib.rs**

Add `pub mod message;` to `lib.rs` and call `message::emit_message()` / `message::emit_config()` in the `generate()` function's TypeDef match.

- [ ] **Step 4: Run cargo check**

Run: `cargo check -p vexil-codegen-ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-codegen-ts/src/message.rs crates/vexil-codegen-ts/src/lib.rs
git commit -m "feat(vexil-codegen-ts): message and config code generation"
```

---

## Task 15: `vexil-codegen-ts` — Enum and Flags Generation

**Files:**
- Create: `crates/vexil-codegen-ts/src/enum_gen.rs`
- Create: `crates/vexil-codegen-ts/src/flags.rs`
- Modify: `crates/vexil-codegen-ts/src/lib.rs`

- [ ] **Step 1: Create enum_gen.rs**

```rust
use crate::emit::CodeWriter;
use vexil_lang::ir::{EnumDef, TypeRegistry};

/// Emit TypeScript string literal union + const object for an enum.
///
/// ```typescript
/// export type Status = 'Active' | 'Inactive';
/// export const Status = {
///   Active: 'Active' as const,
///   Inactive: 'Inactive' as const,
/// };
/// ```
pub fn emit_enum(w: &mut CodeWriter, e: &EnumDef, _registry: &TypeRegistry) {
    // Type alias (string literal union)
    let variants: Vec<String> = e.variants.iter().map(|v| format!("'{}'", v.name)).collect();
    w.line(&format!("export type {} = {};", e.name, variants.join(" | ")));
    w.blank();

    // Const object
    w.open_block(&format!("export const {} =", e.name));
    for v in &e.variants {
        w.line(&format!("{}: '{}' as const,", v.name, v.name));
    }
    w.close_block_with(";");
    w.blank();

    // Encode function
    w.open_block(&format!(
        "export function encode{}(v: {}, w: BitWriter): void",
        e.name, e.name
    ));
    // Determine bit width from number of variants
    let bits = bits_for_variants(e.variants.len());
    w.open_block("const ordinal = (() =>");
    w.open_block("switch (v)");
    for v in &e.variants {
        w.line(&format!("case '{}': return {};", v.name, v.ordinal));
    }
    w.close_block();
    w.close_block_with(")();");
    w.line(&format!("w.writeBits(ordinal, {bits});"));
    w.close_block();
    w.blank();

    // Decode function
    w.open_block(&format!(
        "export function decode{}(r: BitReader): {}",
        e.name, e.name
    ));
    w.line(&format!("const ordinal = r.readBits({bits});"));
    w.open_block("switch (ordinal)");
    for v in &e.variants {
        w.line(&format!("case {}: return '{}';", v.ordinal, v.name));
    }
    w.line(&format!(
        "default: throw new Error(`unknown {} variant: ${{ordinal}}`);",
        e.name
    ));
    w.close_block();
    w.close_block();
}

fn bits_for_variants(count: usize) -> u8 {
    if count <= 1 { return 1; }
    let max_ordinal = count - 1;
    (64 - (max_ordinal as u64).leading_zeros()) as u8
}
```

- [ ] **Step 2: Create flags.rs**

```rust
use crate::emit::CodeWriter;
use vexil_lang::ir::{FlagsDef, TypeRegistry};

/// Emit TypeScript number type + named constants for flags.
///
/// ```typescript
/// export type Permissions = number;
/// export const Permissions = {
///   Read: 1,
///   Write: 2,
///   Execute: 4,
/// };
/// ```
pub fn emit_flags(w: &mut CodeWriter, f: &FlagsDef, _registry: &TypeRegistry) {
    w.line(&format!("export type {} = number;", f.name));
    w.blank();

    w.open_block(&format!("export const {} =", f.name));
    for bit in &f.bits {
        w.line(&format!("{}: {},", bit.name, 1u64 << bit.position));
    }
    w.close_block_with(";");
    w.blank();

    // Encode: write as fixed-width integer
    let wire_bytes = f.wire_bytes;
    w.open_block(&format!(
        "export function encode{}(v: {}, w: BitWriter): void",
        f.name, f.name
    ));
    match wire_bytes {
        1 => w.line("w.writeU8(v);"),
        2 => w.line("w.writeU16(v);"),
        4 => w.line("w.writeU32(v);"),
        _ => w.line(&format!("w.writeBits(v, {});", wire_bytes * 8)),
    }
    w.close_block();
    w.blank();

    // Decode
    w.open_block(&format!(
        "export function decode{}(r: BitReader): {}",
        f.name, f.name
    ));
    match wire_bytes {
        1 => w.line("return r.readU8();"),
        2 => w.line("return r.readU16();"),
        4 => w.line("return r.readU32();"),
        _ => w.line(&format!("return r.readBits({});", wire_bytes * 8)),
    }
    w.close_block();
}
```

- [ ] **Step 3: Wire into lib.rs**

Add `pub mod enum_gen;` and `pub mod flags;` to `lib.rs`. In the `generate()` function's match on `TypeDef`, add arms for `TypeDef::Enum` and `TypeDef::Flags`.

- [ ] **Step 4: Run cargo check**

Run: `cargo check -p vexil-codegen-ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-codegen-ts/src/enum_gen.rs crates/vexil-codegen-ts/src/flags.rs \
        crates/vexil-codegen-ts/src/lib.rs
git commit -m "feat(vexil-codegen-ts): enum and flags code generation"
```

---

## Task 16: `vexil-codegen-ts` — Union and Newtype Generation

**Files:**
- Create: `crates/vexil-codegen-ts/src/union_gen.rs`
- Create: `crates/vexil-codegen-ts/src/newtype.rs`
- Modify: `crates/vexil-codegen-ts/src/lib.rs`

- [ ] **Step 1: Create union_gen.rs**

Unions generate a discriminated TypeScript union type. Each variant is a tagged interface. The encode function uses a switch on a `tag` property, the decode function reads the discriminant and dispatches.

```rust
use crate::emit::CodeWriter;
use crate::types::ts_type;
use vexil_lang::ir::{UnionDef, TypeRegistry};

pub fn emit_union(w: &mut CodeWriter, u: &UnionDef, registry: &TypeRegistry) {
    // Emit per-variant interfaces
    for variant in &u.variants {
        w.open_block(&format!("export interface {}_{}", u.name, variant.name));
        w.line(&format!("tag: '{}';", variant.name));
        for field in &variant.fields {
            let ts = ts_type(&field.resolved_type, registry);
            w.line(&format!("{}: {};", field.name, ts));
        }
        w.close_block();
        w.blank();
    }

    // Emit union type alias
    let variant_types: Vec<String> = u
        .variants
        .iter()
        .map(|v| format!("{}_{}", u.name, v.name))
        .collect();
    w.line(&format!(
        "export type {} = {};",
        u.name,
        variant_types.join(" | ")
    ));
    w.blank();

    // Encode function
    w.open_block(&format!(
        "export function encode{}(v: {}, w: BitWriter): void",
        u.name, u.name
    ));
    w.open_block("switch (v.tag)");
    for variant in &u.variants {
        w.open_block(&format!("case '{}':", variant.name));
        w.line(&format!("w.writeLeb128({});", variant.ordinal));
        // Union variants are length-prefixed: encode to temp writer, write length, then bytes
        w.line("const inner = new BitWriter();");
        for field in &variant.fields {
            // Write fields to inner writer
            // The implementer should call the field-write dispatch here
            // similar to message emit_write_field
        }
        w.line("inner.flushToByteBoundary();");
        w.line("const payload = inner.finish();");
        w.line("w.writeLeb128(payload.length);");
        w.line("w.writeRawBytes(payload);");
        w.line("break;");
        w.close_block();
    }
    w.close_block();
    w.close_block();
    w.blank();

    // Decode function — reads discriminant, dispatches by variant
    w.open_block(&format!(
        "export function decode{}(r: BitReader): {}",
        u.name, u.name
    ));
    w.line("const discriminant = r.readLeb128();");
    w.line("const length = r.readLeb128();");
    w.open_block("switch (discriminant)");
    for variant in &u.variants {
        w.open_block(&format!("case {}:", variant.ordinal));
        for field in &variant.fields {
            // Read fields — the implementer should call field-read dispatch here
        }
        let field_names: Vec<String> = variant
            .fields
            .iter()
            .map(|f| f.name.to_string())
            .collect();
        w.line(&format!(
            "return {{ tag: '{}', {} }};",
            variant.name,
            field_names.join(", ")
        ));
        w.close_block();
    }
    w.line(&format!(
        "default: throw new Error(`unknown {} variant: ${{discriminant}}`);",
        u.name
    ));
    w.close_block();
    w.close_block();
}
```

Note: The emit_write_field / emit_read_field calls inside the union body should reuse the same dispatch functions from `message.rs`. The implementer should extract shared field codegen into a common module or make `message::emit_write_field` and `message::emit_read_field` pub(crate).

- [ ] **Step 2: Create newtype.rs**

```rust
use crate::emit::CodeWriter;
use crate::types::ts_type;
use vexil_lang::ir::{NewtypeDef, TypeRegistry};

/// Emit a TypeScript type alias for a newtype.
/// ```typescript
/// export type UserId = string;
/// ```
pub fn emit_newtype(w: &mut CodeWriter, n: &NewtypeDef, registry: &TypeRegistry) {
    let inner_ts = ts_type(&n.inner_type, registry);
    w.line(&format!("export type {} = {};", n.name, inner_ts));
    w.blank();
}
```

- [ ] **Step 3: Wire into lib.rs**

Add `pub mod union_gen;` and `pub mod newtype;`. Add match arms in `generate()`.

- [ ] **Step 4: Run cargo check**

Run: `cargo check -p vexil-codegen-ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-codegen-ts/src/union_gen.rs crates/vexil-codegen-ts/src/newtype.rs \
        crates/vexil-codegen-ts/src/lib.rs
git commit -m "feat(vexil-codegen-ts): union and newtype code generation"
```

---

## Task 17: `vexil-codegen-ts` — Golden Tests

**Files:**
- Create: `crates/vexil-codegen-ts/tests/golden.rs`
- Create: `crates/vexil-codegen-ts/tests/golden/` (directory with golden .ts files)

Golden tests follow the exact same pattern as `vexil-codegen-rust/tests/golden.rs`.

- [ ] **Step 1: Read the existing Rust golden test harness for reference**

Read: `crates/vexil-codegen-rust/tests/golden.rs`

- [ ] **Step 2: Create tests/golden.rs**

```rust
use std::fs;
use std::path::{Path, PathBuf};

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("corpus/valid")
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn golden_test(corpus_name: &str) {
    let source_path = corpus_dir().join(format!("{corpus_name}.vexil"));
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", source_path.display()));

    let result = vexil_lang::compile(&source);
    let compiled = result
        .compiled
        .unwrap_or_else(|| panic!("{corpus_name}: compilation failed: {:?}", result.diagnostics));

    let generated = vexil_codegen_ts::generate(&compiled);

    let golden_path = golden_dir().join(format!("{corpus_name}.ts"));

    if std::env::var("UPDATE_GOLDEN").is_ok() {
        fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        fs::write(&golden_path, &generated).unwrap();
        return;
    }

    let expected = fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("read golden {}: {e}\nRun with UPDATE_GOLDEN=1 to create", golden_path.display()));

    if generated != expected {
        // Line-by-line diff
        let gen_lines: Vec<&str> = generated.lines().collect();
        let exp_lines: Vec<&str> = expected.lines().collect();
        let max = gen_lines.len().max(exp_lines.len());
        let mut diffs = String::new();
        for i in 0..max {
            let g = gen_lines.get(i).unwrap_or(&"<missing>");
            let e = exp_lines.get(i).unwrap_or(&"<missing>");
            if g != e {
                diffs.push_str(&format!("line {}: expected: {e}\nline {}:      got: {g}\n", i + 1, i + 1));
            }
        }
        panic!("{corpus_name}: generated code differs from golden:\n{diffs}");
    }
}

#[test] fn test_006_message() { golden_test("006_message"); }
#[test] fn test_007_enum() { golden_test("007_enum"); }
#[test] fn test_008_flags() { golden_test("008_flags"); }
#[test] fn test_009_union() { golden_test("009_union"); }
#[test] fn test_010_newtype() { golden_test("010_newtype"); }
#[test] fn test_011_config() { golden_test("011_config"); }
#[test] fn test_016_recursive() { golden_test("016_recursive"); }
```

- [ ] **Step 3: Generate initial golden files**

Run: `UPDATE_GOLDEN=1 cargo test -p vexil-codegen-ts`
Expected: Golden `.ts` files created in `tests/golden/`.

- [ ] **Step 4: Review generated golden files**

Read each golden file and verify:
- Interfaces have correct field types
- Encode/decode functions reference correct BitWriter/BitReader methods
- Enums use string literal unions + const objects
- Recursive types include `enterNested()`/`leaveNested()` calls
- Imports are present (`import { BitReader, BitWriter } from '@vexil/runtime'`)

- [ ] **Step 5: Run golden tests without UPDATE_GOLDEN**

Run: `cargo test -p vexil-codegen-ts`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add crates/vexil-codegen-ts/tests/
git commit -m "test(vexil-codegen-ts): golden output tests for all declaration kinds"
```

---

## Task 18: `vexil-codegen-ts` — Project Generation and Cross-File Imports

**Files:**
- Modify: `crates/vexil-codegen-ts/src/lib.rs` (implement `generate_project()`)
- Create: `crates/vexil-codegen-ts/tests/project_compile_check.rs`

- [ ] **Step 1: Read the Rust backend's project generation for reference**

Read: `crates/vexil-codegen-rust/src/backend.rs` lines 30–135

- [ ] **Step 2: Implement generate_project() in lib.rs**

The flow mirrors the Rust backend:
1. Build global type name → TypeScript import path map
2. For each schema: generate code + compute relative import paths + prepend imports
3. Generate `index.ts` barrel files per namespace directory

Key difference from Rust: imports use `import { Foo, encodeFoo, decodeFoo } from './relative/path'` syntax, and barrel files use `export * from './module'`.

- [ ] **Step 3: Create project_compile_check.rs**

Test that multi-file projects (simple, diamond, mixed from `corpus/projects/`) produce valid TypeScript:
1. Compile the Vexil project
2. Generate TypeScript via `generate_project()`
3. Write to a temp directory
4. Run `tsc --noEmit --strict` on the generated code
5. Assert `tsc` exits successfully

```rust
use std::fs;
use std::process::Command;

fn check_project(project_name: &str) {
    let projects_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("corpus/projects");

    let root_path = projects_dir.join(project_name).join("root.vexil");
    let source = fs::read_to_string(&root_path).unwrap();

    let loader = vexil_lang::FilesystemLoader::new(vec![projects_dir.join(project_name)]);
    let result = vexil_lang::compile_project(&source, &root_path, &loader).unwrap();

    let backend = vexil_codegen_ts::TypeScriptBackend;
    let files = backend.generate_project(&result).unwrap();

    let tmp = tempfile::tempdir().unwrap();
    for (path, content) in &files {
        let full = tmp.path().join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, content).unwrap();
    }

    // Write a minimal tsconfig
    fs::write(tmp.path().join("tsconfig.json"), r#"{
        "compilerOptions": {
            "target": "ES2022",
            "module": "ESNext",
            "moduleResolution": "bundler",
            "strict": true,
            "noEmit": true
        }
    }"#).unwrap();

    let output = Command::new("npx")
        .arg("tsc")
        .arg("--noEmit")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run tsc");

    assert!(
        output.status.success(),
        "tsc failed for {project_name}:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test] fn project_simple() { check_project("simple"); }
#[test] fn project_diamond() { check_project("diamond"); }
#[test] fn project_mixed() { check_project("mixed"); }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p vexil-codegen-ts project_compile_check`
Expected: All pass — generated TypeScript type-checks.

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-codegen-ts/src/lib.rs crates/vexil-codegen-ts/tests/project_compile_check.rs
git commit -m "feat(vexil-codegen-ts): project generation with cross-file imports and barrel files"
```

---

## Task 19: CLI Integration — `--target typescript`

**Files:**
- Modify: `crates/vexilc/src/main.rs`
- Modify: `crates/vexilc/Cargo.toml`

- [ ] **Step 1: Add vexil-codegen-ts dependency to vexilc**

In `crates/vexilc/Cargo.toml`, add:

```toml
vexil-codegen-ts = { path = "../vexil-codegen-ts", version = "^0.2.0" }
```

- [ ] **Step 2: Add typescript target to cmd_build dispatch**

In `crates/vexilc/src/main.rs`, find the `match target` block in `cmd_build` (around the `"rust" => Box::new(vexil_codegen_rust::RustBackend)` line).

Change:
```rust
        other => {
            eprintln!("error: unknown target `{other}` (available: rust)");
            return 1;
        }
```

To:
```rust
        "typescript" => Box::new(vexil_codegen_ts::TypeScriptBackend),
        other => {
            eprintln!("error: unknown target `{other}` (available: rust, typescript)");
            return 1;
        }
```

Also add the same dispatch to `cmd_codegen` if it exists (single-file codegen command).

- [ ] **Step 3: Run cargo build**

Run: `cargo build -p vexilc`
Expected: PASS.

- [ ] **Step 4: Smoke test the CLI**

Run: `cargo run -p vexilc -- codegen corpus/valid/006_message.vexil --target typescript`
Expected: TypeScript code printed to stdout.

- [ ] **Step 5: Commit**

```bash
git add crates/vexilc/
git commit -m "feat(vexilc): add --target typescript to build and codegen commands"
```

---

## Task 20: Benchmark Suite

**Files:**
- Create: `crates/vexil-bench/Cargo.toml`
- Create: `crates/vexil-bench/src/lib.rs`
- Create: `crates/vexil-bench/src/messages.rs`
- Create: `crates/vexil-bench/benches/encode_decode.rs`
- Modify: `Cargo.toml` (workspace root — add member)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "vexil-bench"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
vexil-runtime = { path = "../vexil-runtime" }

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "encode_decode"
harness = false
```

- [ ] **Step 2: Add to workspace**

Add `"crates/vexil-bench"` to workspace members in root `Cargo.toml`. Also add it to `exclude` in the `[workspace.package]` version section if lockstep versioning is configured — the bench crate is `publish = false` and should not be version-managed.

- [ ] **Step 3: Create src/messages.rs**

```rust
//! Hand-written message types mirroring VNP-representative workloads.

use vexil_runtime::{BitReader, BitWriter, DecodeError, EncodeError};

/// Small, hot-path header with sub-byte fields.
pub struct Envelope {
    pub version: u8,    // 4 bits
    pub domain: u8,     // 4 bits
    pub msg_type: u8,   // 7 bits
    pub session_id: u32,
    pub timestamp: u64, // 48 bits
    pub msg_id: Option<u32>,
}

impl Envelope {
    pub fn encode(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_bits(self.version as u64, 4);
        w.write_bits(self.domain as u64, 4);
        w.write_bits(self.msg_type as u64, 7);
        w.flush_to_byte_boundary();
        w.write_u32(self.session_id);
        w.write_bits(self.timestamp, 48);
        w.write_bool(self.msg_id.is_some());
        w.flush_to_byte_boundary();
        if let Some(id) = self.msg_id {
            w.write_u32(id);
        }
        w.flush_to_byte_boundary();
        Ok(())
    }

    pub fn decode(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        let version = r.read_bits(4)? as u8;
        let domain = r.read_bits(4)? as u8;
        let msg_type = r.read_bits(7)? as u8;
        r.flush_to_byte_boundary();
        let session_id = r.read_u32()?;
        let timestamp = r.read_bits(48)?;
        let has_id = r.read_bool()?;
        r.flush_to_byte_boundary();
        let msg_id = if has_id { Some(r.read_u32()?) } else { None };
        r.flush_to_byte_boundary();
        Ok(Self { version, domain, msg_type, session_id, timestamp, msg_id })
    }
}

/// Medium, mixed-type render command.
pub struct DrawText {
    pub x: u16,
    pub y: u16,
    pub fg: [u8; 3],
    pub bg: [u8; 3],
    pub bold: bool,
    pub italic: bool,
    pub text: String,
}

impl DrawText {
    pub fn encode(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_u16(self.x);
        w.write_u16(self.y);
        w.write_raw_bytes(&self.fg);
        w.write_raw_bytes(&self.bg);
        w.write_bool(self.bold);
        w.write_bool(self.italic);
        w.flush_to_byte_boundary();
        w.write_string(&self.text);
        w.flush_to_byte_boundary();
        Ok(())
    }

    pub fn decode(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        let x = r.read_u16()?;
        let y = r.read_u16()?;
        let fg_bytes = r.read_raw_bytes(3)?;
        let bg_bytes = r.read_raw_bytes(3)?;
        let fg = [fg_bytes[0], fg_bytes[1], fg_bytes[2]];
        let bg = [bg_bytes[0], bg_bytes[1], bg_bytes[2]];
        let bold = r.read_bool()?;
        let italic = r.read_bool()?;
        r.flush_to_byte_boundary();
        let text = r.read_string()?;
        r.flush_to_byte_boundary();
        Ok(Self { x, y, fg, bg, bold, italic, text })
    }
}

/// Large, variable-size data transfer message.
pub struct OutputChunk {
    pub session_id: u32,
    pub pane_id: u16,
    pub sequence: u64,
    pub data: Vec<u8>,
    pub command_tag: Option<String>,
}

impl OutputChunk {
    pub fn encode(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_u32(self.session_id);
        w.write_u16(self.pane_id);
        w.write_u64(self.sequence);
        w.write_bytes(&self.data);
        w.write_bool(self.command_tag.is_some());
        w.flush_to_byte_boundary();
        if let Some(ref s) = self.command_tag {
            w.write_string(s);
        }
        w.flush_to_byte_boundary();
        Ok(())
    }

    pub fn decode(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        let session_id = r.read_u32()?;
        let pane_id = r.read_u16()?;
        let sequence = r.read_u64()?;
        let data = r.read_bytes()?;
        let has_tag = r.read_bool()?;
        r.flush_to_byte_boundary();
        let command_tag = if has_tag { Some(r.read_string()?) } else { None };
        r.flush_to_byte_boundary();
        Ok(Self { session_id, pane_id, sequence, data, command_tag })
    }
}
```

- [ ] **Step 4: Create src/lib.rs**

```rust
pub mod messages;
```

- [ ] **Step 5: Create benches/encode_decode.rs**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vexil_bench::messages::{DrawText, Envelope, OutputChunk};
use vexil_runtime::{BitReader, BitWriter};

fn bench_envelope(c: &mut Criterion) {
    let env = Envelope {
        version: 1, domain: 3, msg_type: 42,
        session_id: 1, timestamp: 1234567890123, msg_id: Some(99),
    };
    let mut w = BitWriter::new();
    env.encode(&mut w).unwrap();
    let bytes = w.finish();

    c.bench_function("Envelope encode", |b| {
        b.iter(|| { let mut w = BitWriter::new(); env.encode(&mut w).unwrap(); black_box(w.finish()); })
    });
    c.bench_function("Envelope decode", |b| {
        b.iter(|| { let mut r = BitReader::new(&bytes); black_box(Envelope::decode(&mut r).unwrap()); })
    });
}

fn bench_draw_text(c: &mut Criterion) {
    let cmd = DrawText {
        x: 10, y: 5, fg: [255, 255, 255], bg: [0, 0, 0],
        bold: true, italic: false,
        text: "fn main() { println!(\"hello\"); }".to_string(),
    };
    let mut w = BitWriter::new();
    cmd.encode(&mut w).unwrap();
    let bytes = w.finish();

    c.bench_function("DrawText encode", |b| {
        b.iter(|| { let mut w = BitWriter::new(); cmd.encode(&mut w).unwrap(); black_box(w.finish()); })
    });
    c.bench_function("DrawText decode", |b| {
        b.iter(|| { let mut r = BitReader::new(&bytes); black_box(DrawText::decode(&mut r).unwrap()); })
    });
}

fn bench_output_chunk(c: &mut Criterion) {
    let msg = OutputChunk {
        session_id: 1, pane_id: 0, sequence: 12345,
        data: vec![b'x'; 4096],
        command_tag: Some("cargo build".to_string()),
    };
    let mut w = BitWriter::new();
    msg.encode(&mut w).unwrap();
    let bytes = w.finish();

    c.bench_function("OutputChunk encode 4KiB", |b| {
        b.iter(|| { let mut w = BitWriter::new(); msg.encode(&mut w).unwrap(); black_box(w.finish()); })
    });
    c.bench_function("OutputChunk decode 4KiB", |b| {
        b.iter(|| { let mut r = BitReader::new(&bytes); black_box(OutputChunk::decode(&mut r).unwrap()); })
    });
}

fn bench_batch(c: &mut Criterion) {
    let env = Envelope {
        version: 1, domain: 6, msg_type: 1,
        session_id: 1, timestamp: 0, msg_id: None,
    };
    let cmds: Vec<DrawText> = (0..50).map(|i| DrawText {
        x: 0, y: i, fg: [200, 200, 200], bg: [30, 30, 30],
        bold: false, italic: false,
        text: format!("line {} of output from cargo build", i),
    }).collect();

    c.bench_function("Frame batch encode (1 env + 50 DrawText)", |b| {
        b.iter(|| {
            let mut w = BitWriter::new();
            env.encode(&mut w).unwrap();
            for cmd in &cmds { cmd.encode(&mut w).unwrap(); }
            black_box(w.finish());
        })
    });
}

criterion_group!(benches, bench_envelope, bench_draw_text, bench_output_chunk, bench_batch);
criterion_main!(benches);
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check -p vexil-bench`
Expected: PASS.

- [ ] **Step 7: Run benchmarks**

Run: `cargo bench -p vexil-bench`
Expected: Benchmark results printed. Record baseline numbers.

- [ ] **Step 8: Commit**

```bash
git add crates/vexil-bench/ Cargo.toml
git commit -m "feat: add encode/decode benchmark suite with VNP-representative messages"
```

---

## Task 21: Limitations and Gaps Document

**Files:**
- Create: `docs/limitations-and-gaps.md`

- [ ] **Step 1: Write the document**

After all tests and benchmarks have run, write `docs/limitations-and-gaps.md` with actual data:

```markdown
# Vexil Wire Format — Limitations, Gaps, and Room for Improvement

A living document tracking what has been validated, what is known to be
limited, and where improvements would have the most impact.

Last updated: 2026-03-27

## What Was Validated

- **Deterministic encoding:** Golden byte vectors produce identical bytes in
  both Rust and TypeScript implementations for all primitive types, sub-byte
  packing, messages, enums, unions, optionals, arrays, maps, and evolution
  scenarios.
- **Schema evolution:** Forward and backward compatibility verified for field
  append and variant addition. Trailing bytes tolerated.
- **Recursion safety:** Depth limit of 64 enforced at both encode and decode
  time. Stack overflow prevented.
- **NaN canonicalization:** All NaN inputs produce canonical quiet NaN bytes.
- **Cross-implementation compliance:** Rust and TypeScript pass the same
  vector suite.
- **Performance baseline:** [Fill in benchmark numbers after running]

## Known Limitations

- **No zero-copy decode:** BitReader copies data for strings and byte arrays.
  Applications needing zero-copy access to large payloads should consider
  a bytes-reference mode (future work).
- **No streaming / incremental decode:** The entire message must be available
  in memory before decoding starts. Not suitable for unbounded streams
  without framing.
- **No built-in compression:** Wire format is uncompressed. Applications can
  layer compression (zstd, etc.) on top.
- **No self-description:** The wire format contains no type information.
  Both sides must agree on the schema. This is by design (schema = contract)
  but means debug tooling needs the schema.

## Gaps

- **Reflection metadata:** No runtime type information emitted. Consumers
  needing schema introspection at runtime would need a separate metadata
  format.
- **Runtime validation / type guards:** Generated code does not emit
  TypeScript type guards or runtime validators. Can be layered on top of
  existing interfaces.
- **Schema registry integration:** No built-in registry or discovery. Schema
  hash provides identity but not distribution.

## Performance Characteristics

- **Wire size:** Competitive for sub-byte fields (bit-packing is more compact
  than byte-aligned formats). Equivalent for byte-aligned fields.
- **Encode throughput:** [Fill in after benchmarks]
- **Decode throughput:** [Fill in after benchmarks]
- **Comparison with protobuf:** [Fill in after benchmarks]

## Room for Improvement

Prioritized by likely consumer demand:

1. **Zero-copy byte slices** — Return `&[u8]` / `Uint8Array` views instead
   of copies for large payloads.
2. **Streaming decode** — Allow progressive decode with a framing layer.
3. **Wire size optimization** — Consider optional field presence bitsets
   for messages with many optional fields.
4. **Additional backend targets** — Python, Go, C.
5. **Compression integration** — First-class zstd or LZ4 support.
```

- [ ] **Step 2: Run all tests and benchmarks, fill in numbers**

Run: `cargo test --workspace && cd packages/runtime-ts && npx vitest run && cd ../.. && cargo bench -p vexil-bench`
Fill in the `[Fill in]` placeholders with actual numbers.

- [ ] **Step 3: Commit**

```bash
git add docs/limitations-and-gaps.md
git commit -m "docs: add limitations, gaps, and room for improvement"
```

---

## Task 22: Final Integration Verification

**Files:** No new files — verification only.

- [ ] **Step 1: Run Rust full test suite**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 2: Run TypeScript tests**

Run: `cd packages/runtime-ts && npx vitest run`
Expected: All tests pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: Clean.

- [ ] **Step 4: Run fmt**

Run: `cargo fmt --all -- --check`
Expected: Clean.

- [ ] **Step 5: Record final counts**

Document: X Rust tests, Y TypeScript tests, Z compliance vectors verified, benchmark numbers recorded.

- [ ] **Step 6: Commit final state**

```bash
git add -A
git commit -m "chore: final integration verification — all tests passing"
```
