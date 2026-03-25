# CLAUDE.md

## What is Vexil?

Vexil (Validated Exchange Language) is a typed schema definition language with first-class encoding semantics. It describes the shape, constraints, and wire encoding of data crossing system boundaries.

This repo contains the language specification, formal grammar, test corpus, and (eventually) the reference implementation.

## Repo Structure

```
spec/
  vexil-spec.md        # Language specification (normative)
  vexil-grammar.peg    # Formal PEG grammar derived from spec
corpus/
  MANIFEST.md          # Index of all test files with spec references
  valid/               # 18 files — conformant impl MUST accept all
  invalid/             # 56 files — conformant impl MUST reject all
```

## Build Sequence

spec → formal grammar → test corpus → reference implementation

## Key Design Decisions

- **Separate AST and IR** in the compiler pipeline. AST is source-faithful (for errors, formatting, LSP). IR is fully resolved and name-free (for type checking and codegen). Design for future complexity.
- **Compiler pipeline:** Lexer → Parser → AST → Lowering → IR → Type Checker → Validated IR → Codegen
- **Wire encoding:** LSB-first bit packing, LEB128 varints, ZigZag signed encoding, BLAKE3 schema hashing
- **No self-description on the wire** — schema is the contract

## Naming

- **Vexil** — the schema definition language (this repo)
- **MALT** — the terminal platform
- **MASH** — the shell
- **maltty** — the GPU renderer
- **VNP** — Vexil Native Protocol (wire protocol using Vexil schemas)

## Git Workflow

- A pre-push hook runs `cargo fmt --all` and aborts if it modifies files.
- Agents MUST bypass it with `VEXIL_NO_FMT=1 git push origin main` to avoid blocking on fmt.
- Always `git pull origin main` before starting local edits — multiple agents and direct GitHub commits are common.

## Code Standards (for reference implementation, when it exists)

- Rust, edition 2021
- `thiserror` for error types
- `#[derive(Debug, Clone, PartialEq)]` on data types
- No `unwrap()` or `expect()` in non-test code
- All `unsafe` blocks require `// SAFETY:` comments
