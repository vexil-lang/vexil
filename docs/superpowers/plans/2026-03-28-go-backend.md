# Go Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Go code generation backend and runtime module, feature-complete with Rust and TypeScript — including delta encoding, unknown field preservation, typed tombstones, SchemaHandshake, and compliance vector testing.

**Architecture:** Rust codegen crate (`vexil-codegen-go`) implementing `CodegenBackend`, emitting Go source. Hand-written Go runtime module (`packages/runtime-go/`) with zero dependencies. CLI integration via `--target go`. Compliance testing against shared golden byte vectors.

**Tech Stack:** Rust (codegen crate), Go 1.22+ (runtime module).

**Spec reference:** `docs/superpowers/specs/2026-03-28-go-backend-design.md`

---

## File Structure

```
crates/
  vexil-codegen-go/                        # NEW Rust crate
    Cargo.toml
    src/
      lib.rs                               # generate(), GoBackend re-export
      backend.rs                           # CodegenBackend impl, project generation
      emit.rs                              # CodeWriter (tab-indented for Go)
      types.rs                             # Vexil→Go type mapping
      message.rs                           # Message/Config: struct + Pack/Unpack
      enum_gen.rs                          # Enum: int type + iota constants
      flags.rs                             # Flags: uint type + bit constants
      union_gen.rs                         # Union: interface + variant structs
      newtype.rs                           # Newtype: type alias + Pack/Unpack
      delta.rs                             # Delta encoder/decoder structs
    tests/
      golden.rs                            # Golden output tests
      golden/                              # .go golden files
  vexilc/
    src/main.rs                            # MODIFY: add "go" target dispatch
    Cargo.toml                             # MODIFY: add vexil-codegen-go dependency
packages/
  runtime-go/                              # NEW Go module
    go.mod
    bitwriter.go                           # BitWriter struct
    bitwriter_test.go
    bitreader.go                           # BitReader struct
    bitreader_test.go
    errors.go                              # EncodeError, DecodeError
    interfaces.go                          # Packer, Unpacker interfaces
    leb128.go                              # LEB128 encode/decode
    zigzag.go                              # ZigZag encode/decode
    handshake.go                           # SchemaHandshake
    handshake_test.go
    compliance_test.go                     # Golden vector tests
    delta_compliance_test.go               # Delta vector tests
Cargo.toml                                 # MODIFY: add workspace member
.github/workflows/ci.yml                   # MODIFY: add go-runtime job
```

---

## Task 1: Go Runtime — BitWriter

**Files:**
- Create: `packages/runtime-go/go.mod`
- Create: `packages/runtime-go/errors.go`
- Create: `packages/runtime-go/interfaces.go`
- Create: `packages/runtime-go/leb128.go`
- Create: `packages/runtime-go/bitwriter.go`
- Create: `packages/runtime-go/bitwriter_test.go`

This is the foundation. The Go BitWriter must match the Rust BitWriter byte-for-byte.

- [ ] **Step 1: Create go.mod**

```
module github.com/vexil-lang/vexil/packages/runtime-go

go 1.22
```

- [ ] **Step 2: Create errors.go**

```go
package vexil

import "fmt"

type EncodeError struct {
	Field   string
	Message string
}

func (e *EncodeError) Error() string {
	return fmt.Sprintf("encode error on field %q: %s", e.Field, e.Message)
}

type DecodeError struct {
	Message string
}

func (e *DecodeError) Error() string {
	return fmt.Sprintf("decode error: %s", e.Message)
}

var (
	ErrUnexpectedEOF        = &DecodeError{Message: "unexpected end of input"}
	ErrInvalidUTF8          = &DecodeError{Message: "invalid UTF-8 in string field"}
	ErrInvalidVarint        = &DecodeError{Message: "invalid or overlong varint encoding"}
	ErrRecursionLimit       = &DecodeError{Message: "recursive type nesting exceeded 64 levels"}
	ErrSchemaMismatch       = &DecodeError{Message: "schema hash mismatch"}
)

const MaxRecursionDepth = 64
```

