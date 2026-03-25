# Milestone E: Canonical Form + BLAKE3 Schema Hash — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement deterministic canonical form and BLAKE3 schema hash for single-file Vexil schemas, wire the hash into codegen output.

**Architecture:** New `canonical` module in `vexil-lang` walks the `CompiledSchema` IR to produce a normalized single-space-delimited string. BLAKE3 hashes it to 32 bytes. `vexil-codegen` emits the hash as `SCHEMA_HASH` constant. `vexilc check` prints it.

**Tech Stack:** `blake3 = "1"` (pure Rust, default features)

**Spec:** `docs/superpowers/specs/2026-03-25-milestone-e-canonical-hash-design.md`

---

## File Structure

| File | Responsibility |
|---|---|
| `crates/vexil-lang/Cargo.toml` | Add `blake3` dependency |
| `crates/vexil-lang/src/canonical.rs` | `canonical_form()`, `schema_hash()`, type/annotation/field emitters, tests |
| `crates/vexil-lang/src/lib.rs` | Add `pub mod canonical;` |
| `crates/vexil-codegen/src/lib.rs` | Emit `SCHEMA_HASH` constant, reorder HASH before VERSION |
| `crates/vexil-codegen/tests/golden/*.rs` | Updated golden files |
| `crates/vexilc/src/main.rs` | Print hash in `cmd_check` |

---

## Task 1: Add `blake3` dependency + scaffold `canonical` module

**Files:**
- Modify: `crates/vexil-lang/Cargo.toml`
- Create: `crates/vexil-lang/src/canonical.rs`
- Modify: `crates/vexil-lang/src/lib.rs`

- [ ] **Step 1: Add blake3 to Cargo.toml**

In `crates/vexil-lang/Cargo.toml`, add to `[dependencies]`:
```toml
blake3 = "1"
```

- [ ] **Step 2: Create `canonical.rs` with public stubs**

`crates/vexil-lang/src/canonical.rs`:

```rust
use crate::ir::CompiledSchema;

/// Compute the canonical form of a single-file schema per spec §7.
/// Returns a deterministic UTF-8 string — single-space-delimited, no newlines.
pub fn canonical_form(compiled: &CompiledSchema) -> String {
    let mut out = String::new();
    // namespace
    out.push_str("namespace ");
    out.push_str(&compiled.namespace.join("."));
    // TODO: schema-level annotations, declarations
    out
}

/// Compute the BLAKE3 hash of the canonical form.
pub fn schema_hash(compiled: &CompiledSchema) -> [u8; 32] {
    let form = canonical_form(compiled);
    *blake3::hash(form.as_bytes()).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_namespace_only() {
        let result = crate::compile("namespace test.minimal\nmessage Empty {}");
        let compiled = result.compiled.unwrap();
        let form = canonical_form(&compiled);
        assert!(form.starts_with("namespace test.minimal"));
        // Hash is 32 bytes
        let hash = schema_hash(&compiled);
        assert_eq!(hash.len(), 32);
    }
}
```

- [ ] **Step 3: Add `pub mod canonical;` to lib.rs**

In `crates/vexil-lang/src/lib.rs`, add after `pub mod validate;`:
```rust
pub mod canonical;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p vexil-lang`

- [ ] **Step 5: Run the stub test**

Run: `cargo test -p vexil-lang canonical`
Expected: 1 test passes.

- [ ] **Step 6: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-lang/Cargo.toml crates/vexil-lang/src/canonical.rs crates/vexil-lang/src/lib.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): scaffold canonical module + blake3 dependency"
```

---

## Task 2: Type string emission

**Files:**
- Modify: `crates/vexil-lang/src/canonical.rs`

- [ ] **Step 1: Write type string tests**

Add to `canonical.rs` tests module:

```rust
use crate::ast::{PrimitiveType, SemanticType, SubByteType};
use crate::ir::ResolvedType;

