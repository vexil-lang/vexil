package vexil

import (
	"errors"
	"math"
	"testing"
)

func TestReadSingleBit(t *testing.T) {
	r := NewBitReader([]byte{0x01})
	v, err := r.ReadBool()
	assertNoError(t, err)
	if !v {
		t.Fatal("expected true")
	}
}

func TestRoundTripSubByte(t *testing.T) {
	w := NewBitWriter()
	w.WriteBits(5, 3)
	w.WriteBits(19, 5)
	w.WriteBits(42, 6)
	buf := w.Finish()
	r := NewBitReader(buf)

	v, err := r.ReadBits(3)
	assertNoError(t, err)
	assertUint64(t, v, 5)

	v, err = r.ReadBits(5)
	assertNoError(t, err)
	assertUint64(t, v, 19)

	v, err = r.ReadBits(6)
	assertNoError(t, err)
	assertUint64(t, v, 42)
}

func TestRoundTripU8(t *testing.T) {
	w := NewBitWriter()
	w.WriteU8(0xAB)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadU8()
	assertNoError(t, err)
	if v != 0xAB {
		t.Fatalf("got %02X, want AB", v)
	}
}

func TestRoundTripU16(t *testing.T) {
	w := NewBitWriter()
	w.WriteU16(0x1234)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadU16()
	assertNoError(t, err)
	if v != 0x1234 {
		t.Fatalf("got %04X, want 1234", v)
	}
}

func TestRoundTripU32(t *testing.T) {
	w := NewBitWriter()
	w.WriteU32(0x12345678)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadU32()
	assertNoError(t, err)
	if v != 0x12345678 {
		t.Fatalf("got %08X, want 12345678", v)
	}
}

func TestRoundTripU64(t *testing.T) {
	w := NewBitWriter()
	w.WriteU64(0x0102030405060708)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadU64()
	assertNoError(t, err)
	if v != 0x0102030405060708 {
		t.Fatalf("got %016X, want 0102030405060708", v)
	}
}

func TestRoundTripI32Neg(t *testing.T) {
	w := NewBitWriter()
	w.WriteI32(-42)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadI32()
	assertNoError(t, err)
	if v != -42 {
		t.Fatalf("got %d, want -42", v)
	}
}

func TestRoundTripI64(t *testing.T) {
	w := NewBitWriter()
	w.WriteI64(-1000)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadI64()
	assertNoError(t, err)
	if v != -1000 {
		t.Fatalf("got %d, want -1000", v)
	}
}

func TestRoundTripF32(t *testing.T) {
	w := NewBitWriter()
	w.WriteF32(math.Pi)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadF32()
	assertNoError(t, err)
	if v != float32(math.Pi) {
		t.Fatalf("got %v, want %v", v, float32(math.Pi))
	}
}

func TestRoundTripF64Nan(t *testing.T) {
	w := NewBitWriter()
	w.WriteF64(math.NaN())
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadF64()
	assertNoError(t, err)
	if !math.IsNaN(v) {
		t.Fatal("expected NaN")
	}
	if math.Float64bits(v) != 0x7FF8000000000000 {
		t.Fatalf("got %016X, want canonical NaN", math.Float64bits(v))
	}
}

func TestRoundTripString(t *testing.T) {
	w := NewBitWriter()
	w.WriteString("hello")
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadString()
	assertNoError(t, err)
	if v != "hello" {
		t.Fatalf("got %q, want %q", v, "hello")
	}
}

func TestRoundTripStringEmpty(t *testing.T) {
	w := NewBitWriter()
	w.WriteString("")
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadString()
	assertNoError(t, err)
	if v != "" {
		t.Fatalf("got %q, want empty", v)
	}
}

func TestRoundTripLeb128(t *testing.T) {
	w := NewBitWriter()
	w.WriteLeb128(300)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadLeb128(4)
	assertNoError(t, err)
	assertUint64(t, v, 300)
}

func TestRoundTripZigZag(t *testing.T) {
	w := NewBitWriter()
	w.WriteZigZag(-42, 64)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadZigZag(64, 10)
	assertNoError(t, err)
	if v != -42 {
		t.Fatalf("got %d, want -42", v)
	}
}

func TestRoundTripBytes(t *testing.T) {
	w := NewBitWriter()
	w.WriteBytes([]byte{0xDE, 0xAD})
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadBytes()
	assertNoError(t, err)
	assertEqual(t, v, []byte{0xDE, 0xAD})
}

func TestRoundTripRawBytes(t *testing.T) {
	w := NewBitWriter()
	w.WriteRawBytes([]byte{0xCA, 0xFE})
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadRawBytes(2)
	assertNoError(t, err)
	assertEqual(t, v, []byte{0xCA, 0xFE})
}