- [ ] **Step 3: Create interfaces.go**

```go
package vexil

type Packer interface {
	Pack(w *BitWriter) error
}

type Unpacker interface {
	Unpack(r *BitReader) error
}
```

- [ ] **Step 4: Create leb128.go**

```go
package vexil

func encodeLeb128(buf []byte, v uint64) int {
	i := 0
	for {
		b := byte(v & 0x7f)
		v >>= 7
		if v != 0 {
			b |= 0x80
		}
		buf[i] = b
		i++
		if v == 0 {
			break
		}
	}
	return i
}
```

- [ ] **Step 5: Create bitwriter.go**

The BitWriter must implement:
- `WriteBool(v bool)`
- `WriteBits(v uint64, count uint8)`
- `FlushToByteBoundary()`
- `WriteU8(v uint8)` through `WriteU64(v uint64)`
- `WriteI8(v int8)` through `WriteI64(v int64)`
- `WriteF32(v float32)` with NaN canonicalization
- `WriteF64(v float64)` with NaN canonicalization
- `WriteLeb128(v uint64)`
- `WriteZigZag(v int64, typeBits uint8)`
- `WriteString(s string)`
- `WriteBytes(data []byte)`
- `WriteRawBytes(data []byte)`
- `EnterRecursive() error`
- `LeaveRecursive()`
- `Finish() []byte`

All methods follow LSB-first bit packing, little-endian byte order, matching the Rust implementation exactly. Read `crates/vexil-runtime/src/bit_writer.rs` for the reference implementation.

Key implementation details:
- Internal state: `buf []byte`, `currentByte byte`, `bitOffset uint8`, `depth uint32`
- `WriteBits`: pack LSB-first within current byte, push byte when full
- `FlushToByteBoundary`: push current byte if bitOffset > 0
- Multi-byte writes (U16, U32, etc.): flush first, then append little-endian bytes
- `WriteF32` NaN: canonicalize to `0x7FC00000` before writing
- `WriteF64` NaN: canonicalize to `0x7FF8000000000000` before writing
- `WriteString`: flush, write LEB128 length, write UTF-8 bytes
- `Finish`: calls `FlushToByteBoundary`, returns `buf`

- [ ] **Step 6: Create bitwriter_test.go**

Test against the compliance vectors:
```go
package vexil

import (
	"encoding/hex"
	"math"
	"testing"
)

func TestWriteBoolFalse(t *testing.T) {
	w := NewBitWriter()
	w.WriteBool(false)
	w.FlushToByteBoundary()
	got := hex.EncodeToString(w.Finish())
	if got != "00" {
		t.Fatalf("bool false: want 00, got %s", got)
	}
}

func TestWriteBoolTrue(t *testing.T) {
	w := NewBitWriter()
	w.WriteBool(true)
	w.FlushToByteBoundary()
	got := hex.EncodeToString(w.Finish())
	if got != "01" {
		t.Fatalf("bool true: want 01, got %s", got)
	}
}

func TestWriteU32LE(t *testing.T) {
	w := NewBitWriter()
	w.WriteU32(305419896)
	got := hex.EncodeToString(w.Finish())
	if got != "78563412" {
		t.Fatalf("u32 LE: want 78563412, got %s", got)
	}
}

func TestWriteF32NaN(t *testing.T) {
	w := NewBitWriter()
	w.WriteF32(float32(math.NaN()))
	got := hex.EncodeToString(w.Finish())
	if got != "0000c07f" {
		t.Fatalf("f32 NaN: want 0000c07f, got %s", got)
	}
}

func TestWriteStringHello(t *testing.T) {
	w := NewBitWriter()
	w.WriteString("hello")
	got := hex.EncodeToString(w.Finish())
	if got != "0568656c6c6f" {
		t.Fatalf("string hello: want 0568656c6c6f, got %s", got)
	}
}

func TestSubBytePacking(t *testing.T) {
	w := NewBitWriter()
	w.WriteBits(5, 3)  // u3 = 5
	w.WriteBits(18, 5) // u5 = 18
	w.FlushToByteBoundary()
	got := hex.EncodeToString(w.Finish())
	if got != "95" {
		t.Fatalf("u3+u5: want 95, got %s", got)
	}
}

func TestRecursionDepthLimit(t *testing.T) {
	w := NewBitWriter()
	for i := 0; i < 64; i++ {
		if err := w.EnterRecursive(); err != nil {
			t.Fatalf("enter %d failed: %v", i, err)
		}
	}
	if err := w.EnterRecursive(); err == nil {
		t.Fatal("expected error at depth 65")
	}
}
```