#[test]
fn type_string_primitives() {
    assert_eq!(type_str(&ResolvedType::Primitive(PrimitiveType::Bool), &dummy_registry()), "bool");
    assert_eq!(type_str(&ResolvedType::Primitive(PrimitiveType::U32), &dummy_registry()), "u32");
    assert_eq!(type_str(&ResolvedType::Primitive(PrimitiveType::I64), &dummy_registry()), "i64");
    assert_eq!(type_str(&ResolvedType::Primitive(PrimitiveType::F64), &dummy_registry()), "f64");
    assert_eq!(type_str(&ResolvedType::Primitive(PrimitiveType::Void), &dummy_registry()), "void");
}

#[test]
fn type_string_sub_byte() {
    assert_eq!(type_str(&ResolvedType::SubByte(SubByteType { bits: 3, signed: false }), &dummy_registry()), "u3");
    assert_eq!(type_str(&ResolvedType::SubByte(SubByteType { bits: 5, signed: true }), &dummy_registry()), "i5");
}

#[test]
fn type_string_semantic() {
    assert_eq!(type_str(&ResolvedType::Semantic(SemanticType::String), &dummy_registry()), "string");
    assert_eq!(type_str(&ResolvedType::Semantic(SemanticType::Uuid), &dummy_registry()), "uuid");
    assert_eq!(type_str(&ResolvedType::Semantic(SemanticType::Timestamp), &dummy_registry()), "timestamp");
}

#[test]
fn type_string_parameterized() {
    let inner = ResolvedType::Primitive(PrimitiveType::U32);
    assert_eq!(
        type_str(&ResolvedType::Optional(Box::new(inner.clone())), &dummy_registry()),
        "optional<u32>"
    );
    assert_eq!(
        type_str(&ResolvedType::Array(Box::new(inner.clone())), &dummy_registry()),
        "array<u32>"
    );
    let key = ResolvedType::Semantic(SemanticType::String);
    assert_eq!(
        type_str(&ResolvedType::Map(Box::new(key), Box::new(inner.clone())), &dummy_registry()),
        "map<string, u32>"
    );
    let ok = ResolvedType::Primitive(PrimitiveType::U32);
    let err = ResolvedType::Semantic(SemanticType::String);
    assert_eq!(
        type_str(&ResolvedType::Result(Box::new(ok), Box::new(err)), &dummy_registry()),
        "result<u32, string>"
    );
}

fn dummy_registry() -> crate::ir::TypeRegistry {
    crate::ir::TypeRegistry::new()
}
```

- [ ] **Step 2: Run tests — they should fail**

Run: `cargo test -p vexil-lang canonical`
Expected: FAIL — `type_str` not defined.

- [ ] **Step 3: Implement `type_str`**

Add to `canonical.rs` (above tests module):

```rust
use crate::ast::{PrimitiveType, SemanticType, SubByteType};
use crate::ir::{CompiledSchema, ResolvedType, TypeDef, TypeRegistry};

fn type_str(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    match ty {
        ResolvedType::Primitive(p) => primitive_str(p).to_string(),
        ResolvedType::SubByte(s) => {
            let prefix = if s.signed { "i" } else { "u" };
            format!("{prefix}{}", s.bits)
        }
        ResolvedType::Semantic(s) => semantic_str(s).to_string(),
        ResolvedType::Named(id) => {
            match registry.get(*id) {
                Some(def) => type_def_name(def).to_string(),
                None => {
                    debug_assert!(false, "unresolved type id in canonical form: {id:?}");
                    "<unresolved>".to_string()
                }
            }
        }
        ResolvedType::Optional(inner) => format!("optional<{}>", type_str(inner, registry)),
        ResolvedType::Array(inner) => format!("array<{}>", type_str(inner, registry)),
        ResolvedType::Map(k, v) => format!("map<{}, {}>", type_str(k, registry), type_str(v, registry)),
        ResolvedType::Result(ok, err) => format!("result<{}, {}>", type_str(ok, registry), type_str(err, registry)),
        _ => {
            debug_assert!(false, "unknown ResolvedType variant in canonical form");
            "<unknown>".to_string()
        }
    }
}