func TestReadUnexpectedEOF(t *testing.T) {
	r := NewBitReader([]byte{})
	_, err := r.ReadU8()
	if !errors.Is(err, ErrUnexpectedEOF) {
		t.Fatalf("expected ErrUnexpectedEOF, got %v", err)
	}
}

func TestReadInvalidUTF8(t *testing.T) {
	w := NewBitWriter()
	w.WriteLeb128(2)
	w.WriteRawBytes([]byte{0xFF, 0xFE})
	buf := w.Finish()
	r := NewBitReader(buf)
	_, err := r.ReadString()
	if !errors.Is(err, ErrInvalidUTF8) {
		t.Fatalf("expected ErrInvalidUTF8, got %v", err)
	}
}

func TestReaderRecursionDepthLimit(t *testing.T) {
	r := NewBitReader([]byte{})
	for i := 0; i < 64; i++ {
		if err := r.EnterRecursive(); err != nil {
			t.Fatalf("depth %d: %v", i+1, err)
		}
	}
	if err := r.EnterRecursive(); err == nil {
		t.Fatal("expected error at depth 65")
	}
}

func TestReaderRecursionDepthLeave(t *testing.T) {
	r := NewBitReader([]byte{})
	for i := 0; i < 64; i++ {
		assertNoError(t, r.EnterRecursive())
	}
	r.LeaveRecursive()
	assertNoError(t, r.EnterRecursive())
}

func TestTrailingBytesNotRejected(t *testing.T) {
	data := []byte{0x2a, 0x00, 0x00, 0x00, 0x63, 0x00}
	r := NewBitReader(data)
	v, err := r.ReadU32()
	assertNoError(t, err)
	if v != 42 {
		t.Fatalf("got %d, want 42", v)
	}
	r.FlushToByteBoundary()
	// Remaining bytes must not cause error
}

func TestReadRemainingAfterPartialDecode(t *testing.T) {
	data := []byte{0x2a, 0x00, 0x00, 0x00, 0x63, 0x00}
	r := NewBitReader(data)
	_, _ = r.ReadU32()
	remaining := r.ReadRemaining()
	assertEqual(t, remaining, []byte{0x63, 0x00})
}

func TestReadRemainingWhenFullyConsumed(t *testing.T) {
	data := []byte{0x2a, 0x00, 0x00, 0x00}
	r := NewBitReader(data)
	_, _ = r.ReadU32()
	remaining := r.ReadRemaining()
	if len(remaining) != 0 {
		t.Fatalf("expected empty, got %X", remaining)
	}
}

func TestReadRemainingFromStart(t *testing.T) {
	data := []byte{0x01, 0x02, 0x03}
	r := NewBitReader(data)
	remaining := r.ReadRemaining()
	assertEqual(t, remaining, []byte{0x01, 0x02, 0x03})
}

func TestReaderFlushToByteBoundary(t *testing.T) {
	w := NewBitWriter()
	w.WriteBits(0b101, 3)
	w.FlushToByteBoundary()
	w.WriteU8(0xAB)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadBits(3)
	assertNoError(t, err)
	assertUint64(t, v, 0b101)
	r.FlushToByteBoundary()
	u, err := r.ReadU8()
	assertNoError(t, err)
	if u != 0xAB {
		t.Fatalf("got %02X, want AB", u)
	}
}

func TestReadBitsFromMultipleBytes(t *testing.T) {
	// Write 3+5+6 = 14 bits across 2 bytes
	w := NewBitWriter()
	w.WriteBits(7, 3)  // 111
	w.WriteBits(31, 5) // 11111
	w.WriteBits(63, 6) // 111111
	buf := w.Finish()

	r := NewBitReader(buf)
	a, err := r.ReadBits(3)
	assertNoError(t, err)
	assertUint64(t, a, 7)

	b, err := r.ReadBits(5)
	assertNoError(t, err)
	assertUint64(t, b, 31)

	c, err := r.ReadBits(6)
	assertNoError(t, err)
	assertUint64(t, c, 63)
}

func TestReadI8(t *testing.T) {
	w := NewBitWriter()
	w.WriteI8(-1)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadI8()
	assertNoError(t, err)
	if v != -1 {
		t.Fatalf("got %d, want -1", v)
	}
}

func TestReadI16(t *testing.T) {
	w := NewBitWriter()
	w.WriteI16(-1000)
	buf := w.Finish()
	r := NewBitReader(buf)
	v, err := r.ReadI16()
	assertNoError(t, err)
	if v != -1000 {
		t.Fatalf("got %d, want -1000", v)
	}
}

// helper functions

func assertNoError(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

func assertUint64(t *testing.T, got, want uint64) {
	t.Helper()
	if got != want {
		t.Fatalf("got %d, want %d", got, want)
	}
}
