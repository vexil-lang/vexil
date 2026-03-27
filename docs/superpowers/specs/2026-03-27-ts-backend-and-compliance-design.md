# TypeScript Backend and Compliance Infrastructure Design

> **Scope:** Unified plan covering encoding edge-case spec additions, cross-implementation compliance vectors, `@vexil/runtime` TypeScript package, `vexil-codegen-ts` Rust crate, CLI integration, benchmark suite, and limitations documentation. Subsumes and unifies the Phase 0 protocol validation plan (MALT) with the TypeScript backend spec.

**Goal:** Ship a complete TypeScript backend for Vexil — codegen crate, runtime package, and compliance infrastructure — while simultaneously validating the wire format's determinism, evolution, and performance characteristics that downstream consumers (starting with MALT/VNP) depend on.

**Architecture:** One feature branch (`feature/ts-backend-and-compliance`) with 8 task groups in strict dependency order. All TypeScript work lives inside the vexil-lang monorepo at `packages/runtime-ts/`. Compliance vectors in `compliance/vectors/` serve as the shared contract between Rust and TypeScript implementations.

**Tech Stack:** Rust (vexil-lang workspace), TypeScript (runtime package), npm (`@vexil/runtime`), Criterion (benchmarks), Vitest (TS tests).

**Depends on:** SDK Architecture (v0.2.0) — specifically `CodegenBackend` trait, `CodegenError`, Tier 1 API.

---

## 1. Task Groups and Dependencies

```
1. Spec §11 + Corpus Schemas
   ↓
2. Rust Runtime Changes (depth tracking, trailing byte tolerance)
   ↓
3. Codegen Depth Emission (vexil-codegen-rust recursive type support)
   ↓                          ↘
4. Compliance Vectors          7. Benchmark Suite (can start after 2)
   ↓
5. @vexil/runtime TypeScript Package
   ↓
6. vexil-codegen-ts + CLI Integration
   ↓
8. Limitations & Gaps Document (written last, after all evidence collected)
```

All work lands on a single feature branch with clean commit boundaries per task group, structured so individual groups could be cherry-picked if needed.

---

## 2. Spec Additions (§11 Encoding Edge Cases)

Append a normative §11 to `spec/vexil-spec.md` covering edge cases that any conformant implementation must handle.

### 11.1 Empty Optionals

`optional<T>` with no value encodes as a single 0 bit. With a value: 1 bit + T's encoding.

Nested optionals (`optional<optional<T>>`):
- None → `0` (1 bit)
- Some(None) → `1 0` (2 bits)
- Some(Some(v)) → `1 1` + v's encoding

### 11.2 Zero-Length Payloads

A message with zero fields encodes as zero bytes. A union variant with no fields encodes as: discriminant (LEB128) + length 0 (LEB128).

### 11.3 Maximum Recursion Depth

Recursive types (self-referencing via optional or array) have a maximum nesting depth of 64 at encode/decode time. Exceeding returns `EncodeError::MaxDepthExceeded` / `DecodeError::MaxDepthExceeded`.

### 11.4 Trailing Bytes

After consuming all declared fields, any remaining bytes are ignored. Decoders MUST NOT reject messages with trailing bytes. This enables forward compatibility (v2 encoder appends fields, v1 decoder skips them).

### 11.5 Sub-Byte Boundary at Message End

After encoding all fields, encoder calls `flush_to_byte_boundary()`. Padding bits MUST be zero. Decoder flushes after reading all known fields, before checking for trailing bytes.

### 11.6 Union Discriminant Overflow

Unknown discriminant in a `@non_exhaustive` union: decode as `Unknown { discriminant, data }`. In an exhaustive union: return `DecodeError::UnknownVariant`. Length-prefixed payload enables skipping.

### 11.7 NaN Canonicalization

All f32 NaN → `0x7FC00000` (canonical quiet NaN). All f64 NaN → `0x7FF8000000000000`. Signaling NaN, negative NaN, and NaN with payload all map to canonical qNaN before encoding.

### 11.8 Negative Zero

`-0.0` is preserved on the wire (distinct bit pattern from `+0.0`).

### 11.9 String Encoding Errors

