# Schema Evolution and Versioning — Design Spec

> **Scope:** CLI compatibility checker, runtime unknown-field preservation, decode-and-discard for typed `@removed` tombstones, `SchemaHandshake` runtime helper, handshake example. Three phases (A → B → C), each independently useful.

**Goal:** Make schema evolution safe, detectable, and round-trip preserving — from CI-level breaking change detection to runtime schema negotiation.

**Architecture:** Core compatibility logic in `vexil-lang` (reusable by CLI, library, and future package manager). Runtime changes in `vexil-runtime` and `@vexil-lang/runtime`. Codegen changes in both Rust and TypeScript backends.

**Tech Stack:** Rust (workspace), TypeScript (runtime), serde_json (CLI JSON output).

**Depends on:** Spec §8-§11 (schema hash, versioning, breaking changes, trailing bytes).

---

## Phase A: CLI Compatibility Checker

### Core Library: `compat::check()`

New module in `vexil-lang`: `src/compat.rs`

```rust
pub fn check(old: &CompiledSchema, new: &CompiledSchema) -> CompatReport;
```

Compares two compiled schemas declaration-by-declaration, field-by-field, and produces a structured report.

**Changes detected** (maps to §10 table):

| Change | Classification |
|--------|---------------|
| Field added (new ordinal) | minor |
| Variant added to `@non_exhaustive` enum/union | minor |
| Flags bit position added | minor |
| Declaration added | minor |
| Field deprecated | patch |
| Field renamed (ordinal unchanged) | patch |
| `@since`, `@doc`, annotation-only changes | patch |
| Field removed | **major** |
| Field type changed | **major** |
| Field ordinal changed | **major** |
| Enum/union variant removed or ordinal changed | **major** |
| Encoding changed (@varint/@zigzag/@delta) | **major** |
| `@non_exhaustive` removed | **major** |
| Flags bit ordinal changed | **major** |
| Namespace changed | **major** |

**Report type:**

```rust
pub struct CompatReport {
    pub changes: Vec<Change>,
    pub result: CompatResult,
    pub suggested_bump: BumpKind,
}

pub struct Change {
    pub kind: ChangeKind,
    pub declaration: String,
    pub field: Option<String>,
    pub detail: String,
    pub classification: BumpKind,
}

pub enum CompatResult {
    Compatible,
    Breaking,
}

pub enum BumpKind {
    Patch,
    Minor,
    Major,
}

pub enum ChangeKind {
    FieldAdded,
    FieldRemoved,
    FieldTypeChanged,
    FieldOrdinalChanged,
    FieldRenamed,
    FieldDeprecated,
    FieldEncodingChanged,
    VariantAdded,
    VariantRemoved,
    VariantOrdinalChanged,
    DeclarationAdded,
    DeclarationRemoved,
    NamespaceChanged,
    NonExhaustiveChanged,
    AnnotationChanged,
}
```

### CLI Subcommand

```
vexilc compat old.vexil new.vexil [--format human|json]
```

**Human output (default):**
```
  ✓ field "flags" added at @2           compatible (minor)
  ✓ field "old_name" deprecated         compatible (patch)
  ✗ field "timeout" type u32 → optional<u32>  BREAKING (major)

Result: BREAKING — requires major version bump
Minimum version bump: 1.0.0 → 2.0.0
```

**JSON output (`--format json`):**
```json
{
  "changes": [
    {
      "kind": "field_added",
      "declaration": "Header",
      "field": "flags",
      "detail": "added at @2, type u16",
      "classification": "minor"
    }
  ],
  "result": "breaking",
  "suggested_bump": "major"
}
```

**Exit codes:** 0 = compatible, 1 = breaking, 2 = error.

### CLI Dependency

Add `serde` and `serde_json` to `vexilc` for JSON output. The `compat` module in `vexil-lang` does not depend on serde — it returns plain structs. The CLI handles serialization.

---

## Phase B: Runtime Evolution Support

### B.1: Unknown Field Preservation

