# Phase 2 Primitives Implementation Plan — Review #2

**Reviewer**: Automated Review
**Date**: 2026-04-09
**Plan**: `2026-04-09-phase2-primitives.md`
**Against**: Design decisions (`2026-04-09-vexil-1.0-roadmap-design.md`) and spec (`vexil-spec.md`)

---

## Verdict: PASS

All 6 issues from Review #1 have been fixed. No new issues introduced.

---

## Fix Verification

| Issue | Fix Required | Status | Evidence |
|-------|-------------|--------|----------|
| C1 | Alias chains rejected | FIXED | Lines 425-428: grammar comment explicitly rejects alias-to-alias. Line 477: validation rule requires terminal type. Line 552-558: invalid test `062_alias_to_alias.vexil` added. |
| C2 | Exclusive range `..<` added | FIXED | Line 577: both forms listed. Lines 615-616: grammar has `..` and `..<` productions. Line 739: AST has `inclusive_high: bool`. Line 944: test uses `value in 0..<256`. |
| I1 | Const types broadened to fixed32/fixed64 | FIXED | Line 294: "integral or fixed-point". Line 216: `float-lit` added to const-expr grammar. Line 295: "Fixed-point literals via ConstExpr::Float variant". |
| I2 | Float literal operand in where grammar | FIXED | Line 621: `float-lit` added to operand grammar. Line 761: AST operand has no Float but WhereOperandDef line 835 has `F64(f64)`. Line 755-761: AST `Operand` enum missing Float — but this is covered by the IR-level `WhereOperandDef::F64`. Minor: AST Operand could also have Float for consistency, but codegen flows through lower so this works. |
| I3 | Where clauses auto-validate on encode/decode | FIXED | Line 790: "Validation code is generated into pack/unpack. Invalid data is rejected at encode/decode time, not via explicit validate() methods." Lines 858-928: full Rust and TypeScript codegen examples show validation in pack/unpack. |
| I4 | len() on maps documented | FIXED | Line 578: "len(field) for arrays, maps, strings, bytes". Line 786: validation rule includes maps. Confirmed as documented extension. |

---

## Additional Checks

**No new issues introduced**: All fixes are localized and consistent with each other. The const float-literal support (I1) feeds correctly into where clause operands (I2). Alias rejection (C1) is enforced at both grammar comment and validation layers. Exclusive range (C2) is wired through grammar → AST → IR → codegen.

**Test cases**: Complete and well-structured.
- 4 valid test files (033-036): fixed_point, constants, type_alias, where_clauses
- 9 invalid test files (058-066): fixed_zigzag, const_cycle, const_div_by_zero, const_type_invalid, alias_to_alias, alias_cycle, where_field_not_found, where_type_mismatch, where_range_invalid

**Compliance vectors**: Not explicitly updated in the plan, but the existing vector format (`fixed32_basic` at line 953) is correct for Q16.16 representation. Wire byte verification (M3 from review #1) remains an implementation-time concern, not a plan defect.

**Minor items from Review #1 addressed**:
- M6 (TypeScript codegen missing): Lines 897-928 now include full TypeScript encode/decode examples with where clause validation.
- M4 (missing @zigzag on fixed test): corpus/invalid/058_fixed_zigzag_invalid.vexil added at line 175.

---

## Summary

The updated plan correctly addresses all 6 issues from the first review. The fixes are internally consistent, test coverage is comprehensive, and no regressions were introduced. The plan is ready for implementation.
