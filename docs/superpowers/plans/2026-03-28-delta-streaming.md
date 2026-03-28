# Delta Streaming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete cross-language delta encoding support — `@delta` on messages, TypeScript delta codegen, compliance vectors, and system-monitor example with generated TS decoder.

**Architecture:** Spec addition (desugaring `@delta` on message to per-field `@delta`), lowering change to propagate, TS codegen delta module mirroring Rust's `delta.rs`, compliance vectors with multi-frame stream format, system-monitor example rebuilt with generated code and esbuild bundling.

**Tech Stack:** Rust (vexil-lang workspace), TypeScript (codegen + runtime), esbuild (example bundling).

**Spec reference:** `docs/superpowers/specs/2026-03-28-delta-streaming-design.md`

---

## File Structure

```
spec/
  vexil-spec.md                                     # MODIFY: add @delta on message to §13.4
crates/
  vexil-lang/
    src/
      lower.rs                                       # MODIFY: propagate @delta from message to fields
      validate.rs                                    # MODIFY: allow @delta on message declarations
  vexil-codegen-ts/
    src/
      delta.rs                                       # NEW: delta encoder/decoder class generation
      lib.rs                                         # MODIFY: add delta module, call emit_delta after emit_message
    tests/
      golden/
        013_annotations.ts                           # MODIFY: updated golden (has @delta fields)
  vexil-codegen-rust/
    tests/
      delta_compliance.rs                            # NEW: multi-frame delta compliance test
compliance/
  vectors/
    delta.json                                       # NEW: multi-frame delta vectors
packages/
  runtime-ts/
    tests/
      delta-compliance.test.ts                       # NEW: TS delta compliance test
corpus/
  valid/
    027_delta_on_message.vexil                       # NEW: corpus test for @delta on message
  MANIFEST.md                                        # MODIFY: add entry
examples/
  system-monitor/
    schema/telemetry.vexil                           # MODIFY: add @delta on message
    package.json                                     # NEW: esbuild + @vexil-lang/runtime deps
    ts/
      generated.ts                                   # NEW: vexilc --target typescript output
    static/
      index.html                                     # MODIFY: import from bundle.js
      bundle.js                                      # NEW: esbuild output
    src/
      main.rs                                        # MODIFY: use SystemSnapshotEncoder
      generated.rs                                   # MODIFY: regenerated with @delta
```

---

## Task 1: Spec — `@delta` on Message Declarations

**Files:**
- Modify: `spec/vexil-spec.md` (§13.4, after existing @delta text)

- [ ] **Step 1: Find the insertion point**

Read: `spec/vexil-spec.md` around the @delta section. Search for "Stream context boundaries" — the new paragraph goes after the existing @delta block.

- [ ] **Step 2: Insert the message-level @delta paragraph**

After the paragraph ending with "...apply to the encoded delta value, not the raw field value.", add:

```markdown

`@delta` may also be applied to a `message` declaration.  This is equivalent
to annotating every eligible field with `@delta`.  Fields whose types are not
valid for `@delta` are silently skipped — no error is produced.  If a field
already carries an explicit `@delta` annotation, the message-level annotation
does not double-wrap it.  The desugaring happens during IR lowering; the wire
format is identical to per-field `@delta` annotations.
```

- [ ] **Step 3: Commit**

```bash
git add spec/vexil-spec.md
git commit -m "spec: add @delta on message declarations (syntactic sugar)"
```

---

## Task 2: Corpus — `@delta` on Message

**Files:**
- Create: `corpus/valid/027_delta_on_message.vexil`
- Modify: `corpus/MANIFEST.md`

- [ ] **Step 1: Write the corpus schema**

Create `corpus/valid/027_delta_on_message.vexil`:

```vexil
namespace test.delta.message

@delta
message Telemetry {
    timestamp @0 : i64
    value     @1 : f32
    label     @2 : string
    count     @3 : u32
}
```

This tests that `@delta` on a message:
- Applies `@delta` to `timestamp` (i64), `value` (f32), `count` (u32)
- Skips `label` (string — ineligible type)

- [ ] **Step 2: Update MANIFEST.md**

Append to the valid corpus table:

```markdown
| 027_delta_on_message.vexil       | §13.4  | `@delta` on message declaration desugars to per-field |
```

- [ ] **Step 3: Run tests**