Generated message structs gain a `_unknown` field that captures trailing bytes after all known fields are decoded. This enables round-tripping: a v1 decoder can receive v2 data, preserve the unknown fields, and re-encode without data loss.

**Rust:**

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub kind: u8,
    pub status: u8,
    pub _unknown: Vec<u8>,
}

impl Unpack for Header {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        let kind = r.read_u8()?;
        let status = r.read_u8()?;
        r.flush_to_byte_boundary();
        let _unknown = r.read_remaining();
        Ok(Self { kind, status, _unknown })
    }
}

impl Pack for Header {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_u8(self.kind);
        w.write_u8(self.status);
        w.flush_to_byte_boundary();
        if !self._unknown.is_empty() {
            w.write_raw_bytes(&self._unknown);
        }
        Ok(())
    }
}
```

**TypeScript:**

```typescript
export interface Header {
  kind: number;
  status: number;
  _unknown: Uint8Array;
}

export function decodeHeader(r: BitReader): Header {
  const kind = r.readU8();
  const status = r.readU8();
  r.flushToByteBoundary();
  const _unknown = r.readRemaining();
  return { kind, status, _unknown };
}

export function encodeHeader(v: Header, w: BitWriter): void {
  w.writeU8(v.kind);
  w.writeU8(v.status);
  w.flushToByteBoundary();
  if (v._unknown.length > 0) {
    w.writeRawBytes(v._unknown);
  }
}
```

**Runtime changes:**
- `BitReader`: add `read_remaining(&mut self) -> Vec<u8>` — returns all bytes from current position to end
- `BitReader` (TS): add `readRemaining(): Uint8Array` — same
- Default `_unknown` is empty (`Vec::new()` / `new Uint8Array(0)`)

### B.2: Typed Tombstones — Decode-and-Discard

The `@removed` tombstone syntax gains an optional type annotation so the codegen can emit read-and-discard code:

**Current syntax:**
```vexil
@removed(1, "migrated to new_name")
```

**New syntax:**
```vexil
@removed(1, "migrated to new_name") : u32
```

The type tells the codegen how many bytes to read and discard at that ordinal position. Without the type, the tombstone only reserves the ordinal — the decoder cannot skip the bytes and decoding a message that includes the removed field will fail.

**Parser change:** Accept an optional `: TypeExpr` after the tombstone closing paren.

**IR change:** `TombstoneDef` gains `original_type: Option<ResolvedType>`.

**Codegen change (both backends):** For typed tombstones, emit read-and-discard at the tombstone's ordinal position:

```rust
// Rust: ordinal @1 removed, was u32
let _ = r.read_u32()?;
```

```typescript
// TypeScript: ordinal @1 removed, was u32
r.readU32();
```

For untyped tombstones, no decode code is emitted (ordinal is just reserved).

**Spec changes:**
- §10: Note that `@removed` SHOULD include the original type for decode-and-discard
- §12 (Codegen Contract): Backends MUST emit read-and-discard for typed tombstones
- Tombstone syntax addition in §4.1 (message body items)

### B.3: Codegen Updates

Both `vexil-codegen-rust` and `vexil-codegen-ts` updated:

1. **`_unknown` field** on every message struct/interface
2. **`read_remaining()`** call at the end of every Unpack/decode
3. **`write_raw_bytes(_unknown)`** at the end of every Pack/encode
4. **Typed tombstone decode-and-discard** in ordinal order
5. **Delta encoder/decoder classes** also preserve `_unknown` (read after delta fields, write after delta fields)
6. **Golden files regenerated** for both backends

---

## Phase C: Schema Handshake Helpers

### Wire Format

32-byte BLAKE3 hash + LEB128 length-prefixed UTF-8 version string.
Typically ~37 bytes (32 hash + 1 length byte + 4-5 version chars).

### Rust API

In `vexil-runtime`:

```rust
pub struct SchemaHandshake {
    pub hash: [u8; 32],
    pub version: String,
}

impl SchemaHandshake {
    /// Create from generated constants.
    pub fn new(hash: [u8; 32], version: &str) -> Self;

    /// Encode to wire format.
    pub fn encode(&self) -> Vec<u8>;