Strings use UTF-8. Invalid UTF-8 at encode → `EncodeError::InvalidUtf8`. Invalid UTF-8 at decode → `DecodeError::InvalidUtf8`. Bytes fields have no encoding restriction.

### 11.10 Schema Evolution Compatibility Rules

- **Field append:** v1→v2 decoder fills new field with default; v2→v1 decoder ignores trailing bytes.
- **Variant addition (`@non_exhaustive`):** v2→v1 decoder reads unknown discriminant as `Unknown`.
- **Deprecation:** Source-level only, no wire change. Ordinal remains reserved.
- **Required→optional:** **BREAKING** — inserts presence bit, changes wire layout. Requires wire version bump.

### Corpus Schemas

8 new schemas (019–026):

| File | Spec Ref | Purpose |
|------|----------|---------|
| `019_evolution_append_field.vexil` | §11.10 | Field appended to message |
| `020_evolution_add_variant.vexil` | §11.10 | Variant added to `@non_exhaustive` union |
| `021_empty_optionals.vexil` | §11.1 | Empty and nested optional encoding |
| `022_nested_schemas.vexil` | §4.1 | Nested message references, arrays of messages |
| `023_recursive_depth.vexil` | §11.3 | Self-recursive and mutual recursive types |
| `024_zero_length_payload.vexil` | §11.2 | Empty messages, empty union variants |
| `025_evolution_deprecate.vexil` | §11.10 | Deprecated field (no wire change) |
| `026_required_to_optional.vexil` | §11.10 | Breaking change: required→optional wire difference |

---

## 3. Rust Runtime Changes

### Recursion Depth Tracking

Add to both `BitWriter` and `BitReader`:

```rust
const MAX_DEPTH: u32 = 64;

// New struct field:
depth: u32,  // initialized to 0

// New methods:
pub fn enter_nested(&mut self) -> Result<(), Error> { ... }
pub fn leave_nested(&mut self) { ... }
pub fn depth(&self) -> u32 { ... }
```

New error variants: `EncodeError::MaxDepthExceeded`, `DecodeError::MaxDepthExceeded`.

### Trailing Byte Tolerance

Verify and test that `BitReader` does not error when dropped with unread bytes. This is the §11.4 guarantee for schema evolution.

### Codegen Impact

`vexil-codegen-rust` emits `enter_nested()`/`leave_nested()` calls in Pack/Unpack impls for types that reference themselves through optional/array. Golden files regenerated with `UPDATE_GOLDEN=1`.

---

## 4. Compliance Vectors

New directory: `compliance/vectors/`

### Vector Format

Standard vectors:
```json
{
  "name": "human-readable test name",
  "schema": "inline Vexil schema text",
  "type": "message/enum/union type name to encode",
  "value": { "field": "value" },
  "expected_bytes": "hex-encoded expected output",
  "notes": "optional explanation"
}
```

Evolution vectors use a dual-schema format:
```json
{
  "name": "v1_encode_v2_decode_appended_field",
  "schema_v1": "namespace test.evo\nmessage M { x @0 : u32 }",
  "schema_v2": "namespace test.evo\nmessage M { x @0 : u32  y @1 : u16 }",
  "type": "M",
  "value_v1": { "x": 42 },
  "encoded_v1": "2a000000",
  "decoded_as_v2": { "x": 42, "y": 0 },
  "notes": "v2 decoder fills y with default when reading v1-encoded bytes"
}
```

### Vector Files

| File | Coverage |
|------|----------|
| `primitives.json` | bool, u8/u16/u32, i32, f32 (NaN), f64 (-0.0), string |
| `sub_byte.json` | Sub-byte packing, cross-byte boundaries |
| `messages.json` | Empty message, multi-field, mixed types, nested |
| `enums.json` | Enum encoding/decoding |
| `unions.json` | Known variants, non-exhaustive unknown |
| `optionals.json` | None, Some, nested optionals |
| `arrays_maps.json` | Empty/populated arrays, maps |
| `evolution.json` | Cross-version vectors (v1↔v2) |

### Rust Compliance Validator

Integration test in `vexil-codegen-rust` that:
1. Compiles each vector's schema and verifies it succeeds
2. Encodes known values via `BitWriter` and asserts byte-identical output against `expected_bytes`
3. Decodes `expected_bytes` via `BitReader` and asserts value equality