Add more tests covering: U8, U16, U64, I32 negative, F64 negative zero, empty string, LEB128, ZigZag.

- [ ] **Step 7: Run tests**

Run: `cd packages/runtime-go && go test ./...`
Expected: All pass.

- [ ] **Step 8: Commit**

```bash
git add packages/runtime-go/
git commit -m "feat(runtime-go): BitWriter with LSB-first bitpack, LEB128, ZigZag, NaN canonicalization"
```

---

## Task 2: Go Runtime — BitReader

**Files:**
- Create: `packages/runtime-go/bitreader.go`
- Create: `packages/runtime-go/bitreader_test.go`

Mirror of BitWriter. Read `crates/vexil-runtime/src/bit_reader.rs` for reference.

- [ ] **Step 1: Create bitreader.go**

Methods to implement:
- `ReadBool() (bool, error)`
- `ReadBits(count uint8) (uint64, error)`
- `FlushToByteBoundary()`
- `ReadU8() (uint8, error)` through `ReadU64() (uint64, error)`
- `ReadI8() (int8, error)` through `ReadI64() (int64, error)`
- `ReadF32() (float32, error)` / `ReadF64() (float64, error)`
- `ReadLeb128() (uint64, error)` — with max bytes limit
- `ReadZigZag(typeBits uint8) (int64, error)`
- `ReadString() (string, error)` — LEB128 length + UTF-8, validate UTF-8
- `ReadBytes() ([]byte, error)` — LEB128 length + raw bytes
- `ReadRawBytes(n int) ([]byte, error)` — exactly n bytes
- `ReadRemaining() []byte` — all remaining bytes
- `EnterRecursive() error`
- `LeaveRecursive()`

Internal state: `data []byte`, `bytePos int`, `bitOffset uint8`, `depth uint32`

Go returns `(value, error)` — every read method returns an error for unexpected EOF.

- [ ] **Step 2: Create bitreader_test.go**

Test decode of the same byte sequences that BitWriter produces:
```go
func TestReadBoolFalse(t *testing.T) {
	r := NewBitReader([]byte{0x00})
	v, err := r.ReadBool()
	if err != nil { t.Fatal(err) }
	if v != false { t.Fatal("expected false") }
}

func TestReadU32LE(t *testing.T) {
	r := NewBitReader([]byte{0x78, 0x56, 0x34, 0x12})
	v, err := r.ReadU32()
	if err != nil { t.Fatal(err) }
	if v != 305419896 { t.Fatalf("want 305419896, got %d", v) }
}

func TestReadRemaining(t *testing.T) {
	r := NewBitReader([]byte{0x2a, 0x00, 0x00, 0x00, 0x63, 0x00})
	_, _ = r.ReadU32() // consume 4 bytes
	remaining := r.ReadRemaining()
	if len(remaining) != 2 { t.Fatalf("want 2 remaining, got %d", len(remaining)) }
}

func TestTrailingBytesTolerated(t *testing.T) {
	r := NewBitReader([]byte{0x2a, 0x00, 0x00, 0x00, 0x63, 0x00})
	v, _ := r.ReadU32()
	if v != 42 { t.Fatalf("want 42, got %d", v) }
	// Remaining bytes don't cause error
}
```

