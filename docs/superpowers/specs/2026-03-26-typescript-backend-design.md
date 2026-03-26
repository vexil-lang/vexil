# TypeScript Backend Design

> **Scope:** TypeScript code generation backend for the Vexil schema language. Covers type mapping, generated code shape, `@vexil/runtime` npm package, and integration with the SDK's `CodegenBackend` trait. Does NOT cover runtime validation, reflection metadata, or IR serialization for external tools.

**Goal:** Ship a Rust-based TypeScript codegen backend (`vexil-codegen-ts`) that implements `CodegenBackend`, producing idiomatic TypeScript interfaces and codec functions. Validates the SDK trait design with a real non-Rust backend.

**Architecture:** Rust crate emitting TypeScript strings, compiled into `vexilc`. Generated code depends on `@vexil/runtime` npm package for wire primitives. Backend is fully independent from `vexil-codegen-rust`.

**Tech Stack:** Rust (codegen crate), TypeScript (runtime package), npm (`@vexil/runtime`).

**Depends on:** SDK Architecture Design (2026-03-26-sdk-architecture-design.md) — specifically the `CodegenBackend` trait, `CodegenError` type, and Tier 1 API.

---

## 1. Crate Structure

```
vexil-codegen-ts    — Rust crate, implements CodegenBackend, emits TypeScript
@vexil/runtime      — npm package, wire primitives (varint, zigzag, buffer r/w)
```

`vexil-codegen-ts` depends on `vexil-lang` for IR types and the `CodegenBackend` trait. No dependency on `vexil-codegen-rust`. The two backends share no code.

`@vexil/runtime` is a hand-written TypeScript package developed in `packages/runtime-ts/`. It is published to npm independently and is a dependency of generated code, not of the codegen crate.

---

## 2. Type Mapping

| Vexil Type | TypeScript Type | Notes |
|---|---|---|
| `bool` | `boolean` | |
| `u8`, `u16`, `u32`, `i8`, `i16`, `i32` | `number` | |
| `u64`, `i64` | `bigint` | Correctness over convenience |
| `f32`, `f64` | `number` | |
| `string` | `string` | |
| `bytes` | `Uint8Array` | |
| `uuid` | `string` | Formatted as standard UUID string |
| `timestamp` | `Date` | |
| `Optional<T>` | `T \| null` | |
| `Array<T>` | `T[]` | |
| `Map<K,V>` | `Map<K,V>` | Supports non-string keys |
| `Result<T,E>` | `{ ok: T } \| { err: E }` | Discriminated union |
| `SubByte(bits:N)` | `number` | Range-limited, codec enforces bounds |
| Enum | String literal union + const object | `type Direction = 'Active' \| 'Inactive'` |
| Flags | `number` (bitfield) + named constants | Bitwise operations |
| Union | Discriminated union | Tagged by variant name |
| Newtype | Type alias | `type UserId = string` |
| Config | Interface | Same shape as message, no codec |

### Key type decisions

- **64-bit integers → `bigint`:** Prevents silent precision loss above 2^53. Consumers who need JSON compatibility handle `bigint` → `number`/`string` conversion themselves.
- **Maps → `Map<K,V>`:** Vexil supports non-string keys. `Record<string, V>` would silently break for `Map<u32, V>` or `Map<SomeEnum, V>`.

---

## 3. Generated Code Shape

### Single file example

For a schema:
```
namespace diamond.base
message Id { value @0 : uuid }
enum Status { Active @0  Inactive @1 }
```

Generated `diamond/base.ts`:
```typescript
import { BufReader, BufWriter } from '@vexil/runtime';

// --- Types ---

export interface Id {
  value: string;
}

export type Status = 'Active' | 'Inactive';
export const Status = {
  Active: 'Active' as const,
  Inactive: 'Inactive' as const,
};

// --- Codecs ---

export function encodeId(v: Id, w: BufWriter): void {
  w.writeUuid(v.value);
}

export function decodeId(r: BufReader): Id {
  return { value: r.readUuid() };
}

export function encodeStatus(v: Status, w: BufWriter): void {
  w.writeU8(v === 'Active' ? 0 : 1);
}

export function decodeStatus(r: BufReader): Status {
  return r.readU8() === 0 ? 'Active' : 'Inactive';
}
```

### Cross-file imports

`diamond/left.ts` importing from `diamond/base`:
```typescript
import { Id, encodeId, decodeId } from './base';
```

The backend computes relative import paths from namespace structure. Import strategy is entirely owned by the backend (per SDK spec decision).

### Barrel files

`diamond/index.ts`:
```typescript
export * from './base';
export * from './left';
export * from './right';
```

Generated for each namespace directory. Enables `import { Id, LeftNode } from './diamond'`.

### File layout

Namespaces mirror to directories (same convention as Rust backend):
```
diamond/base.ts
diamond/left.ts
diamond/right.ts
diamond/index.ts
```

---

## 4. `@vexil/runtime` npm Package

Minimal surface — only what generated code needs.

```typescript
export class BufReader {
  constructor(buf: Uint8Array);
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
  readBool(): boolean;
  readString(): string;
  readBytes(): Uint8Array;
  readUuid(): string;
  readTimestamp(): Date;
  readVarint(): number;
  readVarint64(): bigint;
  readZigZag(): number;
  readZigZag64(): bigint;
}

export class BufWriter {
  // Mirror of BufReader methods (writeU8, writeU16, etc.)
  finish(): Uint8Array;
}

export type SchemaHash = Uint8Array & { readonly length: 32 };
```

### Properties