Run: `cargo test --workspace`
Expected: FAIL — the parser/validator doesn't accept `@delta` on message declarations yet.

- [ ] **Step 4: Commit**

```bash
git add corpus/valid/027_delta_on_message.vexil corpus/MANIFEST.md
git commit -m "corpus: add 027_delta_on_message for @delta on message declarations"
```

---

## Task 3: Lowering — Propagate `@delta` from Message to Fields

**Files:**
- Modify: `crates/vexil-lang/src/lower.rs`
- Modify: `crates/vexil-lang/src/validate.rs`

The `@delta` annotation on a message must be propagated to eligible fields during lowering. The change is in `lower_message()` at `crates/vexil-lang/src/lower.rs` around line 283.

- [ ] **Step 1: Check if message has @delta annotation**

In `lower_message()`, after building the fields list but before returning `MessageDef`, check if the message's annotations contain `@delta`:

```rust
fn lower_message(msg: &crate::ast::MessageDecl, span: Span, ctx: &mut LowerCtx) -> MessageDef {
    let has_message_delta = msg
        .annotations
        .iter()
        .any(|a| a.name.node == "delta");

    let mut fields = Vec::new();
    let mut tombstones = Vec::new();
    for item in &msg.body {
        match item {
            MessageBodyItem::Field(f) => fields.push(lower_field(&f.node, f.span, ctx)),
            MessageBodyItem::Tombstone(t) => tombstones.push(lower_tombstone(&t.node, t.span)),
        }
    }

    // Desugar @delta on message: apply to all eligible fields
    if has_message_delta {
        for field in &mut fields {
            if is_delta_eligible(&field.resolved_type) && !is_already_delta(&field.encoding) {
                field.encoding.encoding = Encoding::Delta(Box::new(field.encoding.encoding.clone()));
            }
        }
    }

    MessageDef {
        name: msg.name.node.clone(),
        span,
        fields,
        tombstones,
        annotations: resolve_annotations(&msg.annotations),
        wire_size: None,
    }
}
```

- [ ] **Step 2: Add helper functions**

Add these helpers in `lower.rs` (near the existing `compute_field_encoding` function):

```rust
use crate::ir::{Encoding, ResolvedType, PrimitiveType};

/// Returns true if the type is eligible for @delta encoding.
fn is_delta_eligible(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(
            PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::F32
                | PrimitiveType::F64
        ) | ResolvedType::SubByte(_)
    )
}

/// Returns true if the encoding is already Delta-wrapped.
fn is_already_delta(enc: &FieldEncoding) -> bool {
    matches!(enc.encoding, Encoding::Delta(_))
}
```

- [ ] **Step 3: Allow @delta on message declarations in validation**

In `crates/vexil-lang/src/validate.rs`, find the annotation validation code that restricts `@delta`. It currently only accepts `@delta` on fields. Add message declarations as a valid target.

