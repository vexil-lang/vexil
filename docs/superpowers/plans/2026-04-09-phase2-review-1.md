# Phase 2 Primitives Implementation Plan — Review #1

**Reviewer**: Automated Review
**Date**: 2026-04-09
**Plan**: `2026-04-09-phase2-primitives.md
**Against**: Design decisions (`2026-04-09-vexil-1.0-roadmap-design.md`) and spec (`vexil-spec.md`)

---

## Verdict: REQUEST_CHANGES

The plan is well-structured and covers the pipeline stages thoroughly, but has several spec compliance gaps and one critical conflict with the approved design decisions.

---

## Critical Issues (Must Fix)

### C1: Type Alias Chains Violate Design Decision

**Location**: Plan line 464-465, edge case line 939

**Plan says**: "Alias chains are allowed but validated (A = B, B = C, C = u32)"

**Design doc says** (line 97): "Aliases cannot reference other aliases (no `type A = B` where B is an alias). Resolve to terminal type."

**Conflict**: The plan explicitly allows alias chains, but the approved design decision prohibits them. Aliases must resolve directly to a terminal (non-alias) type. `type A = B` where B is `type B = u32` should be rejected at parse/validation time.

**Fix**: Update validate.rs rules to reject alias-to-alias references. The `lower_alias` function should check that the target is not itself an alias. Remove the chain example from the test file and add a new invalid test case for alias-to-alias.

### C2: Range Syntax Is Missing Exclusive Variant

**Location**: Plan lines 548, 585-586

**Plan says**: Only `value in low..high` (described as "inclusive-exclusive")

**Design doc says** (line 122): Two forms:
- `value in low..high` — inclusive
- `value in low..<high` — exclusive

**Conflict**: The design doc specifies both inclusive and exclusive range syntax. The plan has only one form. The naming "inclusive-exclusive" is also ambiguous — is the upper bound included or excluded?

**Fix**: Add the exclusive range variant (`..<`) to the grammar, AST, and codegen. Define whether `low..high` means inclusive-inclusive (design doc) or inclusive-exclusive (plan). Follow the design doc: `low..high` = inclusive on both ends, `low..<high` = exclusive upper bound.

---

## Important Issues (Should Fix)

### I1: Const Type Restriction Too Narrow

**Location**: Plan lines 287-288

**Plan says**: "Type must be integral (u8-u64, i8-i64, or sub-byte)"

**Design doc says** (line 77): "Type must be a primitive or fixed-point type"

**Gap**: The design doc allows fixed-point types as const types (example on line 69: `const MAX_HEALTH : fixed64 = 100.0`). The plan restricts to integral types only. If fixed32/fixed64 are implemented in this same phase, allowing them as const types is a natural use case.

**Fix**: Broaden the const type restriction to include fixed32/fixed64. This requires adding fixed-point literal support to ConstExpr (currently only Int, UInt, Hex).

### I2: Where Clause Operand Missing Float Literal Support

**Location**: Plan lines 588-593 (operand grammar)

**Plan says**: Operands are dec-int, hex-int, len(), field-ref, const-ref

**Gap**: The valid test case (line 847) shows `value >= 0.0 && value <= 1.0` for a fixed32 field, but the operand grammar has no float literal production. Where do `0.0` and `1.0` come from?

**Fix**: Add float literal support to the operand grammar and AST. Add a `Float(f64)` variant to `Operand`.

### I3: Where Clauses On Decode-Time vs Validate-Time

**Location**: Plan lines 942-945 (edge cases), lines 815-835 (codegen)

**Plan says** (line 943): "No — they generate `validate()` methods for explicit calls"

**Design doc says** (line 129): "Constraints generate validation code in codegen backends" and line 131: "A schema with constraints compiles to code that validates on encode/decode."

**Gap**: The design doc says constraints validate on encode/decode. The plan says they generate explicit `validate()` methods. This is a semantic difference: auto-validate on encode/decode means invalid data is never on the wire, while explicit validation is opt-in.

**Fix**: Clarify in the plan whether where clauses auto-validate on pack/unpack or require explicit calls. Follow the design doc intent (auto-validate) unless there's a deliberate reason to deviate.

### I4: `len()` On Maps Is an Undocumented Extension

**Location**: Plan line 549

**Plan says**: "Built-in: `len(field)` for arrays/strings/bytes" (and the test case line 848 implies arrays)

**Design doc says** (line 124): "Built-in functions: `len(value)` for string/bytes/array length"

**Minor gap**: The plan's validation rules (line 749) say "`len()` only on arrays, maps, strings, bytes" — maps are included here but not in the design doc. This is a reasonable extension but should be confirmed and documented.

---

## Minor Issues (Optional)

### M1: Token Names May Conflict With Existing Tokens

**Location**: Plan line 612-613

The plan proposes `PipePipe`, `AmpersandAmpersand`, `Bang` token names. The existing lexer may already have `LAngle`/`RAngle` that could overlap with `<`/`>`. Verify no naming conflicts in `token.rs`.

### M2: MANIFEST.md Not Updated

The plan adds 4 valid and 6 invalid corpus files. The `corpus/MANIFEST.md` must be updated to include these new entries with proper spec section references. CLAUDE.md explicitly requires this.

### M3: Compliance Vector Details

**Location**: Plan lines 953-957

The compliance vector `fixed32_basic` says "fixed32 value 1.5 (0x00018000)". Verify the byte representation:
- Q16.16 for 1.5 = (1 << 16) + (0.5 << 16) = 0x00018000. In little-endian wire order: `00 80 01 00`. The plan shows the native integer value, not the wire bytes. Compliance vectors should specify the actual wire bytes.

### M4: Missing Negative Test for `@zigzag` on Fixed Types

The plan removes the negative test for `@varint` on fixed types (correctly, since it's valid). But there should be a negative test for `@zigzag` on fixed types, which the design doc explicitly says is NOT valid (line 62).

### M5: Const Arithmetic Division Semantics

The plan uses integer division (`/`) for const expressions. For the design doc example `TICK_DURATION = 1.0 / TICKS_PER_SEC`, this only works if the const system supports fixed-point types (see I1). Without fixed-point const support, integer division truncates. Document the division semantics clearly.

### M6: Codegen Backends Missing From Where Clause Section

**Location**: Plan line 813

The plan says "Both Rust and TypeScript backends need to generate validation methods" but only shows a Rust example. The TS codegen is mentioned in the appendix (line 979, +100 lines) but has no detail on how where clauses map to TypeScript. This should be expanded for completeness.

---

## Spec Compliance Checklist

| Criterion | Status | Notes |
|-----------|--------|-------|
| Plan matches approved design decisions | PARTIAL | C1 (alias chains), I1 (const types), I3 (validate semantics) deviate |
| All 4 features fully covered | YES | fixed32/64, const, type alias, where clauses all addressed |
| Wire encodings match design doc | YES | Two's complement raw bits, optional @varint via LEB128 |
| @varint on fixed-point correctly specified | YES | Design decision #1 allows it; plan correctly includes it |
| Const cross-references via simple arithmetic | YES | Design decision #4; plan includes +,-,*,/ with topo sort |
| set silent dedup not conflicting | N/A | Not in this phase; no conflict detected |
| All pipeline stages covered | YES | Lexer through codegen for each feature |
| Edge cases identified | MOSTLY | Missing exclusive range edge case |
| Error classes defined | YES | Comprehensive set per feature |
| Test cases specified | MOSTLY | Missing alias-to-alias invalid test, @zigzag on fixed test |
| Consistent with codebase patterns | YES | Follows existing naming and structure |
| Logical implementation order | YES | Fixed → Alias → Const → Where is sound |

---

## Summary of Required Changes

1. **Fix C1**: Reject alias-to-alias references (no chains). Update validation and tests.
2. **Fix C2**: Add exclusive range syntax (`..<`) or document the deviation from design doc with rationale.
3. **Fix I1**: Allow fixed-point types in const declarations, or document why not.
4. **Fix I2**: Add float literal support to where clause operands.
5. **Fix I3**: Clarify encode/decode auto-validate vs explicit validate() semantics.

After these fixes, the plan should be ready for approval.

---

## What Was Reviewed

- `docs/superpowers/plans/2026-04-09-phase2-primitives.md` (985 lines, complete)
- `docs/superpowers/specs/2026-04-09-vexil-1.0-roadmap-design.md` (397 lines, complete)
- `spec/vexil-spec.md` sections 1-7 (language spec, type system, declarations, annotations, imports, canonical form)
- `corpus/MANIFEST.md` (corpus index)
- `CLAUDE.md` (repo conventions and build instructions)