---

## 5. `@vexil/runtime` TypeScript Package

Located at `packages/runtime-ts/`. Published to npm as `@vexil/runtime`.

### API Surface

```typescript
export class BitReader {
  constructor(buf: Uint8Array);
  readBool(): boolean;
  readBits(count: number): number;
  readU8(): number;
  readU16(): number;
  readU32(): number;
  readU64(): bigint;
  readI8(): number;
  readI16(): number;
  readI32(): number;
  readI64(): bigint;
  readF32(): number;
  readF64(): number;
  readString(): string;
  readBytes(): Uint8Array;
  readUuid(): string;
  readTimestamp(): Date;
  readVarint(): number;
  readVarint64(): bigint;
  readZigZag(): number;
  readZigZag64(): bigint;
  flushToByteBoundary(): void;
  enterNested(): void;
  leaveNested(): void;
  remainingBytes(): number;
}

export class BitWriter {
  // Mirror of BitReader write methods
  finish(): Uint8Array;
}

export type SchemaHash = Uint8Array & { readonly length: 32 };
```

### Properties

- All methods follow Vexil's LSB-first bitpack wire format
- Pure TypeScript, zero dependencies
- NaN canonicalization in `writeF32`/`writeF64`
- Recursion depth limit (64) enforced in `enterNested()`
- Consistent naming with Rust runtime: `BitReader`/`BitWriter`

### Testing

- Unit tests (Vitest) for each primitive read/write and edge case
- Compliance tests reading `compliance/vectors/*.json` — byte-identical encode/decode
- Cross-implementation: Rust encode → TS decode and TS encode → Rust decode via shared vectors

### Tooling

- TypeScript, ES2022 target
- Vitest for tests
- `npm publish` from `packages/runtime-ts/`
- CI: `npx vitest run` as separate step alongside `cargo test --workspace`

---

## 6. `vexil-codegen-ts` Rust Crate

New workspace member implementing `CodegenBackend` for TypeScript. Depends on `vexil-lang` only.

### Generated Code Shape

**Messages:**
```typescript
export interface Id {
  value: string;
}

export function encodeId(v: Id, w: BitWriter): void {
  w.writeUuid(v.value);
}

export function decodeId(r: BitReader): Id {
  return { value: r.readUuid() };
}
```

**Enums:** String literal union + const object.
```typescript
export type Status = 'Active' | 'Inactive';
export const Status = {
  Active: 'Active' as const,
  Inactive: 'Inactive' as const,
};
```

**Flags:** `number` (bitfield) + named constants.
**Unions:** Discriminated union tagged by variant name.
**Newtypes:** Type alias (`type UserId = string`).
**Config:** Interface, no codec.

### Type Mapping

| Vexil Type | TypeScript Type |
|---|---|
| `bool` | `boolean` |
| `u8`, `u16`, `u32`, `i8`, `i16`, `i32` | `number` |
| `u64`, `i64` | `bigint` |
| `f32`, `f64` | `number` |
| `string` | `string` |
| `bytes` | `Uint8Array` |
| `uuid` | `string` |
| `timestamp` | `Date` |
| `Optional<T>` | `T \| null` |
| `Array<T>` | `T[]` |
| `Map<K,V>` | `Map<K,V>` |
| `Result<T,E>` | `{ ok: T } \| { err: E }` |
| `SubByte(bits:N)` | `number` |
| Enum | String literal union + const object |
| Flags | `number` + named constants |
| Union | Discriminated union |
| Newtype | Type alias |
| Config | Interface (no codec) |

### Cross-File Imports

Backend computes relative import paths from namespace structure. Barrel `index.ts` files generated per namespace directory.

```typescript
// diamond/left.ts importing from diamond/base
import { Id, encodeId, decodeId } from './base';
```

### CodegenBackend Implementation

```rust
pub struct TypeScriptBackend;

impl CodegenBackend for TypeScriptBackend {
    fn name(&self) -> &str { "typescript" }
    fn file_extension(&self) -> &str { "ts" }
    fn generate(&self, compiled: &CompiledSchema) -> Result<String, CodegenError>;
    fn generate_project(&self, result: &ProjectResult) -> Result<BTreeMap<PathBuf, String>, CodegenError>;
}
```