Search for the validation that checks where `@delta` can appear. If there's a check like "delta is only valid on fields", extend it to also allow message declarations. If there's no such check (i.e., declaration-level annotations aren't validated for encoding annotations), then this step may not be needed — the lowering will handle it.

Read the validator code to determine the exact change needed.

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace`
Expected: All pass, including the new corpus file `027_delta_on_message.vexil`.

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/lower.rs crates/vexil-lang/src/validate.rs
git commit -m "feat(vexil-lang): desugar @delta on message to per-field @delta"
```

---

## Task 4: TypeScript Delta Codegen

**Files:**
- Create: `crates/vexil-codegen-ts/src/delta.rs`
- Modify: `crates/vexil-codegen-ts/src/lib.rs`

This is the core task. Create a delta module that generates stateful `{Name}Encoder` and `{Name}Decoder` TypeScript classes, following the exact pattern of `crates/vexil-codegen-rust/src/delta.rs`.

- [ ] **Step 1: Read the Rust reference implementation**

Read: `crates/vexil-codegen-rust/src/delta.rs` (complete file, 239 lines)
This is the template. The TS version follows the same structure.

- [ ] **Step 2: Create `crates/vexil-codegen-ts/src/delta.rs`**

The module exports one public function:

```rust
pub fn emit_delta(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry)
```

**Structure:**

```rust
use vexil_lang::ir::{
    Encoding, FieldEncoding, MessageDef, PrimitiveType, ResolvedType, SemanticType, TypeRegistry,
};

use crate::emit::CodeWriter;
use crate::message::{emit_write, emit_read};
use crate::types::ts_type;

/// Returns true if the field uses delta encoding.
fn is_delta(enc: &FieldEncoding) -> bool {
    matches!(enc.encoding, Encoding::Delta(_))
}

/// Returns true if the type is a float (f32 or f64).
fn is_float(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(PrimitiveType::F32 | PrimitiveType::F64)
    )
}

/// Returns true if the type is a bigint in TypeScript (i64, u64).
fn is_bigint(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(PrimitiveType::I64 | PrimitiveType::U64)
    )
}

/// Returns the TypeScript zero literal for a given type.
fn zero_literal(ty: &ResolvedType) -> &'static str {
    if is_float(ty) {
        "0.0"
    } else if is_bigint(ty) {
        "0n"
    } else {
        "0"
    }
}

/// Unwrap the inner encoding from a Delta wrapper.
fn strip_delta(enc: &FieldEncoding) -> FieldEncoding {
    match &enc.encoding {
        Encoding::Delta(inner) => FieldEncoding {
            encoding: *inner.clone(),
            limit: enc.limit,
        },
        _ => enc.clone(),
    }
}

/// Emit stateful {Name}Encoder and {Name}Decoder classes for messages
/// that have @delta fields.
pub fn emit_delta(w: &mut CodeWriter, msg: &MessageDef, registry: &TypeRegistry) {
    // Collect delta fields
    let delta_fields: Vec<_> = msg
        .fields
        .iter()
        .filter(|f| is_delta(&f.encoding))
        .collect();

    if delta_fields.is_empty() {
        return;
    }

    emit_encoder(w, msg, &delta_fields, registry);
    w.blank();
    emit_decoder(w, msg, &delta_fields, registry);
}
```

**Encoder class:**

```rust
fn emit_encoder(
    w: &mut CodeWriter,
    msg: &MessageDef,
    delta_fields: &[&FieldDef],
    registry: &TypeRegistry,
) {
    let name = &msg.name;
    w.open_block(&format!("export class {name}Encoder"));

    // Private prev fields
    for field in delta_fields {
        let ts = ts_type(&field.resolved_type, registry);
        let zero = zero_literal(&field.resolved_type);
        let fname = to_camel_case(&field.name);
        w.line(&format!("private prev{fname}: {ts} = {zero};"));
    }
    w.blank();

    // encode() method
    w.open_block(&format!("encode(v: {name}, w: BitWriter): void"));
    for field in &msg.fields {
        let fname = &field.name;
        if is_delta(&field.encoding) {
            let camel = to_camel_case(fname);
            let inner_enc = strip_delta(&field.encoding);
            if is_bigint(&field.resolved_type) {
                // BigInt subtraction
                w.line(&format!("const delta{camel} = v.{fname} - this.prev{camel};"));
            } else if is_float(&field.resolved_type) {
                // Float subtraction
                w.line(&format!("const delta{camel} = v.{fname} - this.prev{camel};"));
            } else {
                // Integer wrapping subtraction: (a - b) & mask
                let bits = primitive_bits(&field.resolved_type);
                w.line(&format!(
                    "const delta{camel} = (v.{fname} - this.prev{camel}) & 0x{:X};",
                    (1u64 << bits) - 1
                ));
            }
            // Write delta using inner encoding
            emit_write(w, &format!("delta{camel}"), &field.resolved_type, &inner_enc, registry, "w");
            // Update prev
            w.line(&format!("this.prev{camel} = v.{fname};"));
        } else {
            // Non-delta field: write normally
            emit_write(w, &format!("v.{fname}"), &field.resolved_type, &field.encoding, registry, "w");
        }
    }
    w.line("w.flushToByteBoundary();");
    w.close_block();
    w.blank();

    // reset() method
    w.open_block("reset(): void");
    for field in delta_fields {
        let camel = to_camel_case(&field.name);
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("this.prev{camel} = {zero};"));
    }
    w.close_block();

    w.close_block(); // class
}
```

**Decoder class** mirrors the encoder but reads delta and reconstructs:

```rust
fn emit_decoder(
    w: &mut CodeWriter,
    msg: &MessageDef,
    delta_fields: &[&FieldDef],
    registry: &TypeRegistry,
) {
    let name = &msg.name;
    w.open_block(&format!("export class {name}Decoder"));

    // Private prev fields (same as encoder)
    for field in delta_fields {
        let ts = ts_type(&field.resolved_type, registry);
        let zero = zero_literal(&field.resolved_type);
        let fname = to_camel_case(&field.name);
        w.line(&format!("private prev{fname}: {ts} = {zero};"));
    }
    w.blank();

    // decode() method
    w.open_block(&format!("decode(r: BitReader): {name}"));
    for field in &msg.fields {
        let fname = &field.name;
        if is_delta(&field.encoding) {
            let camel = to_camel_case(fname);
            let inner_enc = strip_delta(&field.encoding);
            // Read delta
            emit_read(w, &format!("delta{camel}"), &field.resolved_type, &inner_enc, registry, "r");
            // Reconstruct value
            if is_bigint(&field.resolved_type) {
                w.line(&format!("const {fname} = this.prev{camel} + delta{camel};"));
            } else if is_float(&field.resolved_type) {
                w.line(&format!("const {fname} = this.prev{camel} + delta{camel};"));
            } else {
                let bits = primitive_bits(&field.resolved_type);
                w.line(&format!(
                    "const {fname} = (this.prev{camel} + delta{camel}) & 0x{:X};",
                    (1u64 << bits) - 1
                ));
            }
            // Update prev
            w.line(&format!("this.prev{camel} = {fname};"));
        } else {
            // Non-delta: read normally
            emit_read(w, fname, &field.resolved_type, &field.encoding, registry, "r");
        }
    }
    w.line("r.flushToByteBoundary();");
    // Return object
    let field_names: Vec<&str> = msg.fields.iter().map(|f| f.name.as_str()).collect();
    w.line(&format!("return {{ {} }};", field_names.join(", ")));
    w.close_block();
    w.blank();

    // reset() method
    w.open_block("reset(): void");
    for field in delta_fields {
        let camel = to_camel_case(&field.name);
        let zero = zero_literal(&field.resolved_type);
        w.line(&format!("this.prev{camel} = {zero};"));
    }
    w.close_block();

    w.close_block(); // class
}
```

**Helpers:**

```rust
fn to_camel_case(snake: &str) -> String {
    snake.split('_')
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                // Keep first segment capitalized for class property naming
                let mut c = part.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().to_string() + c.as_str(),
                }
            } else {
                let mut c = part.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().to_string() + c.as_str(),
                }
            }
        })
        .collect()
}

fn primitive_bits(ty: &ResolvedType) -> u32 {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::U8 | PrimitiveType::I8 => 8,
            PrimitiveType::U16 | PrimitiveType::I16 => 16,
            PrimitiveType::U32 | PrimitiveType::I32 | PrimitiveType::F32 => 32,
            PrimitiveType::U64 | PrimitiveType::I64 | PrimitiveType::F64 => 64,
            _ => 0,
        },
        ResolvedType::SubByte(s) => s.bits as u32,
        _ => 0,
    }
}
```

**NOTE:** The `emit_write` and `emit_read` functions from `message.rs` may need to be made `pub(crate)` if they aren't already, so `delta.rs` can call them. Check their visibility and adjust.

- [ ] **Step 3: Wire delta into lib.rs**

In `crates/vexil-codegen-ts/src/lib.rs`:

Add module declaration:
```rust
pub mod delta;
```

In the `TypeDef::Message` match arm (around line 90), add delta call after emit_message:
```rust
TypeDef::Message(msg) => {
    message::emit_message(&mut w, msg, &compiled.registry);
    delta::emit_delta(&mut w, msg, &compiled.registry);
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p vexil-codegen-ts`
Expected: PASS. Fix any type errors, visibility issues, or missing imports.

- [ ] **Step 5: Regenerate golden files**

Run: `UPDATE_GOLDEN=1 cargo test -p vexil-codegen-ts`

The golden file for `013_annotations.ts` should now include `EncodedEncoder` and `EncodedDecoder` classes (the `Encoded` message in that corpus file has `@delta` fields).

- [ ] **Step 6: Review generated golden file**

Read: `crates/vexil-codegen-ts/tests/golden/013_annotations.ts` (or whichever golden file has delta fields)
Verify the generated encoder/decoder classes look correct.

- [ ] **Step 7: Run all tests**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 8: Commit**

```bash
git add crates/vexil-codegen-ts/
git commit -m "feat(vexil-codegen-ts): generate delta encoder/decoder classes"
```

---

## Task 5: Delta Compliance Vectors

**Files:**
- Create: `compliance/vectors/delta.json`

- [ ] **Step 1: Write the delta compliance vectors**

Before writing vectors, verify the expected bytes by running quick Rust tests. The key insight: delta vectors need the **stateful encoder** — you encode multiple frames and the expected bytes change.

Create `compliance/vectors/delta.json`:

```json
[
  {
    "name": "delta_u32_two_frames",
    "schema": "namespace test.delta\nmessage M {\n  @delta\n  v @0 : u32\n}",
    "type": "M",
    "frames": [
      { "value": { "v": 100 }, "expected_bytes": "64000000" },
      { "value": { "v": 110 }, "expected_bytes": "0a000000" }
    ],
    "notes": "Delta 0->100=100 (0x64), 100->110=10 (0x0A)"
  },
  {
    "name": "delta_i64_three_frames",
    "schema": "namespace test.delta\nmessage M {\n  @delta\n  v @0 : i64\n}",
    "type": "M",
    "frames": [
      { "value": { "v": 1000 }, "expected_bytes": "e803000000000000" },
      { "value": { "v": 2000 }, "expected_bytes": "e803000000000000" },
      { "value": { "v": 2005 }, "expected_bytes": "0500000000000000" }
    ],
    "notes": "Delta 0->1000=1000, 1000->2000=1000, 2000->2005=5"
  },
  {
    "name": "delta_mixed_message",
    "schema": "namespace test.delta\nmessage M {\n  @delta\n  ts @0 : i64\n  label @1 : string\n  @delta\n  count @2 : u32\n}",
    "type": "M",
    "frames": [
      { "value": { "ts": 1000, "label": "hello", "count": 50 }, "expected_bytes": "e8030000000000000568656c6c6f32000000" },
      { "value": { "ts": 2000, "label": "hello", "count": 55 }, "expected_bytes": "e8030000000000000568656c6c6f05000000" }
    ],
    "notes": "ts delta: 1000 both frames. label: always full. count delta: 50 then 5."
  },
  {
    "name": "delta_reset",
    "schema": "namespace test.delta\nmessage M {\n  @delta\n  v @0 : u32\n}",
    "type": "M",
    "frames": [
      { "value": { "v": 100 }, "expected_bytes": "64000000" },
      { "value": { "v": 150 }, "expected_bytes": "32000000" },
      { "reset": true },
      { "value": { "v": 100 }, "expected_bytes": "64000000" }
    ],
    "notes": "After reset, encoder state returns to zero. Frame 3 encodes 0->100=100 again."
  }
]
```

**IMPORTANT:** The expected bytes above are estimates. After creating the file, verify them in Task 6 by running the Rust compliance test. If they don't match, update the vector file with the correct bytes from the Rust reference implementation.

- [ ] **Step 2: Commit**

```bash
git add compliance/vectors/delta.json
git commit -m "feat: add delta encoding compliance vectors"
```

---

## Task 6: Rust Delta Compliance Validator

**Files:**
- Create: `crates/vexil-codegen-rust/tests/delta_compliance.rs`

- [ ] **Step 1: Write the Rust delta compliance test**

This test validates that the Rust `BitWriter` produces the expected bytes for delta-encoded fields. Since we can't easily use the generated `{Name}Encoder` in an integration test (it requires compiling generated code), we test at the BitWriter level — manually encoding the delta sequence.

```rust
//! Delta encoding compliance tests.
//!
//! Validates stateful delta encoding produces expected byte sequences.

use vexil_runtime::BitWriter;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn delta_u32_two_frames() {
    // Frame 1: delta from 0 to 100 = 100
    let mut w = BitWriter::new();
    w.write_u32(100_u32.wrapping_sub(0)); // delta = 100
    w.flush_to_byte_boundary();
    let frame1 = hex(&w.finish());

    // Frame 2: delta from 100 to 110 = 10
    let mut w = BitWriter::new();
    w.write_u32(110_u32.wrapping_sub(100)); // delta = 10
    w.flush_to_byte_boundary();
    let frame2 = hex(&w.finish());

    assert_eq!(frame1, "64000000");
    assert_eq!(frame2, "0a000000");
}

#[test]
fn delta_i64_three_frames() {
    let deltas: Vec<i64> = vec![1000 - 0, 2000 - 1000, 2005 - 2000]; // 1000, 1000, 5

    for (i, delta) in deltas.iter().enumerate() {
        let mut w = BitWriter::new();
        w.write_i64(*delta);
        w.flush_to_byte_boundary();
        let bytes = hex(&w.finish());
        match i {
            0 => assert_eq!(bytes, "e803000000000000"),
            1 => assert_eq!(bytes, "e803000000000000"),
            2 => assert_eq!(bytes, "0500000000000000"),
            _ => unreachable!(),
        }
    }
}

#[test]
fn delta_mixed_message() {
    // Frame 1: ts delta=1000, label="hello", count delta=50
    let mut w = BitWriter::new();
    w.write_i64(1000); // ts delta from 0
    w.write_string("hello");
    w.write_u32(50); // count delta from 0
    w.flush_to_byte_boundary();
    let frame1 = hex(&w.finish());

    // Frame 2: ts delta=1000, label="hello", count delta=5
    let mut w = BitWriter::new();
    w.write_i64(1000); // 2000-1000
    w.write_string("hello");
    w.write_u32(5); // 55-50
    w.flush_to_byte_boundary();
    let frame2 = hex(&w.finish());

    assert_eq!(frame1, "e8030000000000000568656c6c6f32000000");
    assert_eq!(frame2, "e8030000000000000568656c6c6f05000000");
}

#[test]
fn delta_reset() {
    // Frame 1: delta 0->100 = 100
    let mut w = BitWriter::new();
    w.write_u32(100);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "64000000");

    // Frame 2: delta 100->150 = 50
    let mut w = BitWriter::new();
    w.write_u32(50);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "32000000");

    // After reset: delta 0->100 = 100 again
    let mut w = BitWriter::new();
    w.write_u32(100);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "64000000");
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p vexil-codegen-rust delta_compliance`
Expected: All pass. If any expected bytes are wrong, update `compliance/vectors/delta.json` with the correct values.

- [ ] **Step 3: Update compliance vectors if needed**

If any expected bytes didn't match, fix `compliance/vectors/delta.json` now.

- [ ] **Step 4: Commit**

```bash
git add crates/vexil-codegen-rust/tests/delta_compliance.rs compliance/vectors/delta.json
git commit -m "test: delta encoding compliance validator (Rust)"
```

---

## Task 7: TypeScript Delta Compliance Test

**Files:**
- Create: `packages/runtime-ts/tests/delta-compliance.test.ts`

- [ ] **Step 1: Write the TS delta compliance test**

```typescript
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { BitWriter, BitReader } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const vectorsDir = join(__dirname, '../../../compliance/vectors');

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

interface DeltaFrame {
  value?: Record<string, unknown>;
  expected_bytes?: string;
  reset?: boolean;
}

interface DeltaVector {
  name: string;
  schema: string;
  type: string;
  frames: DeltaFrame[];
  notes?: string;
}

describe('delta compliance', () => {
  const vectors: DeltaVector[] = JSON.parse(
    readFileSync(join(vectorsDir, 'delta.json'), 'utf-8')
  );

  it('delta_u32_two_frames: encode', () => {
    const v = vectors.find(v => v.name === 'delta_u32_two_frames')!;
    let prev = 0;
    for (const frame of v.frames) {
      if (frame.reset) { prev = 0; continue; }
      const val = frame.value!.v as number;
      const delta = (val - prev) >>> 0; // unsigned wrapping
      const w = new BitWriter();
      w.writeU32(delta);
      w.flushToByteBoundary();
      expect(toHex(w.finish())).toBe(frame.expected_bytes);
      prev = val;
    }
  });

  it('delta_i64_three_frames: encode', () => {
    const v = vectors.find(v => v.name === 'delta_i64_three_frames')!;
    let prev = 0n;
    for (const frame of v.frames) {
      if (frame.reset) { prev = 0n; continue; }
      const val = BigInt(frame.value!.v as number);
      const delta = val - prev;
      const w = new BitWriter();
      w.writeI64(delta);
      w.flushToByteBoundary();
      expect(toHex(w.finish())).toBe(frame.expected_bytes);
      prev = val;
    }
  });

  it('delta_mixed_message: encode', () => {
    const v = vectors.find(v => v.name === 'delta_mixed_message')!;
    let prevTs = 0n;
    let prevCount = 0;
    for (const frame of v.frames) {
      if (frame.reset) { prevTs = 0n; prevCount = 0; continue; }
      const ts = BigInt(frame.value!.ts as number);
      const label = frame.value!.label as string;
      const count = frame.value!.count as number;
      const w = new BitWriter();
      w.writeI64(ts - prevTs);
      w.writeString(label);
      w.writeU32((count - prevCount) >>> 0);
      w.flushToByteBoundary();
      expect(toHex(w.finish())).toBe(frame.expected_bytes);
      prevTs = ts;
      prevCount = count;
    }
  });

  it('delta_reset: encode', () => {
    const v = vectors.find(v => v.name === 'delta_reset')!;
    let prev = 0;
    for (const frame of v.frames) {
      if (frame.reset) { prev = 0; continue; }
      const val = frame.value!.v as number;
      const delta = (val - prev) >>> 0;
      const w = new BitWriter();
      w.writeU32(delta);
      w.flushToByteBoundary();
      expect(toHex(w.finish())).toBe(frame.expected_bytes);
      prev = val;
    }
  });
});
```

- [ ] **Step 2: Run tests**

Run: `cd packages/runtime-ts && npx vitest run`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add packages/runtime-ts/tests/delta-compliance.test.ts
git commit -m "test(@vexil-lang/runtime): delta encoding compliance tests"
```

---

## Task 8: System-Monitor Example — Rebuild with Generated Code

**Files:**
- Modify: `examples/system-monitor/schema/telemetry.vexil`
- Create: `examples/system-monitor/package.json`
- Modify: `examples/system-monitor/src/main.rs`
- Modify: `examples/system-monitor/static/index.html`
- Create: `examples/system-monitor/ts/` (generated code)
- Create: `examples/system-monitor/static/bundle.js` (esbuild output)
- Regenerate: `examples/system-monitor/src/generated.rs`

This is the largest task. It rebuilds the system-monitor to use `@delta` on the message, generated code on both sides, and esbuild for browser bundling.

- [ ] **Step 1: Update the schema with @delta**

Update `examples/system-monitor/schema/telemetry.vexil`:

```vexil
namespace system.monitor

enum CpuStatus {
    Normal   @0
    Degraded @1
    Critical @2
}

@delta
message SystemSnapshot {
    timestamp_ms    @0 : i64
    hostname        @1 : string
    cpu_usage       @2 : u8
    cpu_count       @3 : u8
    per_core_usage  @4 : array<u8>
    memory_used_mb  @5 : u32
    memory_total_mb @6 : u32
    cpu_status      @7 : CpuStatus
}
```

- [ ] **Step 2: Regenerate Rust code**

Run: `cargo run -p vexilc -- codegen <absolute-path>/examples/system-monitor/schema/telemetry.vexil --target rust > examples/system-monitor/src/generated.rs`

Verify the output includes `SystemSnapshotEncoder` and `SystemSnapshotDecoder` structs.

- [ ] **Step 3: Regenerate TypeScript code**

Run: `cargo run -p vexilc -- codegen <absolute-path>/examples/system-monitor/schema/telemetry.vexil --target typescript > examples/system-monitor/ts/generated.ts`

Verify the output includes `SystemSnapshotEncoder` and `SystemSnapshotDecoder` classes.

- [ ] **Step 4: Create package.json for esbuild**

Create `examples/system-monitor/package.json`:

```json
{
  "private": true,
  "scripts": {
    "codegen:ts": "cargo run -p vexilc -- codegen schema/telemetry.vexil --target typescript > ts/generated.ts",
    "bundle": "esbuild ts/generated.ts --bundle --format=esm --outfile=static/bundle.js",
    "build": "npm run codegen:ts && npm run bundle"
  },
  "devDependencies": {
    "esbuild": "^0.25.0",
    "@vexil-lang/runtime": "file:../../packages/runtime-ts"
  }
}
```

- [ ] **Step 5: Install and bundle**

Run:
```bash
cd examples/system-monitor
npm install
npm run bundle
```

This creates `static/bundle.js` containing the generated decoder + `@vexil-lang/runtime`.

- [ ] **Step 6: Update main.rs to use stateful encoder**

Replace the encode section in `examples/system-monitor/src/main.rs`. The key change: instead of `snapshot.pack(&mut w)`, use `encoder.pack(&snapshot, &mut w)` where `encoder` is a `SystemSnapshotEncoder` that persists across frames.

Read the generated `src/generated.rs` to understand the exact `SystemSnapshotEncoder` API — it should have `new()`, `pack()`, and `reset()`.

The WebSocket handler becomes:

```rust
async fn handle_ws(mut socket: WebSocket) {
    let mut sys = System::new_all();
    sys.refresh_all();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut encoder = SystemSnapshotEncoder::new();
    let mut tick = interval(Duration::from_secs(1));

    loop {
        tick.tick().await;
        sys.refresh_all();

        let snapshot = SystemSnapshot { /* same as before */ };

        let mut w = BitWriter::new();
        if encoder.pack(&snapshot, &mut w).is_err() {
            continue;
        }
        let bytes = w.finish();

        if socket.send(Message::Binary(bytes.into())).await.is_err() {
            break;
        }
    }
}
```

- [ ] **Step 7: Update index.html to use generated bundle**

Replace the inline `<script>` in `examples/system-monitor/static/index.html` with:

```html
<script type="module">
import { SystemSnapshotDecoder } from './bundle.js';
import { BitReader } from './bundle.js';

const decoder = new SystemSnapshotDecoder();

function connect() {
  const el = document.getElementById('connection');
  const ws = new WebSocket(`ws://${location.host}/ws`);
  ws.binaryType = 'arraybuffer';

  ws.onopen = () => { el.textContent = 'live'; el.className = 'ok'; };
  ws.onclose = () => {
    el.textContent = 'reconnecting...'; el.className = 'err';
    decoder.reset(); // Reset delta state on reconnect
    setTimeout(connect, 2000);
  };
  ws.onerror = () => ws.close();

  ws.onmessage = (e) => {
    const bytes = new Uint8Array(e.data);
    document.getElementById('wire-size').textContent = bytes.length;
    try {
      const r = new BitReader(bytes);
      const s = decoder.decode(r);
      render(s);
    } catch(err) { console.error('decode error', err); }
  };
}

function render(s) {
  // Same render logic as before, but field access may need adjustment
  // based on the generated interface field names
  document.getElementById('cpu-val').textContent = s.cpu_usage;
  document.getElementById('cpu-bar').style.width = s.cpu_usage + '%';
  // ... etc (keep the existing render function but update field names if needed)
}

connect();
</script>
```

**IMPORTANT:** The Rust server needs to serve `bundle.js` as a static file, not just `index.html`. Update main.rs to:
1. Serve `index.html` at `/` (already done via `include_str!`)
2. Serve `bundle.js` at `/bundle.js`

Add to main.rs:
```rust
static BUNDLE_JS: &str = include_str!("../static/bundle.js");

// In router:
.route("/bundle.js", get(bundle_js))

// Handler:
async fn bundle_js() -> ([(axum::http::header::HeaderName, &'static str); 1], &'static str) {
    ([(axum::http::header::CONTENT_TYPE, "application/javascript")], BUNDLE_JS)
}
```

- [ ] **Step 8: Build and test**

```bash
cd examples/system-monitor
cargo run --release
```

Open http://127.0.0.1:3000 — verify:
- Dashboard loads
- Data updates every second
- Wire size is smaller than before (~25-30 bytes after first frame vs ~42)
- Reconnect resets the decoder

- [ ] **Step 9: Update README**

Update `examples/system-monitor/README.md` to document:
- The `@delta` annotation
- The build pipeline (`npm run build` then `cargo run --release`)
- Wire size comparison (with vs without delta)

- [ ] **Step 10: Add node_modules and dist to gitignore**

Ensure `examples/system-monitor/node_modules/` is in `.gitignore`.

- [ ] **Step 11: Commit**

```bash
git add examples/system-monitor/
git commit -m "feat(examples): rebuild system-monitor with @delta, generated TS decoder, esbuild bundle"
```

---

## Task 9: Final Integration Verification

**Files:** No new files — verification only.

- [ ] **Step 1: Run Rust test suite**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 2: Run TypeScript tests**

Run: `cd packages/runtime-ts && npx vitest run`
Expected: All pass (including new delta compliance tests).

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: Clean (except pre-existing vexil-store warnings).

- [ ] **Step 4: Run fmt**

Run: `cargo fmt --all -- --check`
Expected: Clean.

- [ ] **Step 5: Commit final state**

```bash
git add -A
git commit -m "chore: final integration verification — delta streaming complete"
```
