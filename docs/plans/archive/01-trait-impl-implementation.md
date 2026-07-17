# Trait/Impl System Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Make trait/impl declarations actually work — lower to IR, check conformance, enable compile-time structural contracts.

**Architecture:** Traits define required fields/functions. Impls provide implementations for specific types. The compiler checks that types implementing traits have the required structure. No wire impact — pure compile-time feature.

**Tech Stack:** Rust, vexil-lang crate (ast, ir, lower, typeck, validate modules)

---

## Current State

- AST: `TraitDecl` and `ImplDecl` exist and parse correctly
- Validation: `check_impl()` exists but only checks existence, not conformance
- IR: NO `TraitDef` or `ImplDef` — they hit `continue` in lower.rs and are discarded
- Typeck: No trait conformance checking
- Codegen: No trait/impl output (but they have "zero wire impact" anyway)

## Target State

- IR has `TraitDef` and `ImplDef` with full field/function info
- Lowering creates these IR nodes
- Typeck validates that impls conform to their traits
- Functions in impls can be called (future: codegen, for now just validated)

---

## Task 1: Add TraitDef and ImplDef to IR

**Objective:** Create IR types to represent traits and impls after lowering.

**Files:**
- Modify: `crates/vexil-lang/src/ir/mod.rs`

**Step 1: Add TraitDef struct**

Add after `GenericAliasDef` (around line 194):

```rust
/// A trait definition defining required fields and functions.
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: SmolStr,
    pub type_params: Vec<ir::types::TypeParam>,
    pub fields: Vec<TraitFieldDef>,
    pub functions: Vec<TraitFnDef>,
    pub annotations: ResolvedAnnotations,
    pub span: Span,
}

/// Required field in a trait.
#[derive(Debug, Clone)]
pub struct TraitFieldDef {
    pub name: SmolStr,
    pub ty: ResolvedType,
    pub ordinal: u32,
    pub annotations: ResolvedAnnotations,
}

/// Function signature in a trait.
#[derive(Debug, Clone)]
pub struct TraitFnDef {
    pub name: SmolStr,
    pub params: Vec<FnParamDef>,
    pub return_type: Option<ResolvedType>,
}

/// Function parameter definition.
#[derive(Debug, Clone)]
pub struct FnParamDef {
    pub name: SmolStr,
    pub ty: ResolvedType,
}
```

**Step 2: Add ImplDef struct**

Add after `TraitDef`:

```rust
/// An implementation of a trait for a specific type.
#[derive(Debug, Clone)]
pub struct ImplDef {
    pub trait_name: SmolStr,
    pub target_type: ResolvedType,
    pub type_args: Vec<ResolvedType>, // Concrete types for trait generics
    pub functions: Vec<ImplFnDef>,
    pub annotations: ResolvedAnnotations,
    pub span: Span,
}

/// Function implementation in an impl block.
#[derive(Debug, Clone)]
pub struct ImplFnDef {
    pub name: SmolStr,
    pub params: Vec<FnParamDef>,
    pub return_type: Option<ResolvedType>,
    pub body: FnBody, // For now, just store signature; body comes later
}

/// Function body (placeholder for now).
#[derive(Debug, Clone)]
pub enum FnBody {
    /// Implemented via FFI or native code (no Vexil source body).
    External,
    /// TODO: Add expression-based body when we have expressions.
    Unimplemented,
}
```

**Step 3: Add variants to TypeDef enum**

Modify `TypeDef` enum (around line 179):

```rust
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TypeDef {
    Message(MessageDef),
    Enum(EnumDef),
    Flags(FlagsDef),
    Union(UnionDef),
    Newtype(NewtypeDef),
    Config(ConfigDef),
    GenericAlias(GenericAliasDef),
    /// A trait definition (compile-time contract, no wire encoding).
    Trait(TraitDef),
    /// An implementation of a trait for a type (compile-time only).
    Impl(ImplDef),
}
```

**Step 4: Update type_names() and find_type()**

In `CompiledSchema::type_names()` (around line 113), add:
```rust
TypeDef::Trait(t) => t.name.as_str(),
TypeDef::Impl(_) => continue, // Impls don't have a simple name
```

In `CompiledSchema::find_type()` (around line 129), add:
```rust
TypeDef::Trait(t) => t.name.as_str(),
TypeDef::Impl(_) => continue, // Skip impls in name lookup
```