- [ ] **Step 3: Run tests**

Run: `cd packages/runtime-go && go test ./...`

- [ ] **Step 4: Commit**

```bash
git add packages/runtime-go/
git commit -m "feat(runtime-go): BitReader with LSB-first bitpack decode"
```

---

## Task 3: Go Runtime — SchemaHandshake + Compliance Tests

**Files:**
- Create: `packages/runtime-go/handshake.go`
- Create: `packages/runtime-go/handshake_test.go`
- Create: `packages/runtime-go/compliance_test.go`
- Create: `packages/runtime-go/delta_compliance_test.go`
- Create: `packages/runtime-go/zigzag.go`

- [ ] **Step 1: Create zigzag.go**

```go
package vexil

func zigzagEncode(v int64) uint64 {
	return uint64((v << 1) ^ (v >> 63))
}

func zigzagDecode(v uint64) int64 {
	return int64(v>>1) ^ -int64(v&1)
}
```

Wire into BitWriter's `WriteZigZag` and BitReader's `ReadZigZag`.

- [ ] **Step 2: Create handshake.go**

```go
package vexil

type SchemaHandshake struct {
	Hash    [32]byte
	Version string
}

type HandshakeResult struct {
	Match         bool
	LocalVersion  string
	RemoteVersion string
	LocalHash     [32]byte
	RemoteHash    [32]byte
}

func NewSchemaHandshake(hash [32]byte, version string) *SchemaHandshake {
	return &SchemaHandshake{Hash: hash, Version: version}
}

func (h *SchemaHandshake) Encode() []byte {
	w := NewBitWriter()
	w.WriteRawBytes(h.Hash[:])
	w.WriteString(h.Version)
	return w.Finish()
}

func DecodeSchemaHandshake(data []byte) (*SchemaHandshake, error) {
	r := NewBitReader(data)
	hashBytes, err := r.ReadRawBytes(32)
	if err != nil { return nil, err }
	var hash [32]byte
	copy(hash[:], hashBytes)
	version, err := r.ReadString()
	if err != nil { return nil, err }
	return &SchemaHandshake{Hash: hash, Version: version}, nil
}

func (h *SchemaHandshake) Check(remote *SchemaHandshake) HandshakeResult {
	if h.Hash == remote.Hash {
		return HandshakeResult{Match: true}
	}
	return HandshakeResult{
		Match:         false,
		LocalVersion:  h.Version,
		RemoteVersion: remote.Version,
		LocalHash:     h.Hash,
		RemoteHash:    remote.Hash,
	}
}
```

- [ ] **Step 3: Create handshake_test.go**

Test encode/decode round-trip, matching hashes, different hashes, wire size (38 bytes for "1.0.0").

- [ ] **Step 4: Create compliance_test.go**

Read `compliance/vectors/primitives.json`, `sub_byte.json`, `messages.json` — encode with BitWriter and assert bytes match. Use `encoding/json` and `os` to read the vector files.

Path to vectors: `../../compliance/vectors/` relative to the test file.

