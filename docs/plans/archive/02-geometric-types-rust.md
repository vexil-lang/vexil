# Rust Geometric Types Fix Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this task.

**Goal:** Emit proper geometric types (Vec2, Vec3, Vec4, Quat, Mat3, Mat4) in Rust codegen instead of plain arrays.

**Architecture:** Add geometric types to vexil-runtime, update Rust codegen to emit these types with proper geometric operations.

**Tech Stack:** Rust, vexil-codegen-rust, vexil-runtime

---

## Current State

Geometric types in Rust backend currently emit as plain arrays:
- `vec2<f32>` → `[f32; 2]`
- `vec3<f32>` → `[f32; 3]`
- etc.

This loses geometric semantics and operations.

## Target State

Geometric types emit as proper types from vexil-runtime:
- `vec2<f32>` → `vexil_runtime::Vec2<f32>`
- `vec3<f32>` → `vexil_runtime::Vec3<f32>`
- `quat<f32>` → `vexil_runtime::Quat<f32>`
- `mat3<f32>` → `vexil_runtime::Mat3<f32>`
- `mat4<f32>` → `vexil_runtime::Mat4<f32>`

---

## Task: Implement Geometric Types in Runtime and Codegen

**Objective:** Add proper geometric types to runtime and wire up Rust codegen.

**Files:**
- Create: `crates/vexil-runtime/src/geometric.rs`
- Modify: `crates/vexil-runtime/src/lib.rs`
- Modify: `crates/vexil-codegen-rust/src/types.rs`

**Step 1: Create geometric.rs with type definitions**

```rust
//! Geometric types for Vexil runtime.
//!
//! Vec2, Vec3, Vec4, Quat, Mat3, Mat4 with basic operations.

use core::ops::{Add, Sub, Mul, Div, Neg};

/// 2-component vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

/// 3-component vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

/// 4-component vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Vec4<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

/// Quaternion.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Quat<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

/// 3x3 matrix (column-major).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Mat3<T> {
    pub cols: [Vec3<T>; 3],
}

/// 4x4 matrix (column-major).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Mat4<T> {
    pub cols: [Vec4<T>; 4],
}

// Implement basic operations for Vec2, Vec3, Vec4
// (component-wise add, sub, mul, div, dot product, etc.)

impl<T: Add<Output = T>> Add for Vec2<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Vec2 { x: self.x + other.x, y: self.y + other.y }
    }
}

// ... similar for Vec3, Vec4, and other operations
```

**Step 2: Add Pack/Unpack impls for geometric types**

```rust
impl<T: Pack> Pack for Vec2<T> {
    fn pack(&self, w: &mut BitWriter) -> Result<(), PackError> {
        self.x.pack(w)?;
        self.y.pack(w)
    }
}

impl<T: Unpack> Unpack for Vec2<T> {
    fn unpack(r: &mut BitReader) -> Result<Self, UnpackError> {
        Ok(Vec2 { x: T::unpack(r)?, y: T::unpack(r)? })
    }
}

// Similar for Vec3, Vec4, Quat, Mat3, Mat4
```

**Step 3: Export from lib.rs**

```rust
pub mod geometric;
pub use geometric::{Vec2, Vec3, Vec4, Quat, Mat3, Mat4};
```

**Step 4: Update Rust codegen to emit geometric types**

In `crates/vexil-codegen-rust/src/types.rs`, change `Type::Vec2(elem) =>` from:
```rust
Type::Vec2(elem) => format!("[{}; 2]", rust_type(elem)),
```

To:
```rust
Type::Vec2(elem) => format!("vexil_runtime::Vec2<{}>", rust_type(elem)),
```

Similar for Vec3, Vec4, Quat, Mat3, Mat4.

**Step 5: Build and test**

Run:
```bash
cargo build -p vexil-runtime
cargo build -p vexil-codegen-rust
cargo test -p vexil-codegen-rust
```

**Step 6: Commit**

```bash
git add crates/vexil-runtime/src/geometric.rs crates/vexil-runtime/src/lib.rs
git add crates/vexil-codegen-rust/src/types.rs
git commit -m "feat: implement proper geometric types in Rust runtime and codegen"
```

---

**Summary:** Add Vec2/3/4, Quat, Mat3/4 to runtime with Pack/Unpack, update codegen to emit them.