**Step 5: Build and verify**

Run: `cargo build -p vexil-lang`
Expected: Compiles successfully (new IR types unused yet)

**Step 6: Commit**

```bash
git add crates/vexil-lang/src/ir/mod.rs
git commit -m "ir: add TraitDef and ImplDef structures"
```

---

## Task 2: Add lower_trait function

**Objective:** Lower AST TraitDecl to IR TraitDef.

**Files:**
- Modify: `crates/vexil-lang/src/lower.rs`

**Step 1: Add lower_trait function**

Add after `lower_config` function (around where other lower_* functions end):

```rust
/// Lower a trait declaration to IR.
fn lower_trait(
    decl: &ast::TraitDecl,
    span: Span,
    ctx: &mut LowerContext,
) -> ir::TraitDef {
    let name = decl.name.node.clone();
    
    // Lower type parameters
    let type_params = decl.type_params.iter()
        .map(|tp| ir::types::TypeParam {
            name: tp.name.node.clone(),
            constraints: vec![], // TODO: constraint lowering
        })
        .collect();
    
    // Lower required fields
    let mut fields = Vec::new();
    for field in &decl.fields {
        let field_def = ir::TraitFieldDef {
            name: field.node.name.node.clone(),
            ty: lower_type(&field.node.ty, ctx),
            ordinal: field.node.ordinal.node,
            annotations: lower_annotations(&field.node.pre_annotations, ctx)
                .merge(lower_annotations(&field.node.post_ordinal_annotations, ctx))
                .merge(lower_annotations(&field.node.post_type_annotations, ctx)),
        };
        fields.push(field_def);
    }
    
    // Lower function signatures
    let functions = decl.functions.iter()
        .map(|fn_decl| ir::TraitFnDef {
            name: fn_decl.name.node.clone(),
            params: fn_decl.params.iter()
                .map(|p| ir::FnParamDef {
                    name: p.name.node.clone(),
                    ty: lower_type(&p.ty, ctx),
                })
                .collect(),
            return_type: fn_decl.return_type.as_ref()
                .map(|ty| lower_type(ty, ctx)),
        })
        .collect();
    
    ir::TraitDef {
        name,
        type_params,
        fields,
        functions,
        annotations: lower_annotations(&decl.annotations, ctx),
        span,
    }
}
```

**Step 2: Wire up in lower_decl**

Find the match on `Decl::Trait(_)` in `lower_decl` (around line 392). Change from:
```rust
Decl::Alias(_) | Decl::Const(_) | Decl::Trait(_) | Decl::Impl(_) => {
    continue;
}
```

To:
```rust
Decl::Alias(_) | Decl::Const(_) | Decl::Impl(_) => {
    continue;
}
Decl::Trait(d) => {
    let type_def = ir::TypeDef::Trait(lower_trait(d, span, ctx));
    let id = ctx.registry_mut().alloc(type_def);
    ctx.type_map.insert(name.clone(), id);
    continue;
}
```

Wait — need to handle the name. Look at how the name is extracted. In the first pass (around line 360), we need to also handle Trait names for type_map. Find:
```rust
Decl::Trait(_) => continue, // Traits don't get TypeIds
```

Change to:
```rust
Decl::Trait(d) => d.name.node.clone(),
```

**Step 3: Build and test**

Run: `cargo build -p vexil-lang`
Expected: Compiles successfully

Run: `cargo test -p vexil-lang`
Expected: Existing tests still pass

**Step 4: Commit**

```bash
git add crates/vexil-lang/src/lower.rs
git commit -m "lower: implement trait lowering to IR"
```

---

## Task 3: Add lower_impl function

**Objective:** Lower AST ImplDecl to IR ImplDef.

**Files:**
- Modify: `crates/vexil-lang/src/lower.rs`

**Step 1: Add lower_impl function**

Add after `lower_trait`:

```rust
/// Lower an impl declaration to IR.
fn lower_impl(
    decl: &ast::ImplDecl,
    span: Span,
    ctx: &mut LowerContext,
) -> ir::ImplDef {
    let trait_name = decl.trait_name.node.clone();
    let target_type = lower_type(&decl.target_type, ctx);
    
    // TODO: Handle type arguments for generic traits
    let type_args = vec![];
    
    // Lower function implementations
    let functions = decl.functions.iter()
        .map(|fn_impl| ir::ImplFnDef {
            name: fn_impl.name.node.clone(),
            params: fn_impl.params.iter()
                .map(|p| ir::FnParamDef {
                    name: p.name.node.clone(),
                    ty: lower_type(&p.ty, ctx),
                })
                .collect(),
            return_type: fn_impl.return_type.as_ref()
                .map(|ty| lower_type(ty, ctx)),
            body: ir::FnBody::External, // TODO: handle actual body expressions
        })
        .collect();
    
    ir::ImplDef {
        trait_name,
        target_type,
        type_args,
        functions,
        annotations: lower_annotations(&decl.annotations, ctx),
        span,
    }
}
```

**Step 2: Wire up in lower_decl**

Change the impl handling in `lower_decl` from `continue` to:
```rust
Decl::Impl(d) => {
    let type_def = ir::TypeDef::Impl(lower_impl(d, span, ctx));
    let id = ctx.registry_mut().alloc(type_def);
    // Impls don't go in type_map by name (they're trait+type)
    ctx.impls.push(id);
    continue;
}
```

**Step 3: Add impls vector to LowerContext**

Find `struct LowerContext` and add:
```rust
/// Collected impl definitions for conformance checking.
pub impls: Vec<ir::TypeId>,
```

Initialize it in the creation of LowerContext.

**Step 4: Build and test**

Run: `cargo build -p vexil-lang`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add crates/vexil-lang/src/lower.rs
git commit -m "lower: implement impl lowering to IR"
```

---

## Task 4: Implement trait conformance checking in typeck

**Objective:** Validate that impls actually match their traits.

**Files:**
- Modify: `crates/vexil-lang/src/typeck.rs`

**Step 1: Add trait_conformance module/section**

At the end of typeck.rs, add:

```rust
// ---------------------------------------------------------------------------
// Trait Conformance Checking
// ---------------------------------------------------------------------------

/// Check that all impls in the schema conform to their traits.
pub fn check_impl_conformance(ctx: &mut TypeckContext, diags: &mut Vec<Diagnostic>) {
    // Collect all traits and impls
    let mut traits: HashMap<SmolStr, &ir::TraitDef> = HashMap::new();
    let mut impls: Vec<&ir::ImplDef> = Vec::new();
    
    for &id in &ctx.schema.declarations {
        if let Some(type_def) = ctx.schema.registry.get(id) {
            match type_def {
                ir::TypeDef::Trait(t) => { traits.insert(t.name.clone(), t); }
                ir::TypeDef::Impl(i) => { impls.push(i); }
                _ => {}
            }
        }
    }
    
    // Check each impl
    for impl_def in impls {
        check_single_impl_conformance(impl_def, &traits, ctx, diags);
    }
}

fn check_single_impl_conformance(
    impl_def: &ir::ImplDef,
    traits: &HashMap<SmolStr, &ir::TraitDef>,
    ctx: &TypeckContext,
    diags: &mut Vec<Diagnostic>,
) {
    let Some(trait_def) = traits.get(&impl_def.trait_name) else {
        diags.push(Diagnostic::error(
            impl_def.span,
            ErrorClass::TypeMismatch, // or create UnknownTrait
            format!("impl references unknown trait '{}'", impl_def.trait_name),
        ));
        return;
    };
    
    // Check target type has all required trait fields
    check_trait_fields(impl_def, trait_def, ctx, diags);
    
    // Check all trait functions are implemented
    check_trait_functions(impl_def, trait_def, diags);
}

fn check_trait_fields(
    impl_def: &ir::ImplDef,
    trait_def: &ir::TraitDef,
    ctx: &TypeckContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Get the target type definition
    let target_type_def = match &impl_def.target_type {
        ir::ResolvedType::Named(name, _) => {
            ctx.schema.declarations.iter()
                .filter_map(|&id| ctx.schema.registry.get(id).map(|d| (id, d)))
                .find(|(_, d)| matches!(d, ir::TypeDef::Message(m) if m.name == *name))
                .map(|(_, d)| d)
        }
        _ => None,
    };
    
    let Some(target_def) = target_type_def else {
        // Type validation happens elsewhere
        return;
    };
    
    let target_fields = match target_def {
        ir::TypeDef::Message(m) => &m.fields,
        _ => {
            diags.push(Diagnostic::error(
                impl_def.span,
                ErrorClass::TypeMismatch,
                format!("impl target '{}' is not a message type", impl_def.target_type),
            ));
            return;
        }
    };
    
    // Check each required trait field exists on target
    for trait_field in &trait_def.fields {
        let found = target_fields.iter().any(|f| {
            f.name == trait_field.name && types_compatible(&f.ty, &trait_field.ty)
        });
        
        if !found {
            diags.push(Diagnostic::error(
                impl_def.span,
                ErrorClass::TypeMismatch, // or MissingTraitField
                format!(
                    "impl for '{}' missing required trait field '{}' of type '{}'",
                    impl_def.target_type, trait_field.name, trait_field.ty
                ),
            ));
        }
    }
}