```go
package vexil

import (
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

type Vector struct {
	Name          string                 `json:"name"`
	ExpectedBytes string                 `json:"expected_bytes"`
	Value         map[string]interface{} `json:"value"`
}

func TestPrimitivesCompliance(t *testing.T) {
	data, err := os.ReadFile(filepath.Join("..", "..", "compliance", "vectors", "primitives.json"))
	if err != nil { t.Fatal(err) }
	var vectors []Vector
	json.Unmarshal(data, &vectors)

	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			w := NewBitWriter()
			switch v.Name {
			case "bool_false":
				w.WriteBool(false)
				w.FlushToByteBoundary()
			case "bool_true":
				w.WriteBool(true)
				w.FlushToByteBoundary()
			case "u8_zero":
				w.WriteU8(0)
			case "u8_max":
				w.WriteU8(255)
			case "u16_le":
				w.WriteU16(258)
			case "u32_le":
				w.WriteU32(305419896)
			case "i32_negative":
				w.WriteI32(-1)
			case "f32_nan_canonical":
				w.WriteF32(float32(math.NaN()))
			case "f64_negative_zero":
				w.WriteF64(math.Copysign(0, -1))
			case "string_hello":
				w.WriteString("hello")
			case "string_empty":
				w.WriteString("")
			default:
				t.Skip("unknown vector")
			}
			got := hex.EncodeToString(w.Finish())
			if got != v.ExpectedBytes {
				t.Errorf("want %s, got %s", v.ExpectedBytes, got)
			}
		})
	}
}
```

- [ ] **Step 5: Create delta_compliance_test.go**

Read `compliance/vectors/delta.json`, encode multi-frame sequences with manual delta computation.

- [ ] **Step 6: Run all tests**

Run: `cd packages/runtime-go && go test -v ./...`

- [ ] **Step 7: Commit**

```bash
git add packages/runtime-go/
git commit -m "feat(runtime-go): SchemaHandshake, compliance vectors, delta compliance"
```

---

## Task 4: Go Codegen Crate — Scaffold + Types + Emit

**Files:**
- Create: `crates/vexil-codegen-go/Cargo.toml`
- Create: `crates/vexil-codegen-go/src/lib.rs`
- Create: `crates/vexil-codegen-go/src/emit.rs`
- Create: `crates/vexil-codegen-go/src/types.rs`
- Create: `crates/vexil-codegen-go/src/backend.rs`
- Modify: `Cargo.toml` (workspace members)

Follow the exact structure of `vexil-codegen-ts`. Read that crate for reference.

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "vexil-codegen-go"
version = "0.2.5"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
readme.workspace = true
description = "Go code generation backend for the Vexil schema compiler"
keywords = ["schema", "codegen", "go", "golang"]
categories = ["compilers", "development-tools"]

[dependencies]
vexil-lang = { path = "../vexil-lang", version = "^0.2.5" }
thiserror = "2"

[dev-dependencies]
test-case = "3"
```

Check the current workspace version — it may have been bumped. Match whatever `vexil-codegen-ts` uses.

- [ ] **Step 2: Add to workspace**

Add `"crates/vexil-codegen-go"` to workspace members in root `Cargo.toml`.

- [ ] **Step 3: Create emit.rs — Go CodeWriter**

Same as TS CodeWriter but uses **tabs** for indentation:

```rust
pub struct CodeWriter {
    buf: String,
    indent: usize,
}

impl CodeWriter {
    pub fn new() -> Self {
        Self { buf: String::new(), indent: 0 }
    }

    pub fn line(&mut self, text: &str) {
        if text.is_empty() {
            self.buf.push('\n');
        } else {
            for _ in 0..self.indent {
                self.buf.push('\t');
            }
            self.buf.push_str(text);
            self.buf.push('\n');
        }
    }

    pub fn blank(&mut self) { self.buf.push('\n'); }

    pub fn open_block(&mut self, header: &str) {
        self.line(&format!("{header} {{"));
        self.indent += 1;
    }

    pub fn close_block(&mut self) {
        self.indent = self.indent.saturating_sub(1);
        self.line("}");
    }

    pub fn close_block_with(&mut self, suffix: &str) {
        self.indent = self.indent.saturating_sub(1);
        self.line(&format!("}}{suffix}"));
    }

    pub fn finish(self) -> String { self.buf }
}
```

- [ ] **Step 4: Create types.rs — Go type mapping**

```rust
use vexil_lang::ir::{PrimitiveType, ResolvedType, SemanticType, SubByteType, TypeDef, TypeId, TypeRegistry};