fn primitive_str(p: &PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::Bool => "bool",
        PrimitiveType::U8 => "u8",
        PrimitiveType::U16 => "u16",
        PrimitiveType::U32 => "u32",
        PrimitiveType::U64 => "u64",
        PrimitiveType::I8 => "i8",
        PrimitiveType::I16 => "i16",
        PrimitiveType::I32 => "i32",
        PrimitiveType::I64 => "i64",
        PrimitiveType::F32 => "f32",
        PrimitiveType::F64 => "f64",
        PrimitiveType::Void => "void",
    }
}

fn semantic_str(s: &SemanticType) -> &'static str {
    match s {
        SemanticType::String => "string",
        SemanticType::Bytes => "bytes",
        SemanticType::Rgb => "rgb",
        SemanticType::Uuid => "uuid",
        SemanticType::Timestamp => "timestamp",
        SemanticType::Hash => "hash",
    }
}

fn type_def_name(def: &TypeDef) -> &str {
    match def {
        TypeDef::Message(m) => m.name.as_str(),
        TypeDef::Enum(e) => e.name.as_str(),
        TypeDef::Flags(f) => f.name.as_str(),
        TypeDef::Union(u) => u.name.as_str(),
        TypeDef::Newtype(n) => n.name.as_str(),
        TypeDef::Config(c) => c.name.as_str(),
        _ => {
            debug_assert!(false, "unknown TypeDef variant in canonical form");
            "<unknown>"
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p vexil-lang canonical`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-lang/src/canonical.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): canonical type string emission"
```

---

## Task 3: Annotation + encoding emission

**Files:**
- Modify: `crates/vexil-lang/src/canonical.rs`

- [ ] **Step 1: Write annotation tests**

```rust
#[test]
fn annotation_sorting() {
    let result = crate::compile(r#"
        @version("2.0.0")
        namespace test.anno
        @since("1.0") @doc("hello") @deprecated(reason: "old") @revision(3)
        @non_exhaustive
        enum Foo { A = 0 }
    "#);
    let compiled = result.compiled.unwrap();
    let form = canonical_form(&compiled);
    // Schema annotations: @version after namespace
    assert!(form.contains("namespace test.anno @version(\"2.0.0\")"));
    // Type annotations sorted: deprecated < doc < non_exhaustive < revision < since
    assert!(form.contains("@deprecated(reason: \"old\") @doc(\"hello\") @non_exhaustive @revision(3) @since(\"1.0\") enum Foo"));
}

#[test]
fn encoding_annotations() {
    let result = crate::compile(r#"
        namespace test.enc
        message Enc {
            a @0 : u32 @varint
            b @1 : i32 @zigzag
            c @2 : u64 @delta
            d @3 : u32 @delta @varint
            e @4 : string @limit(100)
        }
    "#);
    let compiled = result.compiled.unwrap();
    let form = canonical_form(&compiled);
    assert!(form.contains("a @0 : u32 @varint"));
    assert!(form.contains("b @1 : i32 @zigzag"));
    assert!(form.contains("c @2 : u64 @delta"));
    assert!(form.contains("d @3 : u32 @delta @varint"));
    assert!(form.contains("e @4 : string @limit(100)"));
}
```

- [ ] **Step 2: Run tests — they should fail**

- [ ] **Step 3: Implement annotation and encoding emission**

Add to `canonical.rs`:

```rust
use crate::ir::{Encoding, FieldEncoding, ResolvedAnnotations, TombstoneDef};

fn emit_annotations(out: &mut String, ann: &ResolvedAnnotations) {
    // Sorted lexicographically: deprecated, doc, non_exhaustive, revision, since, version
    if let Some(ref dep) = ann.deprecated {
        out.push_str("@deprecated(reason: \"");
        out.push_str(&dep.reason);
        out.push('"');
        if let Some(ref since) = dep.since {
            out.push_str(", since: \"");
            out.push_str(since);
            out.push('"');
        }
        out.push_str(") ");
    }
    for doc in &ann.doc {
        out.push_str("@doc(\"");
        out.push_str(doc);
        out.push_str("\") ");
    }
    if ann.non_exhaustive {
        out.push_str("@non_exhaustive ");
    }
    if let Some(rev) = ann.revision {
        out.push_str(&format!("@revision({rev}) "));
    }
    if let Some(ref since) = ann.since {
        out.push_str("@since(\"");
        out.push_str(since);
        out.push_str("\") ");
    }
    if let Some(ref version) = ann.version {
        out.push_str("@version(\"");
        out.push_str(version);
        out.push_str("\") ");
    }
}

fn emit_encoding(out: &mut String, enc: &FieldEncoding) {
    emit_encoding_inner(out, &enc.encoding);
    if let Some(limit) = enc.limit {
        out.push_str(&format!("@limit({limit}) "));
    }
}

fn emit_encoding_inner(out: &mut String, enc: &Encoding) {
    match enc {
        Encoding::Default => {}
        Encoding::Varint => out.push_str("@varint "),
        Encoding::ZigZag => out.push_str("@zigzag "),
        Encoding::Delta(inner) => {
            out.push_str("@delta ");
            emit_encoding_inner(out, inner);
        }
        _ => {
            debug_assert!(false, "unknown Encoding variant in canonical form");
        }
    }
}

fn emit_tombstones(out: &mut String, tombstones: &[TombstoneDef]) {
    let mut sorted: Vec<_> = tombstones.iter().collect();
    sorted.sort_by_key(|t| t.ordinal);
    for t in sorted {
        out.push_str(&format!("@removed({}, \"{}\"", t.ordinal, t.reason));
        if let Some(ref since) = t.since {
            out.push_str(&format!(", since: \"{since}\""));
        }
        out.push_str(") ");
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p vexil-lang canonical`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-lang/src/canonical.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): canonical annotation + encoding emission"
```

---

## Task 4: Full declaration emission — all 6 type kinds

**Files:**
- Modify: `crates/vexil-lang/src/canonical.rs`

- [ ] **Step 1: Write tests for each type kind**

```rust
#[test]
fn canonical_message() {
    let result = crate::compile("namespace t.m\nmessage Foo { x @0 : u32 y @1 : string }");
    let form = canonical_form(&result.compiled.unwrap());
    assert!(form.contains("message Foo { x @0 : u32 y @1 : string }"));
}

#[test]
fn canonical_enum() {
    let result = crate::compile("namespace t.e\nenum Color { Red = 0 Green = 1 Blue = 2 }");
    let form = canonical_form(&result.compiled.unwrap());
    assert!(form.contains("enum Color { Red = 0 Green = 1 Blue = 2 }"));
}

#[test]
fn canonical_enum_with_backing() {
    let result = crate::compile("namespace t.e\nenum Small : u8 { A = 0 B = 1 }");
    let form = canonical_form(&result.compiled.unwrap());
    assert!(form.contains("enum Small : u8 { A = 0 B = 1 }"));
}

#[test]
fn canonical_flags() {
    let result = crate::compile("namespace t.f\nflags Perms { Read = 0 Write = 1 Exec = 2 }");
    let form = canonical_form(&result.compiled.unwrap());
    assert!(form.contains("flags Perms { Read = 0 Write = 1 Exec = 2 }"));
}

#[test]
fn canonical_union() {
    let result = crate::compile("namespace t.u\nunion Shape { Circle @0 { radius @0 : f32 } Rect @1 { w @0 : f32 h @1 : f32 } }");
    let form = canonical_form(&result.compiled.unwrap());
    assert!(form.contains("union Shape { Circle @0 { radius @0 : f32 } Rect @1 { w @0 : f32 h @1 : f32 } }"));
}

#[test]
fn canonical_newtype() {
    let result = crate::compile("namespace t.n\nnewtype UserId = u64");
    let form = canonical_form(&result.compiled.unwrap());
    assert!(form.contains("newtype UserId = u64"));
}

#[test]
fn canonical_config() {
    let result = crate::compile("namespace t.c\nconfig Defaults { timeout : u32 = 30 name : string = \"hello\" }");
    let form = canonical_form(&result.compiled.unwrap());
    // Config fields sorted by name: name < timeout
    assert!(form.contains("config Defaults { name : string = \"hello\" timeout : u32 = 30 }"));
}

#[test]
fn canonical_tombstones() {
    let result = crate::compile(r#"
        namespace t.t
        message Evolving {
            name @0 : string
            @removed(1, "replaced by full_name")
            @removed(2, "no longer needed", since: "2.0")
        }
    "#);
    let form = canonical_form(&result.compiled.unwrap());
    assert!(form.contains("@removed(1, \"replaced by full_name\")"));
    assert!(form.contains("@removed(2, \"no longer needed\", since: \"2.0\")"));
}
```

- [ ] **Step 2: Run tests — they should fail**

- [ ] **Step 3: Implement full `canonical_form`**

Replace the stub `canonical_form` with the full implementation. Walk `compiled.declarations`, dispatch on `TypeDef` variant, emit each declaration using the helpers from Tasks 2-3.

For each declaration type:
- **Message:** `{annotations}message {Name} { {fields sorted by ordinal} {tombstones} }`
- **Enum:** `{annotations}enum {Name} [: {backing}] { {variants sorted by ordinal} {tombstones} }`
  - Backing strings: `u8`, `u16`, `u32`, `u64`
- **Flags:** `{annotations}flags {Name} { {bits sorted by bit index} {tombstones} }`
- **Union:** `{annotations}union {Name} { {variants sorted by ordinal, each with fields + variant tombstones} {union tombstones} }`
- **Newtype:** `{annotations}newtype {Name} = {inner_type}`
- **Config:** `{annotations}config {Name} { {fields sorted by name} }`

Schema-level annotations: emit `compiled.annotations` after namespace.

Each field: `{field_annotations}{name} @{ordinal} : {type_str} {encoding}`

Config defaults use `{:?}` for floats, standard formatting for others.

Trim trailing spaces from each segment. The final output should have no trailing space.

- [ ] **Step 4: Run tests**

Run: `cargo test -p vexil-lang canonical`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-lang/src/canonical.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): full canonical form — all 6 type kinds"
```

---

## Task 5: Whitespace invariance + hash stability tests

**Files:**
- Modify: `crates/vexil-lang/src/canonical.rs`

- [ ] **Step 1: Write whitespace invariance test**

```rust
#[test]
fn whitespace_invariance() {
    let compact = "namespace t.w\nmessage Foo { x @0 : u32 }";
    let spacey = "  namespace   t.w  \n\n# comment\n  message   Foo  {  \n  x   @0   :   u32  \n  }  \n";
    let h1 = schema_hash(&crate::compile(compact).compiled.unwrap());
    let h2 = schema_hash(&crate::compile(spacey).compiled.unwrap());
    assert_eq!(h1, h2, "whitespace/comments should not affect hash");
}

#[test]
fn field_order_invariance() {
    // Fields with ordinals written out-of-order in source should produce same canonical form
    let ordered = "namespace t.o\nmessage M { a @0 : u32 b @1 : string }";
    let reversed = "namespace t.o\nmessage M { b @1 : string a @0 : u32 }";
    let h1 = schema_hash(&crate::compile(ordered).compiled.unwrap());
    let h2 = schema_hash(&crate::compile(reversed).compiled.unwrap());
    assert_eq!(h1, h2, "field ordering in source should not affect hash");
}
```

- [ ] **Step 2: Write hash stability tests for corpus files**

Write one test per corpus file (006_message, 007_enum, 008_flags, 009_union, 010_newtype, 011_config, 013_annotations, 016_recursive). Each test should:
1. Read the corpus file
2. Compile it
3. Compute the hash
4. Print the hash bytes with `println!("{:?}", hash)` (for capture on first run)
5. Assert `assert_eq!(hash, EXPECTED)` where EXPECTED is initially a placeholder

```rust
fn corpus_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("corpus/valid")
        .join(format!("{name}.vexil"))
}
```

- [ ] **Step 3: Run tests with `--nocapture`, capture actual hash bytes**

Run: `cargo test -p vexil-lang canonical -- --nocapture`

Capture the printed hash values from each corpus test.

- [ ] **Step 4: Pin hash values — REQUIRED before commit**

Replace ALL placeholder assertions with actual `assert_eq!(hash, [0xAA, 0xBB, ...])` using captured values. **Do NOT commit with any placeholder or `let _ = hash` lines — every corpus test must have a pinned assertion.**

- [ ] **Step 5: Run all tests again**

Run: `cargo test -p vexil-lang canonical`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-lang/src/canonical.rs
VEXIL_COMMIT_TASK=1 git commit -m "test(vexil-lang): canonical form whitespace invariance + hash stability"
```

---

## Task 6: Wire `SCHEMA_HASH` into codegen

**Files:**
- Modify: `crates/vexil-codegen/src/lib.rs`

- [ ] **Step 1: Modify `generate()` to emit `SCHEMA_HASH`**

In `crates/vexil-codegen/src/lib.rs`, after the import lines and before `SCHEMA_VERSION`:

```rust
// Emit SCHEMA_HASH (always, before SCHEMA_VERSION)
let hash = vexil_lang::canonical::schema_hash(compiled);
let hash_str = hash
    .iter()
    .map(|b| format!("0x{b:02x}"))
    .collect::<Vec<_>>()
    .join(", ");
w.line(&format!("pub const SCHEMA_HASH: [u8; 32] = [{hash_str}];"));
```

Move the existing `SCHEMA_VERSION` emission to AFTER the hash (spec §8.2 shows HASH before VERSION).

- [ ] **Step 2: Verify codegen compiles**

Run: `cargo check -p vexil-codegen`

- [ ] **Step 3: Update golden files**

Run: `cd crates/vexil-codegen && UPDATE_GOLDEN=1 cargo test --test golden`

- [ ] **Step 4: Verify golden tests pass**

Run: `cargo test -p vexil-codegen --test golden`

- [ ] **Step 5: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexil-codegen/src/lib.rs crates/vexil-codegen/tests/golden/
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-codegen): emit SCHEMA_HASH constant in generated code"
```

---

## Task 7: `vexilc check` hash output

**Files:**
- Modify: `crates/vexilc/Cargo.toml` (if not already depending on vexil-lang)
- Modify: `crates/vexilc/src/main.rs`

- [ ] **Step 1: Modify `cmd_check` to compile and print hash**

Change `cmd_check` in `main.rs` to use `vexil_lang::compile()` instead of `vexil_lang::parse()`, and on success print the schema hash:

```rust
fn cmd_check(filename: &str) -> i32 {
    let source = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {filename}: {e}");
            return 1;
        }
    };
    let result = vexil_lang::compile(&source);
    for diag in &result.diagnostics {
        render_diagnostic(filename, &source, diag);
    }
    if result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return 1;
    }
    if let Some(ref compiled) = result.compiled {
        let hash = vexil_lang::canonical::schema_hash(compiled);
        let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
        println!("schema hash: {hex}");
    }
    0
}
```

- [ ] **Step 2: Verify it builds**

Run: `cargo build -p vexilc`

- [ ] **Step 3: Smoke test**

Run: `cargo run -p vexilc -- check corpus/valid/006_message.vexil`
Expected: Prints `schema hash: <64 hex chars>` to stdout.

- [ ] **Step 4: Commit**

```bash
VEXIL_COMMIT_TASK=1 git add crates/vexilc/src/main.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexilc): print schema hash on successful check"
```

---

## Task 8: Quality gate

- [ ] **Step 1: Format**

Run: `cargo fmt --all`

- [ ] **Step 2: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 3: All tests**

Run: `cargo test --workspace`

- [ ] **Step 4: Fix any issues, iterate**

- [ ] **Step 5: Commit (if any fixes)**

```bash
VEXIL_COMMIT_TASK=1 git add -A
VEXIL_COMMIT_TASK=1 git commit -m "chore: quality gate fixes for Milestone E"
```
