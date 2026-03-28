# Go Backend Design

> **Scope:** Go code generation backend (`vexil-codegen-go`), Go runtime module (`packages/runtime-go/`), CLI integration (`--target go`), compliance testing. Feature-complete parity with Rust and TypeScript backends including delta encoding, unknown field preservation, typed tombstone support, and SchemaHandshake.

**Goal:** Ship a third language backend for Vexil, enabling Go services to encode and decode Vexil bitpack wire format with byte-identical output verified by the same compliance vectors as Rust and TypeScript.

**Architecture:** Rust crate emitting Go source strings (same approach as `vexil-codegen-ts`). Hand-written Go runtime module with zero dependencies. Generated code depends on runtime via standard Go import.

**Tech Stack:** Rust (codegen crate), Go 1.22+ (runtime module + generated code).

**Depends on:** SDK Architecture (`CodegenBackend` trait), compliance vectors, delta encoding spec.

---

## 1. Crate and Module Structure

```
crates/vexil-codegen-go/              Rust crate, implements CodegenBackend
  Cargo.toml
  src/
    lib.rs                            generate(), generate_project(), GoBackend
    backend.rs                        CodegenBackend impl, project generation
    emit.go                           CodeWriter utility (Go indent = tab)
    types.rs                          Vexil→Go type mapping
    message.rs                        Message/Config codegen (struct + Pack/Unpack)
    enum_gen.rs                       Enum codegen (int + iota constants)
    flags.rs                          Flags codegen (uint + bit constants)
    union_gen.rs                      Union codegen (interface + variant structs)
    newtype.rs                        Newtype codegen (type alias)
    delta.rs                          Delta encoder/decoder struct generation
  tests/
    golden.rs                         Golden output tests
    golden/                           .go golden files

packages/runtime-go/                  Go module: github.com/vexil-lang/vexil/packages/runtime-go
  go.mod
  bitreader.go                        BitReader struct
  bitreader_test.go
  bitwriter.go                        BitWriter struct
  bitwriter_test.go
  errors.go                           EncodeError, DecodeError
  interfaces.go                       Packer, Unpacker interfaces
  leb128.go                           LEB128 encode/decode
  zigzag.go                           ZigZag encode/decode
  handshake.go                        SchemaHandshake
  handshake_test.go
  compliance_test.go                  Golden vector compliance tests
  delta_compliance_test.go            Delta vector compliance tests
```

---

## 2. Type Mapping

| Vexil Type | Go Type |
|---|---|
| `bool` | `bool` |
| `u8`, `u16`, `u32` | `uint8`, `uint16`, `uint32` |
| `u64` | `uint64` |
| `i8`, `i16`, `i32` | `int8`, `int16`, `int32` |
| `i64` | `int64` |
| `f32`, `f64` | `float32`, `float64` |
| `string` | `string` |
| `bytes` | `[]byte` |
| `uuid` | `[16]byte` |
| `timestamp` | `int64` |
| `rgb` | `[3]uint8` |
| `hash` | `[32]byte` |
| `SubByte(bits:N)` | `uint8` |
| `Optional<T>` | `*T` (pointer = nullable) |
| `Array<T>` | `[]T` |
| `Map<K,V>` | `map[K]V` |
| `Result<T,E>` | struct with `Ok *T; Err *E` (exactly one non-nil) |
| Enum | `type Status int` + `const ( StatusOnline Status = 0 ... )` |
| Flags | `type Perms uint` + `const ( PermsRead Perms = 1 ... )` |
| Union | Interface with marker method + per-variant structs |
| Newtype | `type UserId string` |
| Config | Struct (no codec) |

---

## 3. Generated Code Shape

### Messages

```go
package telemetry

import vexil "github.com/vexil-lang/vexil/packages/runtime-go"

var SchemaHash = [32]byte{0xab, 0xcd, ...}
const SchemaVersion = "1.0.0"

type Reading struct {
	DeviceID uint32
	Temp     float32
	Status   Status
	Label    *string  // optional
	Unknown  []byte   // trailing bytes from newer versions
}

func (m *Reading) Pack(w *vexil.BitWriter) error {
	w.WriteU32(m.DeviceID)
	w.WriteF32(m.Temp)
	if err := w.EnterRecursive(); err != nil { return err }
	if err := m.Status.Pack(w); err != nil { return err }
	w.LeaveRecursive()
	if m.Label != nil {
		w.WriteBool(true)
		w.FlushToByteBoundary()
		w.WriteString(*m.Label)
	} else {
		w.WriteBool(false)
	}
	w.FlushToByteBoundary()
	if len(m.Unknown) > 0 {
		w.WriteRawBytes(m.Unknown)
	}
	return nil
}

func (m *Reading) Unpack(r *vexil.BitReader) error {
	var err error
	if m.DeviceID, err = r.ReadU32(); err != nil { return err }
	if m.Temp, err = r.ReadF32(); err != nil { return err }
	if err = r.EnterRecursive(); err != nil { return err }
	if err = m.Status.Unpack(r); err != nil { return err }
	r.LeaveRecursive()
	present, err := r.ReadBool()
	if err != nil { return err }
	if present {
		r.FlushToByteBoundary()
		s, err := r.ReadString()
		if err != nil { return err }
		m.Label = &s
	}
	r.FlushToByteBoundary()
	m.Unknown = r.ReadRemaining()
	return nil
}
```

