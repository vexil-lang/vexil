# Delta Streaming and Cross-Language Delta Support

> **Scope:** `@delta` on message declarations (syntactic sugar), TypeScript delta codegen, compliance vectors for delta encoding, system-monitor example update with generated TS decoder.

**Goal:** Complete cross-language delta encoding support. Add `@delta` message-level annotation, implement delta encoder/decoder generation in the TypeScript backend, add compliance vectors, and update the system-monitor example to use generated code with delta encoding for a real wire-size improvement.

**Architecture:** Spec addition (desugaring rule) + TS codegen (stateful encoder/decoder classes) + compliance vectors (multi-frame stream format) + example update (esbuild bundle, real generated code).

**Tech Stack:** Rust (vexil-lang workspace), TypeScript (codegen + runtime), esbuild (example bundling).

**Depends on:** TypeScript backend (v0.2.4), `@delta` spec and Rust codegen (already shipped).

---

## 1. Spec Addition — `@delta` on Messages

Add to §13.4 (Encoding annotations), after the existing `@delta` field-level text:

> `@delta` may be applied to a message declaration. This is equivalent to
> annotating every eligible field with `@delta`. Fields whose types are not
> valid for `@delta` (string, bytes, arrays, maps, optionals, enums, flags,
> unions, configs, named message types) are silently skipped.
>
> The desugaring happens during IR lowering — the wire format is identical
> to per-field `@delta` annotations. No new wire encoding is introduced.

No wire format change. No RFC needed. The parser accepts `@delta` on message
declarations, lowering expands it to per-field annotations on eligible fields,
and everything downstream (codegen, runtime) works unchanged.

---

## 2. Parser and Lowering Changes

**Parser:** Accept `@delta` as a declaration-level annotation on `message`
declarations. Currently `@delta` is only valid at field level. The parser
change is small — allow `@delta` in the annotation set for messages.

**Lowering (AST → IR):** When a message has `@delta`, iterate its fields.
For each field whose resolved type is valid for `@delta` (primitives
`u8`–`u64`, `i8`–`i64`, sub-byte `uN`/`iN`, `f32`, `f64`), set the
field's encoding to `Encoding::Delta(Box::new(existing_encoding))`. Fields
with ineligible types are left unchanged. Fields that already have an
explicit `@delta` annotation are left unchanged (no double-wrapping).

---

## 3. TypeScript Delta Codegen

`vexil-codegen-ts` generates stateful encoder/decoder classes for messages
with `@delta` fields, matching the pattern in `vexil-codegen-rust/src/delta.rs`.

### Generated Output

For a message with `@delta` fields, in addition to the existing stateless
`encode{Name}`/`decode{Name}` functions, the backend emits:

```typescript
export class {Name}Encoder {
  private prev{Field}: {Type} = {zero};
  // ... one prev per @delta field

  encode(v: {Name}, w: BitWriter): void {
    // For @delta fields: compute delta, write delta, update prev
    // For non-delta fields: write normally (same as stateless encode)
  }

  reset(): void {
    // Reset all prev fields to zero/0.0
  }
}

export class {Name}Decoder {
  private prev{Field}: {Type} = {zero};

  decode(r: BitReader): {Name} {
    // For @delta fields: read delta, reconstruct value, update prev
    // For non-delta fields: read normally
  }

  reset(): void { ... }
}
```

### Delta Computation

- **Unsigned integers (u8–u64, uN):** `delta = current - previous` using
  wrapping subtraction. Reconstruct with wrapping addition.
- **Signed integers (i8–i64, iN):** Standard subtraction/addition.
- **Floats (f32, f64):** Standard subtraction/addition.
- **Composition with `@varint`:** Delta is computed first, then LEB128-encoded.
- **Composition with `@zigzag`:** Delta is computed first, then ZigZag + LEB128.
- **Initial previous value:** `0` for integers, `0n` for bigint, `0.0` for floats.

### Implementation Location

New file: `crates/vexil-codegen-ts/src/delta.rs`

Follows the same structure as `crates/vexil-codegen-rust/src/delta.rs`:
- `emit_delta_encoder()` — generates the encoder class
- `emit_delta_decoder()` — generates the decoder class
- Helper functions: `is_delta()`, `is_float()`, `zero_literal()`, `strip_delta()`

Wired into `lib.rs` — when generating a message, check if any fields have
`Encoding::Delta`. If so, call `emit_delta_encoder`/`emit_delta_decoder`
after the normal `emit_message`.

---

## 4. Compliance Vectors for Delta

New file: `compliance/vectors/delta.json`

### Stream Vector Format

Delta vectors use a multi-frame format since delta encoding is stateful
across messages:

```json
{
  "name": "delta_i64_three_frames",
  "schema": "namespace test.delta\nmessage M { @delta\n  v @0 : i64 }",
  "type": "M",
  "frames": [
    { "value": { "v": 1000 }, "expected_bytes": "e807000000000000" },
    { "value": { "v": 2000 }, "expected_bytes": "e807000000000000" },
    { "value": { "v": 2005 }, "expected_bytes": "0500000000000000" }
  ],
  "notes": "Delta from 0->1000=1000, 1000->2000=1000, 2000->2005=5"
}
```

### Vectors to Include

