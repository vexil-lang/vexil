package bitio

import (
	"bytes"
	"math"
	"testing"
)

func TestWriterReader(t *testing.T) {
	// Test basic bitio operations
	var buf bytes.Buffer
	w := NewWriter(&buf)

	// Write some data
	if err := w.WriteU8(0x42); err != nil {
		t.Fatalf("WriteU8 failed: %v", err)
	}
	if err := w.WriteU16(0x1234); err != nil {
		t.Fatalf("WriteU16 failed: %v", err)
	}
	if err := w.WriteU32(0xDEADBEEF); err != nil {
		t.Fatalf("WriteU32 failed: %v", err)
	}
	if err := w.WriteU64(0x0123456789ABCDEF); err != nil {
		t.Fatalf("WriteU64 failed: %v", err)
	}
	if err := w.WriteBool(true); err != nil {
		t.Fatalf("WriteBool(true) failed: %v", err)
	}
	if err := w.WriteBool(false); err != nil {
		t.Fatalf("WriteBool(false) failed: %v", err)
	}
	if err := w.WriteF32(3.14159); err != nil {
		t.Fatalf("WriteF32 failed: %v", err)
	}
	if err := w.WriteF64(2.718281828); err != nil {
		t.Fatalf("WriteF64 failed: %v", err)
	}
	if err := w.Flush(); err != nil {
		t.Fatalf("Flush failed: %v", err)
	}

	// Read it back
	r := NewReader(&buf)

	v8, err := r.ReadU8()
	if err != nil {
		t.Fatalf("ReadU8 failed: %v", err)
	}
	if v8 != 0x42 {
		t.Errorf("ReadU8: got 0x%x, want 0x42", v8)
	}

	v16, err := r.ReadU16()
	if err != nil {
		t.Fatalf("ReadU16 failed: %v", err)
	}
	if v16 != 0x1234 {
		t.Errorf("ReadU16: got 0x%x, want 0x1234", v16)
	}

	v32, err := r.ReadU32()
	if err != nil {
		t.Fatalf("ReadU32 failed: %v", err)
	}
	if v32 != 0xDEADBEEF {
		t.Errorf("ReadU32: got 0x%x, want 0xDEADBEEF", v32)
	}

	v64, err := r.ReadU64()
	if err != nil {
		t.Fatalf("ReadU64 failed: %v", err)
	}
	if v64 != 0x0123456789ABCDEF {
		t.Errorf("ReadU64: got 0x%x, want 0x0123456789ABCDEF", v64)
	}

	b1, err := r.ReadBool()
	if err != nil {
		t.Fatalf("ReadBool failed: %v", err)
	}
	if !b1 {
		t.Errorf("ReadBool: got false, want true")
	}

	b2, err := r.ReadBool()
	if err != nil {
		t.Fatalf("ReadBool failed: %v", err)
	}
	if b2 {
		t.Errorf("ReadBool: got true, want false")
	}
}

func TestWriteReadF32(t *testing.T) {
	var buf bytes.Buffer
	w := NewWriter(&buf)
	
	testValue := float32(3.14159)
	if err := w.WriteF32(testValue); err != nil {
		t.Fatalf("WriteF32 failed: %v", err)
	}
	if err := w.Flush(); err != nil {
		t.Fatalf("Flush failed: %v", err)
	}

	r := NewReader(&buf)
	// Read as U32 and convert back to verify
	bits, err := r.ReadU32()
	if err != nil {
		t.Fatalf("ReadU32 failed: %v", err)
	}
	result := math.Float32frombits(bits)
	if result != testValue {
		t.Errorf("F32 roundtrip: got %v, want %v", result, testValue)
	}
}

func TestWriteReadF64(t *testing.T) {
	var buf bytes.Buffer
	w := NewWriter(&buf)
	
	testValue := float64(2.718281828459045)
	if err := w.WriteF64(testValue); err != nil {
		t.Fatalf("WriteF64 failed: %v", err)
	}
	if err := w.Flush(); err != nil {
		t.Fatalf("Flush failed: %v", err)
	}

	r := NewReader(&buf)
	// Read as U64 and convert back to verify
	bits, err := r.ReadU64()
	if err != nil {
		t.Fatalf("ReadU64 failed: %v", err)
	}
	result := math.Float64frombits(bits)
	if result != testValue {
		t.Errorf("F64 roundtrip: got %v, want %v", result, testValue)
	}
}