### Enums

```go
type Status int

const (
	StatusOnline  Status = 0
	StatusOffline Status = 1
)

func (s Status) Pack(w *vexil.BitWriter) error {
	w.WriteBits(uint64(s), 1)
	return nil
}

func (s *Status) Unpack(r *vexil.BitReader) error {
	v, err := r.ReadBits(1)
	if err != nil { return err }
	*s = Status(v)
	return nil
}
```

### Flags

```go
type Permissions uint32

const (
	PermissionsRead    Permissions = 1
	PermissionsWrite   Permissions = 2
	PermissionsExecute Permissions = 4
)

func (f Permissions) Pack(w *vexil.BitWriter) error {
	w.WriteU32(uint32(f))
	return nil
}

func (f *Permissions) Unpack(r *vexil.BitReader) error {
	v, err := r.ReadU32()
	if err != nil { return err }
	*f = Permissions(v)
	return nil
}
```

### Unions

```go
type Shape interface {
	isShape()
}

type ShapeCircle struct {
	Radius  float32
	Unknown []byte
}
func (ShapeCircle) isShape() {}

type ShapeRect struct {
	W       float32
	H       float32
	Unknown []byte
}
func (ShapeRect) isShape() {}

// Pack/Unpack dispatch by type assertion for encode,
// discriminant switch for decode
```

### Newtypes

```go
type UserId string
```

### Configs

```go
type MyConfig struct {
	Timeout uint32
	Name    string
}
// No Pack/Unpack — config types have no wire format
```

---

## 4. Go Runtime API

### BitWriter

```go
type BitWriter struct { /* internal */ }

func NewBitWriter() *BitWriter
func (w *BitWriter) WriteBool(v bool)
func (w *BitWriter) WriteBits(v uint64, count uint8)
func (w *BitWriter) WriteU8(v uint8)
func (w *BitWriter) WriteU16(v uint16)
func (w *BitWriter) WriteU32(v uint32)
func (w *BitWriter) WriteU64(v uint64)
func (w *BitWriter) WriteI8(v int8)
func (w *BitWriter) WriteI16(v int16)
func (w *BitWriter) WriteI32(v int32)
func (w *BitWriter) WriteI64(v int64)
func (w *BitWriter) WriteF32(v float32)
func (w *BitWriter) WriteF64(v float64)
func (w *BitWriter) WriteLeb128(v uint64)
func (w *BitWriter) WriteZigZag(v int64, typeBits uint8)
func (w *BitWriter) WriteString(s string)
func (w *BitWriter) WriteBytes(data []byte)
func (w *BitWriter) WriteRawBytes(data []byte)
func (w *BitWriter) FlushToByteBoundary()
func (w *BitWriter) EnterRecursive() error
func (w *BitWriter) LeaveRecursive()
func (w *BitWriter) Finish() []byte
```

### BitReader

```go
type BitReader struct { /* internal */ }

func NewBitReader(data []byte) *BitReader
func (r *BitReader) ReadBool() (bool, error)
func (r *BitReader) ReadBits(count uint8) (uint64, error)
func (r *BitReader) ReadU8() (uint8, error)
// ... mirrors BitWriter
func (r *BitReader) ReadString() (string, error)
func (r *BitReader) ReadBytes() ([]byte, error)
func (r *BitReader) ReadRawBytes(n int) ([]byte, error)
func (r *BitReader) ReadRemaining() []byte
func (r *BitReader) FlushToByteBoundary()
func (r *BitReader) EnterRecursive() error
func (r *BitReader) LeaveRecursive()
```

### Interfaces

```go
type Packer interface {
	Pack(w *BitWriter) error
}

type Unpacker interface {
	Unpack(r *BitReader) error
}
```

### SchemaHandshake

```go
type SchemaHandshake struct {
	Hash    [32]byte
	Version string
}

type HandshakeResult struct {
	Match          bool
	LocalVersion   string
	RemoteVersion  string
	LocalHash      [32]byte
	RemoteHash     [32]byte
}

func NewSchemaHandshake(hash [32]byte, version string) *SchemaHandshake
func (h *SchemaHandshake) Encode() []byte
func DecodeSchemaHandshake(data []byte) (*SchemaHandshake, error)
func (h *SchemaHandshake) Check(remote *SchemaHandshake) HandshakeResult
```