pub fn go_type(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::U8 => "uint8".to_string(),
            PrimitiveType::U16 => "uint16".to_string(),
            PrimitiveType::U32 => "uint32".to_string(),
            PrimitiveType::U64 => "uint64".to_string(),
            PrimitiveType::I8 => "int8".to_string(),
            PrimitiveType::I16 => "int16".to_string(),
            PrimitiveType::I32 => "int32".to_string(),
            PrimitiveType::I64 => "int64".to_string(),
            PrimitiveType::F32 => "float32".to_string(),
            PrimitiveType::F64 => "float64".to_string(),
            PrimitiveType::Void => "struct{}".to_string(),
            _ => "interface{}".to_string(),
        },
        ResolvedType::SubByte(_) => "uint8".to_string(),
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => "string".to_string(),
            SemanticType::Bytes => "[]byte".to_string(),
            SemanticType::Uuid => "[16]byte".to_string(),
            SemanticType::Timestamp => "int64".to_string(),
            SemanticType::Rgb => "[3]uint8".to_string(),
            SemanticType::Hash => "[32]byte".to_string(),
            _ => "interface{}".to_string(),
        },
        ResolvedType::Named(id) => type_name_for_id(*id, registry),
        ResolvedType::Optional(inner) => format!("*{}", go_type(inner, registry)),
        ResolvedType::Array(inner) => format!("[]{}", go_type(inner, registry)),
        ResolvedType::Map(k, v) => format!("map[{}]{}", go_type(k, registry), go_type(v, registry)),
        ResolvedType::Result(ok, err) => {
            // Go doesn't have sum types — use a struct
            format!("Result{}{}", go_type(ok, registry), go_type(err, registry))
        }
        _ => "interface{}".to_string(),
    }
}