- All methods follow Vexil's LSB-first bitpack wire format
- Pure TypeScript, zero dependencies
- Published to npm as `@vexil/runtime`
- Hand-written and independently tested (not generated)
- Developed in `packages/runtime-ts/`

---

## 5. CodegenBackend Implementation

```rust
pub struct TypeScriptBackend;

impl CodegenBackend for TypeScriptBackend {
    fn name(&self) -> &str { "typescript" }
    fn file_extension(&self) -> &str { "ts" }

    fn generate(&self, compiled: &CompiledSchema) -> Result<String, CodegenError> {
        // Single-file generation: interfaces + codecs
    }

    fn generate_project(
        &self,
        result: &ProjectResult,
    ) -> Result<BTreeMap<PathBuf, String>, CodegenError> {
        // 1. For each schema in topo order:
        //    - Generate types + codecs
        //    - Compute relative import paths from namespace structure
        //    - Prepend import statements
        // 2. Generate index.ts barrel files per directory
        // Returns: path → content map
    }
}
```

**Tier 1 API consumed:**
- `ProjectResult` with all `CompiledSchema`s in topological order
- `TypeRegistry` iteration: `declarations`, `get(TypeId)` → `TypeDef`
- `ResolvedType` matching for type mapping
- Namespace for computing relative import paths

No Tier 2 access needed. This validates the SDK surface is sufficient.

---

## 6. CLI Integration

`vexilc build` gains `--target typescript`:

```
vexilc build root.vexil --include ./schemas --output ./generated --target typescript
```

Dispatch in `cmd_build`:
```rust
let backend: Box<dyn CodegenBackend> = match target {
    "rust" => Box::new(vexil_codegen_rust::RustBackend),
    "typescript" => Box::new(vexil_codegen_ts::TypeScriptBackend),
    _ => return Err(...)
};
let files = backend.generate_project(&project_result)?;
// Write files to output directory (shared helper)
```

Default target remains `rust`.

---

## 7. Testing Strategy

### Unit tests (in `vexil-codegen-ts`)
- Type mapping: each Vexil type → expected TypeScript string
- Import path computation: namespace pairs → relative path
- Generated code shape: snapshot tests against corpus schemas

### Integration tests
- Compile corpus projects → TypeScript → `tsc --noEmit` (type-checks without emitting)
- Verifies generated code is syntactically and type-valid

### Wire compatibility tests
- Rust encode → TypeScript decode round-trip
- TypeScript encode → Rust decode round-trip
- Ensures both backends produce/consume identical wire bytes
- Requires `@vexil/runtime` to be functional first

### Development sequence
1. `@vexil/runtime` npm package (hand-written, tested with Jest/Vitest)
2. `vexil-codegen-ts` Rust crate (implements `CodegenBackend`)
3. Integration test: corpus → TypeScript → `tsc --noEmit`
4. Wire-level round-trip tests

---

## 8. Decision Log

### Output style: interfaces vs classes vs hybrid

**Chosen:** Interfaces + standalone codec functions (Option A).

**Rejected alternatives:**
- **Classes with methods:** Forces instantiation patterns on consumers. Doesn't compose well with existing TypeScript code that uses plain objects. Tree-shaking is harder with classes.
- **Interfaces + codec classes:** Adds unnecessary class wrapper (`FooCodec.encode()` vs `encodeFoo()`). No practical benefit over free functions.

**Rationale:** Interfaces are the most idiomatic TypeScript. They represent pure data shapes, compose with spread/destructuring, and tree-shaking eliminates unused codec functions automatically.

### 64-bit integers: `bigint` vs `number`

**Chosen:** `bigint`.

**Rejected:** `number` (loses precision above 2^53).

**Rationale:** Correctness over convenience. Silent precision loss is a bug. Consumers who need JSON compatibility can convert explicitly. This matches the Vexil philosophy of being explicit about wire representation.

### Maps: `Map<K,V>` vs `Record<string,V>`

**Chosen:** `Map<K,V>`.

**Rejected:** `Record<string, V>` (only works with string keys).

**Rationale:** Vexil supports non-string map keys (integers, enums). `Record` would silently break for `Map<u32, V>`. `Map` is correct for all key types.

### Runtime: npm package vs inline vs bundled file

**Chosen:** `@vexil/runtime` npm package (Option A).

**Rejected alternatives:**
- **Inline everything:** Duplicates wire primitives in every generated file. Bloats output.
- **Bundled `_runtime.ts`:** Duplicated across projects. No independent versioning or testing.

**Rationale:** Standard pattern (protobuf-ts, buf). Keeps generated code small. Runtime is independently versioned and testable. Small package with zero dependencies.

### Backend implementation: Rust crate vs TypeScript tool vs both

**Chosen:** Rust crate compiled into `vexilc` (Option A).

**Rejected alternatives:**
- **TypeScript tool reading serialized IR:** Requires designing an IR serialization format that doesn't exist yet. Two-step pipeline is more complex for users.
- **Both:** Over-engineering. No concrete consumer for a TS-native codegen tool.

**Rationale:** The codegen is straightforward string emission — no reason it can't be Rust. One binary (`vexilc`) handles all targets. If we later need a TS-native tool, we design the IR export format then.

### Scope: types-only vs validation vs reflection

**Chosen:** Types + codecs only (Approach A).

**Rejected alternatives:**
- **Runtime validation (`validate()` type guards):** More codegen complexity. Can be added later as a `@vexil/runtime` utility consuming the same interfaces.
- **Reflection metadata:** No concrete consumer. Scope creep. Can be added without changing the codegen architecture.

**Rationale:** Ship the minimum that validates the SDK trait surface. Validation and reflection are additive features that don't require codegen changes — they layer on top of existing interfaces.