    /// Decode from wire format.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError>;

    /// Compare against a remote handshake.
    pub fn check(&self, remote: &SchemaHandshake) -> HandshakeResult;
}

pub enum HandshakeResult {
    /// Hashes match — identical schemas.
    Match,
    /// Hashes differ but could be a compatible version change.
    VersionMismatch {
        local_version: String,
        remote_version: String,
        local_hash: [u8; 32],
        remote_hash: [u8; 32],
    },
}
```

`check()` compares hashes first. If hashes match, return `Match`. If hashes differ, return `VersionMismatch` with both versions and hashes. The application decides whether the mismatch is acceptable (using the CLI compat checker, or its own policy).

### TypeScript API

In `@vexil-lang/runtime`:

```typescript
export class SchemaHandshake {
  constructor(public hash: Uint8Array, public version: string);
  encode(): Uint8Array;
  static decode(bytes: Uint8Array): SchemaHandshake;
  check(remote: SchemaHandshake): HandshakeResult;
}

export type HandshakeResult =
  | { kind: 'match' }
  | { kind: 'version_mismatch'; localVersion: string; remoteVersion: string;
      localHash: Uint8Array; remoteHash: Uint8Array };
```

---

## Handshake Example

Update the `examples/system-monitor/` to demonstrate schema handshake:

1. Browser connects WebSocket
2. Browser sends `SchemaHandshake.encode()` as first binary frame (hash + version from generated constants)
3. Rust server receives, decodes, calls `check()` against its own hash
4. If `Match` → start streaming telemetry
5. If `VersionMismatch` → send an error frame with a human-readable message, close connection
6. Dashboard shows "Schema mismatch — please refresh" when server rejects

This demonstrates: what happens when the server updates its schema but the browser has a stale cached `bundle.js`.

---

## Testing

### Phase A Tests
- Unit tests for `compat::check()` covering every change kind from §10 table
- CLI integration tests: run `vexilc compat` on corpus evolution schemas, verify human and JSON output
- Exit code tests: compatible → 0, breaking → 1

### Phase B Tests
- Round-trip test: encode with v2 schema → decode with v1 code → re-encode → bytes match original
- Typed tombstone test: schema with `@removed(1, "reason") : u32` → decoder skips 4 bytes at ordinal 1
- `_unknown` default test: fresh struct has empty `_unknown`, encodes identically to current behavior
- Compliance vectors: add evolution round-trip vectors to `compliance/vectors/evolution.json`

### Phase C Tests
- Encode/decode round-trip for SchemaHandshake
- `check()` with matching hashes → `Match`
- `check()` with different hashes → `VersionMismatch`
- Cross-language: Rust encode → TS decode and vice versa

---

## Decision Log

### Unknown fields: struct field vs wrapper type

**Chosen:** `_unknown: Vec<u8>` directly on the struct.

**Rejected:** `Decoded<T>` wrapper type.

**Rationale:** Simpler for round-tripping — `value.pack()` automatically includes unknown bytes. No trait signature changes. Matches protobuf's approach. The extra field is easy to ignore in application code.

### Tombstone types: required vs optional

**Chosen:** Optional type annotation on `@removed`.

**Rejected:** Requiring type on all tombstones.

**Rationale:** Backward compatible with existing schemas that have untyped tombstones. Untyped tombstones still reserve the ordinal — they just can't enable decode-and-discard. Migration path: add types to existing tombstones when you need interop.

### Handshake: full protocol vs helpers

**Chosen:** Encode/decode/compare helpers in the runtime.

**Rejected:** Full negotiation protocol with version ranges and fallback.

**Rationale:** Vexil is a schema language, not a protocol framework. The helpers provide identity — the application decides policy. The CLI compat checker provides the "is this safe?" answer separately.

### Unknown fields in PartialEq

**Chosen:** `_unknown` participates in `PartialEq`.

**Rationale:** Two messages with identical known fields but different unknown trailing bytes carry different data. Treating them as equal would be semantically wrong — a proxy that strips unknown bytes should produce a different value than one that preserves them.