### Testing

- Unit tests: type mapping correctness, import path computation
- Golden tests: snapshot generated code against corpus schemas
- Integration: corpus projects → TypeScript → `tsc --noEmit`
- Wire round-trip: Rust encode → TS decode and vice versa via compliance vectors

---

## 7. CLI Integration

`vexilc build` gains `--target typescript`:

```
vexilc build root.vexil --include ./schemas --output ./generated --target typescript
```

Dispatch adds one match arm:
```rust
"typescript" => Box::new(vexil_codegen_ts::TypeScriptBackend),
```

Default target remains `rust`. Existing file-writing helper handles the `BTreeMap<PathBuf, String>` output.

---

## 8. Benchmark Suite

New `vexil-bench` crate (workspace member, `publish = false`). Vexil-lang product quality infrastructure.

### Message Shapes

| Message | Profile | Characteristics |
|---------|---------|----------------|
| Envelope | Small, hot path | Sub-byte packed fields (4-bit version, 4-bit domain, 7-bit msg_type), u32, 48-bit timestamp, optional u32 |
| DrawText | Medium, mixed | u16 coordinates, RGB byte arrays, bools, string |
| OutputChunk | Large, variable | u32, u16, u64, 4KiB byte payload, optional string |
| Frame batch | Composite | 1 Envelope + 50 DrawText commands |

Representative of any protocol using Vexil primitives. Happen to mirror VNP-like workloads.

### Comparison

Protobuf via `prost` with equivalent `.proto` definitions. Cap'n Proto and FlatBuffers via published benchmark data with cited sources.

### Tooling

Criterion with HTML reports. `cargo bench -p vexil-bench`.

---

## 9. Limitations, Gaps, and Room for Improvement

Living document at `docs/limitations-and-gaps.md`, written after all evidence is collected.

**Structure:**
- **What was validated** — determinism, schema evolution, recursion safety, cross-impl compliance, performance baseline
- **Known limitations** — no zero-copy decode, no streaming/incremental decode, no compression
- **Gaps** — reflection metadata, runtime validation/type guards, schema registry
- **Performance characteristics** — where bitpack wins (determinism, wire size for sub-byte) vs. where alternatives excel (large payloads, zero-copy)
- **Room for improvement** — prioritized by consumer demand

Updated as new consumers stress-test the format.

---

## 10. Decision Log

### Single TS runtime serving both compliance and codegen

**Chosen:** One `@vexil/runtime` at `packages/runtime-ts/` serving Phase 0 compliance testing and as the runtime dependency for generated TypeScript code.

**Rejected:** Separate throwaway `vexil-ts/` for validation and a later `@vexil/runtime` for production.

**Rationale:** Wire primitives are identical. Building them twice is waste. Compliance vectors from validation become the wire round-trip test suite for the codegen backend.

### TS package inside vexil-lang monorepo

**Chosen:** `packages/runtime-ts/` inside vexil-lang.

**Rejected:** Separate `orix/vexil-ts/` repository.

**Rationale:** It's a vexil-lang deliverable. Codegen crate needs to know the runtime API. Lockstep versioning keeps things simple. `npm publish` works fine from a subdirectory. Extraction possible later if needed.

### Consistent naming across backends

**Chosen:** `BitReader`/`BitWriter` in both Rust and TypeScript runtimes.

**Rejected:** `BufReader`/`BufWriter` for TypeScript (from original TS backend spec).

**Rationale:** Language shouldn't differ on naming. Consistent naming reduces cognitive overhead when working across backends.

### Benchmarks as vexil-lang product quality

**Chosen:** Benchmark suite lives in vexil-lang permanently as `vexil-bench` crate.

**Rejected:** MALT-specific benchmarking effort.

**Rationale:** Performance is a product quality of vexil-lang itself. MALT is the first consumer that validates it, not the owner.

### Limitations doc instead of go/no-go gate

**Chosen:** Living `docs/limitations-and-gaps.md` documenting validated properties, known limitations, gaps, and room for improvement.

**Rejected:** Formal go/no-go justification document.

**Rationale:** VNP commitment to Vexil is already made. A limitations doc is more useful as a living roadmap than a binary gate.
