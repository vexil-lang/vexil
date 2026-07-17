# Spec Freeze Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this task.

**Goal:** Finalize and document the Vexil 1.0 binary format specification.

**Architecture:** Document wire format, encoding rules, and compliance vectors. Lock the spec.

---

## Current State

- SPEC.md may not exist or be incomplete
- Binary format is implemented but not fully documented
- No formal compliance vector specification

## Target State

- Complete SPEC.md with binary format details
- Compliance vectors defined for all types
- Spec marked as 1.0 FROZEN

---

## Task 1: Audit Current Spec State

**Objective:** Find and assess existing spec documentation.

**Files:**
- Search: `docs/spec/`, `SPEC.md`, `docs/book/src/`

**Step 1: Find existing spec files**

```bash
find . -name "*.md" | xargs grep -l "binary format\|wire format\|encoding" 2>/dev/null
```

**Step 2: Check if SPEC.md exists**

```bash
cat docs/spec/SPEC.md 2>/dev/null || echo "No SPEC.md found"
```

**Step 3: Report findings**

Document what exists and what's missing.

---

## Task 2: Write Binary Format Specification

**Objective:** Document the wire format completely.

**Files:**
- Create/Modify: `docs/spec/SPEC.md`

**Step 1: Document scalar encodings**

```markdown
# Vexil 1.0 Binary Format Specification

## Scalar Types

### Integer Encoding

- u8, i8: single byte, little-endian
- u16, i16: 2 bytes, little-endian
- u32, i32: 4 bytes, little-endian
- u64, i64: 8 bytes, little-endian

### Float Encoding

- f32: IEEE 754 binary32, little-endian
- f64: IEEE 754 binary64, little-endian

### Boolean Encoding

- bool: single bit (1 = true, 0 = false)
- Bit-packed: consecutive bools share bytes

### Fixed-Point Encoding

- fixed32: 32-bit signed integer (raw wire value)
- fixed64: 64-bit signed integer (raw wire value)
- Scale factor is schema-defined (not on wire)
```

**Step 2: Document compound types**

```markdown
## Compound Types

### Array Encoding

- Length-prefixed when variable
- Fixed-size arrays: elements concatenated

### Map Encoding

- Length-prefixed (u32 LE) entry count
- Each entry: key, then value

### Message Encoding

- Fields in ordinal order
- Bit-packed boolean fields share prefix bytes
```

**Step 3: Document unions**

```markdown
## Union Encoding

- Tag byte/variant first (ordinal)
- Then variant payload
```

**Step 4: Document delta/diff encoding**

```markdown
## Delta Encoding

- Presence bitmap for each field
- Present fields encoded, absent fields skipped
```

---

## Task 3: Define Compliance Vectors

**Objective:** Create formal test vectors for cross-implementation testing.

**Files:**
- Modify: `compliance/vectors/README.md`
- Create: `compliance/vectors/spec.json`

**Step 1: Document vector format**

```markdown
# Compliance Vectors

Each vector file is JSON:

```json
{
  "name": "simple_message",
  "schema": "...vexil source...",
  "test_cases": [
    {
      "name": "all_zeros",
      "values": {"field1": 0, "field2": false},
      "expected_bytes": "hex string"
    }
  ]
}
```
```

**Step 2: Create sample vectors**

Create JSON vectors for basic types:
- 001_u8.vexil + vector
- 002_u16.vexil + vector
- etc.

---

## Task 4: Mark Spec as Frozen

**Objective:** Add 1.0 frozen banner to spec.

```markdown
---
status: FROZEN
version: 1.0.0
frozen_date: 2026-04-XX
---

> **SPEC FROZEN**: This specification is locked for Vexil 1.0.
> Changes require major version bump.
```

---

## Task 5: Commit and Tag

```bash
git add docs/spec/
git add compliance/vectors/
git commit -m "spec: freeze 1.0 binary format specification"
git tag -a v1.0.0-spec -m "Vexil 1.0 specification frozen"
```

---

**Summary:** Document binary format fully, create compliance vectors, mark spec as frozen.