fn check_trait_functions(
    impl_def: &ir::ImplDef,
    trait_def: &ir::TraitDef,
    diags: &mut Vec<Diagnostic>,
) {
    // Check all trait functions are implemented
    for trait_fn in &trait_def.functions {
        let found = impl_def.functions.iter().any(|f| {
            f.name == trait_fn.name 
                && f.params.len() == trait_fn.params.len()
                && f.return_type == trait_fn.return_type
        });
        
        if !found {
            diags.push(Diagnostic::error(
                impl_def.span,
                ErrorClass::TypeMismatch, // or MissingTraitFn
                format!(
                    "impl for '{}' missing trait function '{}'",
                    impl_def.target_type, trait_fn.name
                ),
            ));
        }
    }
    
    // TODO: Check for extra functions in impl that aren't in trait?
}

fn types_compatible(a: &ir::ResolvedType, b: &ir::ResolvedType) -> bool {
    // For now, exact match only
    // TODO: handle generic substitution, subtyping
    a == b
}
```

**Step 2: Wire into typeck_main**

Find `typeck_main` function and add call to `check_impl_conformance`:

```rust
pub fn typeck_main(ctx: &mut TypeckContext) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    
    // ... existing checks ...
    
    // Check trait conformance
    check_impl_conformance(ctx, &mut diags);
    
    diags
}
```

**Step 3: Build and test**

Run: `cargo build -p vexil-lang`
Expected: Compiles successfully

Run: `cargo test -p vexil-lang`
Expected: Tests pass, trait/impl schemas now fully processed

**Step 4: Commit**

```bash
git add crates/vexil-lang/src/typeck.rs
git commit -m "typeck: add trait conformance checking for impl blocks"
```

---

## Task 5: Add corpus test for trait/impl

**Objective:** Verify trait/impl works end-to-end with test schema.

**Files:**
- Create: `corpus/valid/042_trait_impl.vexil`

**Step 1: Create test schema**

```vexil
// Trait/impl system test

namespace test.trait_impl;

/// Trait for timestamped messages
trait Timestamped {
    @1 timestamp: u64;
    
    fn get_timestamp() -> u64;
}

/// Event with timestamp
message Event {
    @1 timestamp: u64;
    @2 data: bytes;
}

/// Impl of Timestamped for Event
impl Timestamped for Event {
    fn get_timestamp() -> u64;
}
```

**Step 2: Verify it compiles**

Run: `cargo test -p vexil-lang --test corpus_test`
Or manually: Build and check the test file parses/lowers

**Step 3: Commit**

```bash
git add corpus/valid/042_trait_impl.vexil
git commit -m "test: add trait/impl corpus test"
```

---

## Task 6: Update README/CLAUDE.md

**Objective:** Mark trait/impl as actually implemented.

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`

**Step 1: Find trait mentions in README**

Search for "trait" and update any "not yet implemented" language.

**Step 2: Update CLAUDE.md status section**

Find the trait/impl entry and mark complete.

**Step 3: Commit**

```bash
git add README.md CLAUDE.md
git commit -m "docs: mark trait/impl as implemented"
```

---

## Final Verification

Run full test suite:
```bash
cargo test -p vexil-lang
cargo test -p vexilc
cargo clippy -p vexil-lang -- -D warnings
```

Expected: All pass, no new warnings.

---

**Summary of Changes:**

| File | Change |
|------|--------|
| `ir/mod.rs` | +TraitDef, +ImplDef, +supporting types |
| `lower.rs` | +lower_trait(), +lower_impl(), wired into lowering |
| `typeck.rs` | +trait conformance checking |
| `corpus/valid/042_trait_impl.vexil` | New test schema |
| `README.md`, `CLAUDE.md` | Documentation updates |

**Estimated Time:** 2-3 hours with subagent delegation