---

## 5. Delta Encoding

For messages with `@delta` fields, the codegen emits stateful encoder/decoder structs:

```go
type ReadingEncoder struct {
	prevDeviceID uint32
	prevTemp     float32
}

func NewReadingEncoder() *ReadingEncoder {
	return &ReadingEncoder{}
}

func (e *ReadingEncoder) Pack(val *Reading, w *vexil.BitWriter) error {
	// delta fields: subtract prev, write delta, update prev
	delta := val.DeviceID - e.prevDeviceID
	w.WriteLeb128(uint64(delta))
	e.prevDeviceID = val.DeviceID
	// non-delta: write normally
	w.WriteF32(val.Temp)
	// ...
	return nil
}

func (e *ReadingEncoder) Reset() {
	e.prevDeviceID = 0
	e.prevTemp = 0.0
}
```

Go unsigned integer subtraction wraps naturally. Message-level `@delta`
desugars to `Delta(Varint)` for unsigned and `Delta(ZigZag)` for signed,
same as Rust and TypeScript.

---

## 6. Go Naming Conventions

Schema names map to Go conventions:

| Schema | Go |
|--------|-----|
| `device_id` (field) | `DeviceID` (exported PascalCase) |
| `Status` (enum) | `Status` (type), `StatusOnline` (value = type prefix + variant) |
| `ReadWrite` (flags bit) | `PermissionsReadWrite` (type prefix + bit name) |
| `Circle` (union variant) | `ShapeCircle` (type prefix + variant name) |
| `_unknown` (internal) | `Unknown` (exported, `[]byte`) |

The codegen converts `snake_case` → `PascalCase` for struct fields and
prefixes enum/flags values with the type name to avoid Go's package-level
name collisions.

---

## 7. Cross-File Imports

For `generate_project()`, namespace maps to Go packages:

```
sensor.telemetry → sensor/telemetry/telemetry.go (package telemetry)
sensor.config    → sensor/config/config.go (package config)
```

Cross-file import: `import "github.com/vexil-lang/vexil/packages/runtime-go"` for
the runtime, and relative paths for sibling packages within the generated output
(the user provides a base import path via CLI flag or convention).

---

## 8. CLI Integration

`vexilc build/codegen --target go`:

```
vexilc codegen schema.vexil --target go
vexilc build root.vexil --include ./schemas --output ./generated --target go
```

Dispatch in `cmd_build` / `cmd_codegen`:
```rust
"go" => Box::new(vexil_codegen_go::GoBackend),
```

---

## 9. Testing

**Golden tests:** `crates/vexil-codegen-go/tests/golden/` with `.go` golden
files for corpus schemas 006-011, 016, 027, 028. Same `UPDATE_GOLDEN=1` pattern.

**Compliance vectors:** `packages/runtime-go/compliance_test.go` reads
`compliance/vectors/*.json`, encodes with BitWriter, asserts bytes match.
All three implementations (Rust, TypeScript, Go) validate against the same vectors.

**Delta compliance:** `packages/runtime-go/delta_compliance_test.go` reads
`compliance/vectors/delta.json`, runs multi-frame stateful encode/decode.

**CI:** New `go-runtime` job in `.github/workflows/ci.yml`:
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
    - run: go test ./...
```

---

## 10. Decision Log

### Runtime location: in-repo vs separate repo

**Chosen:** `packages/runtime-go/` inside vexil monorepo.

**Rejected:** Separate `vexil-go` repository.

**Rationale:** Same pattern as TypeScript runtime. Keeps development and CI
in one repo. Can extract later if the Go community expects a standalone module.

### Unpack style: return value vs fill receiver

**Chosen:** `Unpack(r *BitReader) error` — fills receiver in-place.

**Rejected:** `func UnpackFoo(r *BitReader) (*Foo, error)` — returns new struct.

**Rationale:** Matches `json.Unmarshal`, `proto.Unmarshal`, and standard Go
patterns. Avoids allocations. Enables reuse of allocated structs in hot loops.

### Result type: struct with pointers vs tuple

**Chosen:** `struct { Ok *T; Err *E }` — exactly one non-nil.

**Rejected:** `(T, E, bool)` tuple pattern.

**Rationale:** Closer to wire format. Self-documenting. Consistent with how
other Go libraries represent sum types when the language lacks them.

### Enum representation: int + iota vs string

**Chosen:** `type Status int` with `iota` constants.

**Rejected:** String constants (like TypeScript).

**Rationale:** Wire format is integer-based. Int enums are idiomatic Go and
enable switch statements with exhaustiveness linters. String representation
loses the ordinal mapping.