- `delta_u32_increment` — basic unsigned delta
- `delta_i64_three_frames` — signed 64-bit delta sequence
- `delta_f32_small_changes` — float delta
- `delta_u32_wrapping` — large jump wraps around (unsigned)
- `delta_zigzag_composition` — `@delta @zigzag` on signed field
- `delta_varint_composition` — `@delta @varint` on unsigned field
- `delta_reset` — encode frames, reset, encode again (bytes match first frame)
- `delta_mixed_message` — message with both delta and non-delta fields

### Rust Compliance Validator

New test in `crates/vexil-codegen-rust/tests/delta_compliance.rs`:
- Creates a stateful encoder
- Encodes each frame in sequence
- Asserts bytes match `expected_bytes` for each frame

### TypeScript Compliance Test

New test in `packages/runtime-ts/tests/delta-compliance.test.ts`:
- Creates `{Name}Encoder` and `{Name}Decoder`
- Encodes each frame, asserts bytes match
- Decodes each frame, asserts values match
- Tests reset behavior

---

## 5. System-Monitor Example Update

### Schema

`@delta` applied to the message — eligible fields get delta encoding
automatically:

```vexil
namespace system.monitor

enum CpuStatus {
    Normal   @0
    Degraded @1
    Critical @2
}

@delta
message SystemSnapshot {
    timestamp_ms    @0 : i64         # delta: ~2-3 bytes vs 8
    hostname        @1 : string      # skipped (string)
    cpu_usage       @2 : u8          # delta: 1 byte (small changes)
    cpu_count       @3 : u8          # delta: 1 byte (always 0 after first)
    per_core_usage  @4 : array<u8>   # skipped (array)
    memory_used_mb  @5 : u32         # delta: ~1-2 bytes vs 4
    memory_total_mb @6 : u32         # delta: 1 byte (always 0 after first)
    cpu_status      @7 : CpuStatus   # skipped (enum)
}
```

### Updated Structure

```
examples/system-monitor/
  schema/telemetry.vexil       # @delta on message
  package.json                 # esbuild + @vexil-lang/runtime
  Cargo.toml
  src/
    main.rs                    # SystemSnapshotEncoder (stateful)
    generated.rs               # vexilc --target rust
  ts/
    generated.ts               # vexilc --target typescript
  static/
    index.html                 # imports bundle.js
    bundle.js                  # esbuild: generated.ts + @vexil-lang/runtime
```

### Build Pipeline

```bash
# Generate code for both targets
cargo run -p vexilc -- codegen schema/telemetry.vexil --target rust > src/generated.rs
cargo run -p vexilc -- codegen schema/telemetry.vexil --target typescript > ts/generated.ts

# Bundle TS for browser
npx esbuild ts/generated.ts --bundle --format=esm --outfile=static/bundle.js

# Run
cargo run --release
```

Wrapped in `package.json` scripts:
```json
{
  "private": true,
  "scripts": {
    "codegen": "cargo run -p vexilc -- codegen schema/telemetry.vexil --target typescript > ts/generated.ts",
    "bundle": "esbuild ts/generated.ts --bundle --format=esm --outfile=static/bundle.js",
    "build": "npm run codegen && npm run bundle"
  },
  "devDependencies": {
    "esbuild": "^0.25.0",
    "@vexil-lang/runtime": "file:../../packages/runtime-ts"
  }
}
```

### Rust Side

Switches from stateless `pack()` to stateful `SystemSnapshotEncoder`:

```rust
let mut encoder = SystemSnapshotEncoder::new();
loop {
    let snapshot = /* collect metrics */;
    let mut w = BitWriter::new();
    encoder.pack(&snapshot, &mut w)?;
    socket.send(Message::Binary(w.finish().into())).await?;
}
```

### Browser Side

`index.html` imports the real generated decoder from `bundle.js`:

```html
<script type="module">
import { SystemSnapshotDecoder } from './bundle.js';
import { BitReader } from './bundle.js';

const decoder = new SystemSnapshotDecoder();
ws.onmessage = (e) => {
  const r = new BitReader(new Uint8Array(e.data));
  const snapshot = decoder.decode(r);
  render(snapshot);
};
</script>
```

### Wire Size

- First frame: ~42 bytes (all deltas from zero = same as absolute values)
- Steady-state frames: ~25-30 bytes (timestamp, memory, cpu_count, memory_total all delta-compressed)
- The HTML footer displays current wire size per frame

---

## 6. Decision Log

### `@delta` on message: syntactic sugar vs. frame-level delta

**Chosen:** Syntactic sugar — desugars to per-field `@delta` on eligible fields.

**Rejected:** Frame-level delta with presence bitmask (skip unchanged fields).

**Rationale:** No wire format change, no RFC needed, ships fast. Frame-level
delta is a future enhancement requiring a wire format change and 14-day RFC
comment period per GOVERNANCE.md.

### Browser TS: generated code vs. hand-written decoder

**Chosen:** Generated TypeScript, bundled with esbuild for the browser.

**Rejected:** Hand-written JavaScript decoder inline in HTML.

**Rationale:** Hand-writing decoders defeats the purpose of a schema-driven
codegen system. The generated code IS the product. The example must showcase
the real pipeline: schema → codegen → use.

### Bundling: esbuild vs. tsc vs. Vite

**Chosen:** esbuild — single command, handles TS natively, fast (<50ms).

**Rejected alternatives:**
- **tsc:** Requires separate compilation step, doesn't bundle, needs module
  resolution config for the browser.
- **Vite:** Full dev server, replaces the Rust HTTP server, overkill for
  an example.