fn type_name_for_id(id: TypeId, registry: &TypeRegistry) -> String {
    registry.get(id)
        .map(|def| match def {
            TypeDef::Message(m) => m.name.to_string(),
            TypeDef::Enum(e) => e.name.to_string(),
            TypeDef::Flags(f) => f.name.to_string(),
            TypeDef::Union(u) => u.name.to_string(),
            TypeDef::Newtype(n) => n.name.to_string(),
            TypeDef::Config(c) => c.name.to_string(),
            _ => "Unknown".to_string(),
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Convert snake_case to PascalCase for Go exported names.
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect()
}
```

- [ ] **Step 5: Create lib.rs and backend.rs stubs**

Follow the `vexil-codegen-ts` pattern exactly. `lib.rs` declares modules, exports `GoBackend`, and has a `generate()` function. `backend.rs` implements `CodegenBackend`.

The `generate()` function emits:
1. `// Code generated by vexilc. DO NOT EDIT.`
2. `package {last_namespace_segment}`
3. `import vexil "github.com/vexil-lang/vexil/packages/runtime-go"`
4. `var SchemaHash = [32]byte{...}`
5. Each type declaration

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p vexil-codegen-go`

- [ ] **Step 7: Commit**

```bash
git add crates/vexil-codegen-go/ Cargo.toml
git commit -m "feat(vexil-codegen-go): scaffold Go codegen crate with emit, types, and backend"
```

---

## Task 5: Go Codegen — Message + Config

**Files:**
- Create: `crates/vexil-codegen-go/src/message.rs`

The largest codegen module. Generates Go structs with `Pack`/`Unpack` methods. Read `crates/vexil-codegen-ts/src/message.rs` for the full pattern.

Key differences from TypeScript:
- Go uses `(value, error)` return pattern for reads
- Field names are PascalCase
- `Unpack` fills receiver in-place: `func (m *Foo) Unpack(r *vexil.BitReader) error`
- Error handling: `if err != nil { return err }` after every read
- `Unknown []byte` field instead of `_unknown: Uint8Array`
- Typed tombstones: `_, err = r.ReadU32(); if err != nil { return err }`
- Sorted decode actions (fields + typed tombstones by ordinal)

The implementer should:
1. Read `crates/vexil-codegen-ts/src/message.rs` completely
2. Implement the Go equivalent with `emit_write`, `emit_read`, `emit_write_type`, `emit_read_type`, `emit_message`, `emit_config`, `emit_tombstone_read`
3. Use `to_pascal_case()` for field names

- [ ] **Step 1: Implement message.rs**

See spec Section 3 for the exact generated code shape. The message codegen must handle all `ResolvedType` variants and all `Encoding` variants.

- [ ] **Step 2: Wire into lib.rs**

Add `pub mod message;` and call `message::emit_message()` for `TypeDef::Message` and `message::emit_config()` for `TypeDef::Config`.

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p vexil-codegen-go`

- [ ] **Step 4: Commit**

```bash
git add crates/vexil-codegen-go/src/message.rs crates/vexil-codegen-go/src/lib.rs
git commit -m "feat(vexil-codegen-go): message and config code generation"
```

---

## Task 6: Go Codegen — Enum, Flags, Union, Newtype

**Files:**
- Create: `crates/vexil-codegen-go/src/enum_gen.rs`
- Create: `crates/vexil-codegen-go/src/flags.rs`
- Create: `crates/vexil-codegen-go/src/union_gen.rs`
- Create: `crates/vexil-codegen-go/src/newtype.rs`

### Enum

```go
type Status int
const (
	StatusOnline  Status = 0
	StatusOffline Status = 1
)
func (s Status) Pack(w *vexil.BitWriter) error { ... }
func (s *Status) Unpack(r *vexil.BitReader) error { ... }
```

Enum values prefixed with type name. `Pack` writes `wire_bits` bits. `Unpack` reads `wire_bits` bits. Non-exhaustive enums don't error on unknown values.

### Flags

```go
type Permissions uint32
const (
	PermissionsRead    Permissions = 1
	PermissionsWrite   Permissions = 2
	PermissionsExecute Permissions = 4
)
func (f Permissions) Pack(w *vexil.BitWriter) error { ... }
func (f *Permissions) Unpack(r *vexil.BitReader) error { ... }
```

### Union

Go doesn't have sum types. Use interface with marker method:

```go
type Shape interface { isShape() }
type ShapeCircle struct { Radius float32; Unknown []byte }
func (ShapeCircle) isShape() {}
type ShapeRect struct { W float32; H float32; Unknown []byte }
func (ShapeRect) isShape() {}
```

Encode dispatches by type assertion. Decode switches on discriminant. Wire format: LEB128 discriminant + LEB128 payload length + payload bytes. Each variant struct has `Unknown []byte`.

### Newtype

```go
type UserId string
```

Plus `Pack`/`Unpack` functions that delegate to the inner type.

- [ ] **Step 1: Implement all four modules**

Read the TypeScript equivalents for reference. Wire each into `lib.rs`.

- [ ] **Step 2: Run cargo check**

Run: `cargo check -p vexil-codegen-go`

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-codegen-go/src/
git commit -m "feat(vexil-codegen-go): enum, flags, union, newtype code generation"
```

---

## Task 7: Go Codegen — Delta

**Files:**
- Create: `crates/vexil-codegen-go/src/delta.rs`

Same pattern as Rust and TypeScript delta codegen. Read `crates/vexil-codegen-ts/src/delta.rs` for reference.

Generates `{Name}Encoder` and `{Name}Decoder` structs with unexported `prev*` fields:

```go
type ReadingEncoder struct {
	prevDeviceID uint32
	prevTemp     float32
}

func NewReadingEncoder() *ReadingEncoder {
	return &ReadingEncoder{}
}

func (e *ReadingEncoder) Pack(val *Reading, w *vexil.BitWriter) error {
	delta := val.DeviceID - e.prevDeviceID
	w.WriteLeb128(uint64(delta))
	e.prevDeviceID = val.DeviceID
	// ...
	return nil
}

func (e *ReadingEncoder) Reset() {
	e.prevDeviceID = 0
	e.prevTemp = 0.0
}
```

Go unsigned subtraction wraps naturally. Message-level `@delta` desugars to `Delta(Varint)` / `Delta(ZigZag)`.

- [ ] **Step 1: Implement delta.rs**

- [ ] **Step 2: Wire into lib.rs** (call after `emit_message`)

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-codegen-go/src/delta.rs crates/vexil-codegen-go/src/lib.rs
git commit -m "feat(vexil-codegen-go): delta encoder/decoder generation"
```

---

## Task 8: Golden Tests

**Files:**
- Create: `crates/vexil-codegen-go/tests/golden.rs`
- Create: `crates/vexil-codegen-go/tests/golden/` (directory)

Follow the exact pattern from `crates/vexil-codegen-ts/tests/golden.rs`. Replace `.ts` extension with `.go`.

Test schemas: 006_message, 007_enum, 008_flags, 009_union, 010_newtype, 011_config, 016_recursive, 027_delta_on_message, 028_typed_tombstone.

- [ ] **Step 1: Create golden.rs**

- [ ] **Step 2: Generate golden files**

Run: `UPDATE_GOLDEN=1 cargo test -p vexil-codegen-go`

- [ ] **Step 3: Review golden files**

Read a few generated `.go` golden files and verify they look like valid Go.

- [ ] **Step 4: Run tests without UPDATE_GOLDEN**

Run: `cargo test -p vexil-codegen-go`

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-codegen-go/tests/
git commit -m "test(vexil-codegen-go): golden output tests"
```

---

## Task 9: CLI Integration

**Files:**
- Modify: `crates/vexilc/src/main.rs`
- Modify: `crates/vexilc/Cargo.toml`

- [ ] **Step 1: Add dependency**

In `crates/vexilc/Cargo.toml`:
```toml
vexil-codegen-go = { path = "../vexil-codegen-go", version = "^0.2.5" }
```

- [ ] **Step 2: Add target dispatch**

In both target match blocks in `main.rs`, add:
```rust
"go" => Box::new(vexil_codegen_go::GoBackend),
```

Update error messages to include `go` in the available targets list.

- [ ] **Step 3: Smoke test**

```bash
cargo run -p vexilc -- codegen corpus/valid/006_message.vexil --target go
```

- [ ] **Step 4: Commit**

```bash
git add crates/vexilc/
git commit -m "feat(vexilc): add --target go to build and codegen commands"
```

---

## Task 10: CI + Release Config

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `release-plz.toml`

- [ ] **Step 1: Add Go runtime CI job**

In `.github/workflows/ci.yml`, add after the `ts-runtime` job:

```yaml
  go-runtime:
    name: Go runtime
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: packages/runtime-go
    steps:
      - uses: actions/checkout@v6
      - uses: actions/setup-go@v5
        with:
          go-version: "1.22"
      - run: go test -v ./...
```

- [ ] **Step 2: Add release-plz config**

In `release-plz.toml`, add:

```toml
[[package]]
name = "vexil-codegen-go"
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml release-plz.toml
git commit -m "ci: add Go runtime test job and release-plz config for vexil-codegen-go"
```

---

## Task 11: Final Verification

- [ ] **Step 1: Run Rust tests**

Run: `cargo test --workspace`

- [ ] **Step 2: Run Go tests**

Run: `cd packages/runtime-go && go test -v ./...`

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p vexil-codegen-go -p vexilc -- -D warnings`

- [ ] **Step 4: Run fmt**

Run: `cargo fmt --all -- --check`

- [ ] **Step 5: Smoke test CLI**

```bash
cargo run -p vexilc -- codegen corpus/valid/006_message.vexil --target go
cargo run -p vexilc -- codegen corpus/valid/009_union.vexil --target go
cargo run -p vexilc -- codegen corpus/valid/027_delta_on_message.vexil --target go
```
