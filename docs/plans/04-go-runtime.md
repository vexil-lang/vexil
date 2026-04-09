# Go Runtime Module Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this task.

**Goal:** Create `github.com/vexil-lang/vexil-runtime` Go module for Go codegen support.

**Architecture:** Go module with bitio, pack/unpack support, geometric types.

**Tech Stack:** Go, Go modules

---

## Current State

- Go codegen references `github.com/vexil-lang/vexil-runtime`
- No actual module exists at that path

## Target State

- `go get github.com/vexil-lang/vexil-runtime` works
- Module provides bitio, pack/unpack interfaces

---

## Task: Create Go Runtime Module

**Objective:** Set up Go module with runtime support.

**Files:**
- Create: `runtimes/go/go.mod`
- Create: `runtimes/go/bitio/bitio.go`
- Create: `runtimes/go/pack/pack.go`

**Step 1: Create directory structure**

```bash
mkdir -p runtimes/go/bitio
mkdir -p runtimes/go/pack
```

**Step 2: Create go.mod**

```go
module github.com/vexil-lang/vexil-runtime

go 1.21
```

**Step 3: Create bitio/bitio.go**

```go
// Package bitio provides bit-level I/O for Vexil binary format.
package bitio

import (
	"encoding/binary"
	"errors"
	"io"
)

// Writer writes bits and bytes to an underlying writer.
type Writer struct {
	w       io.Writer
	buf     byte
	bitPos  uint8
}

// NewWriter creates a new bit writer.
func NewWriter(w io.Writer) *Writer {
	return &Writer{w: w}
}

// WriteU8 writes an unsigned 8-bit value.
func (w *Writer) WriteU8(v uint8) error {
	w.alignToByte()
	_, err := w.w.Write([]byte{v})
	return err
}

// WriteU16 writes an unsigned 16-bit value (little-endian).
func (w *Writer) WriteU16(v uint16) error {
	w.alignToByte()
	var buf [2]byte
	binary.LittleEndian.PutUint16(buf[:], v)
	_, err := w.w.Write(buf[:])
	return err
}

// WriteU32 writes an unsigned 32-bit value (little-endian).
func (w *Writer) WriteU32(v uint32) error {
	w.alignToByte()
	var buf [4]byte
	binary.LittleEndian.PutUint32(buf[:], v)
	_, err := w.w.Write(buf[:])
	return err
}

// WriteU64 writes an unsigned 64-bit value (little-endian).
func (w *Writer) WriteU64(v uint64) error {
	w.alignToByte()
	var buf [8]byte
	binary.LittleEndian.PutUint64(buf[:], v)
	_, err := w.w.Write(buf[:])
	return err
}

// WriteF32 writes a 32-bit float (little-endian).
func (w *Writer) WriteF32(v float32) error {
	return w.WriteU32(math.Float32bits(v))
}

// WriteF64 writes a 64-bit float (little-endian).
func (w *Writer) WriteF64(v float64) error {
	return w.WriteU64(math.Float64bits(v))
}

// WriteBool writes a single bit.
func (w *Writer) WriteBool(v bool) error {
	if v {
		w.buf |= 1 << w.bitPos
	}
	w.bitPos++
	if w.bitPos == 8 {
		_, err := w.w.Write([]byte{w.buf})
		if err != nil {
			return err
		}
		w.buf = 0
		w.bitPos = 0
	}
	return nil
}

// Flush any remaining bits.
func (w *Writer) Flush() error {
	if w.bitPos > 0 {
		_, err := w.w.Write([]byte{w.buf})
		w.buf = 0
		w.bitPos = 0
		return err
	}
	return nil
}

func (w *Writer) alignToByte() {
	if w.bitPos > 0 {
		w.w.Write([]byte{w.buf})
		w.buf = 0
		w.bitPos = 0
	}
}

// Reader reads bits and bytes from an underlying reader.
type Reader struct {
	r       io.Reader
	buf     byte
	bitPos  uint8
	valid   bool
}

// NewReader creates a new bit reader.
func NewReader(r io.Reader) *Reader {
	return &Reader{r: r}
}

// ReadU8 reads an unsigned 8-bit value.
func (r *Reader) ReadU8() (uint8, error) {
	r.alignToByte()
	var buf [1]byte
	_, err := io.ReadFull(r.r, buf[:])
	return buf[0], err
}

// ReadU16 reads an unsigned 16-bit value (little-endian).
func (r *Reader) ReadU16() (uint16, error) {
	r.alignToByte()
	var buf [2]byte
	_, err := io.ReadFull(r.r, buf[:])
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint16(buf[:]), nil
}

// ReadU32 reads an unsigned 32-bit value (little-endian).
func (r *Reader) ReadU32() (uint32, error) {
	r.alignToByte()
	var buf [4]byte
	_, err := io.ReadFull(r.r, buf[:])
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint32(buf[:]), nil
}

// ReadU64 reads an unsigned 64-bit value (little-endian).
func (r *Reader) ReadU64() (uint64, error) {
	r.alignToByte()
	var buf [8]byte
	_, err := io.ReadFull(r.r, buf[:])
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint64(buf[:]), nil
}

// ReadBool reads a single bit.
func (r *Reader) ReadBool() (bool, error) {
	if !r.valid {
		_, err := io.ReadFull(r.r, []byte{r.buf})
		if err != nil {
			return false, err
		}
		r.valid = true
	}
	
	v := (r.buf >> r.bitPos) & 1
	r.bitPos++
	if r.bitPos == 8 {
		r.valid = false
		r.bitPos = 0
	}
	
	return v == 1, nil
}

func (r *Reader) alignToByte() {
	if r.bitPos > 0 {
		r.valid = false
		r.bitPos = 0
	}
}
```

**Step 4: Create pack/pack.go**

```go
// Package pack provides Pack/Unpack interfaces for Vexil types.
package pack

import "github.com/vexil-lang/vexil-runtime/bitio"

// Packer is the interface for types that can be packed.
type Packer interface {
	Pack(w *bitio.Writer) error
}

// Unpacker is the interface for types that can be unpacked.
type Unpacker interface {
	Unpack(r *bitio.Reader) error
}
```

**Step 5: Test locally**

```bash
cd runtimes/go
go mod tidy
go test ./...
```

**Step 6: Commit**

```bash
git add runtimes/go/
git commit -m "feat: add Go runtime module with bitio and pack support"
```

---

## Task 2: Publish Go Module

**Objective:** Push to GitHub so `go get` works.

**Step 1: Create git tag**

```bash
git tag -a runtimes/go/v0.1.0 -m "Go runtime v0.1.0"
git push origin runtimes/go/v0.1.0
```

**Step 2: Verify**

```bash
go get github.com/vexil-lang/vexil-runtime@latest
```

---

**Summary:** Create Go runtime module with bitio and Pack/Unpack interfaces, tag and publish.
