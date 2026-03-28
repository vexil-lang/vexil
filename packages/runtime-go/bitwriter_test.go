package vexil

import (
	"math"
	"testing"
)

func TestWriteSingleBitTrue(t *testing.T) {
	w := NewBitWriter()
	w.WriteBool(true)
	assertEqual(t, w.Finish(), []byte{0x01})
}

func TestWriteSingleBitFalse(t *testing.T) {
	w := NewBitWriter()
	w.WriteBool(false)
	assertEqual(t, w.Finish(), []byte{0x00})
}

func TestWriteBitsLSBFirst(t *testing.T) {
	w := NewBitWriter()
	w.WriteBits(5, 3)  // 101
	w.WriteBits(19, 5) // 10011
	// LSB-first: byte = 10011_101 = 0x9D
	assertEqual(t, w.Finish(), []byte{0x9D})
}

func TestWriteBitsCrossByteBoundary(t *testing.T) {
	w := NewBitWriter()
	w.WriteBits(5, 3)
	w.WriteBits(19, 5)
	w.WriteBits(42, 6) // 101010
	// Byte 0: 0x9D, Byte 1: 00_101010 = 0x2A
	assertEqual(t, w.Finish(), []byte{0x9D, 0x2A})
}

func TestFlushToByteBoundaryPadsZeros(t *testing.T) {
	w := NewBitWriter()
	w.WriteBits(0b101, 3)
	w.FlushToByteBoundary()
	w.WriteBits(0xFF, 8)
	assertEqual(t, w.Finish(), []byte{0x05, 0xFF})
}

func TestWriteU8FlushesFirst(t *testing.T) {
	w := NewBitWriter()
	w.WriteBool(true)
	w.WriteU8(0xAB)
	assertEqual(t, w.Finish(), []byte{0x01, 0xAB})
}

func TestWriteU16LE(t *testing.T) {
	w := NewBitWriter()
	w.WriteU16(0x0102)
	assertEqual(t, w.Finish(), []byte{0x02, 0x01})
}

func TestWriteU32LE(t *testing.T) {
	w := NewBitWriter()
	w.WriteU32(0x01020304)
	assertEqual(t, w.Finish(), []byte{0x04, 0x03, 0x02, 0x01})
}

func TestWriteU64LE(t *testing.T) {
	w := NewBitWriter()
	w.WriteU64(0x0102030405060708)
	assertEqual(t, w.Finish(), []byte{0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01})
}

func TestWriteI16Negative(t *testing.T) {
	w := NewBitWriter()
	w.WriteI16(-1)
	assertEqual(t, w.Finish(), []byte{0xFF, 0xFF})
}

func TestWriteI32Negative(t *testing.T) {
	w := NewBitWriter()
	w.WriteI32(-1)
	assertEqual(t, w.Finish(), []byte{0xFF, 0xFF, 0xFF, 0xFF})
}

func TestWriteF32NanCanonicalized(t *testing.T) {
	w := NewBitWriter()
	w.WriteF32(float32(math.NaN()))
	assertEqual(t, w.Finish(), []byte{0x00, 0x00, 0xC0, 0x7F})
}

func TestWriteF64NanCanonicalized(t *testing.T) {
	w := NewBitWriter()
	w.WriteF64(math.NaN())
	// 0x7FF8000000000000 little-endian
	assertEqual(t, w.Finish(), []byte{0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x7F})
}

func TestWriteF32NegativeZeroPreserved(t *testing.T) {
	w := NewBitWriter()
	negZero := math.Float32frombits(0x80000000)
	w.WriteF32(negZero)
	buf := w.Finish()
	assertEqual(t, buf, []byte{0x00, 0x00, 0x00, 0x80})

	// Verify it differs from positive zero
	w2 := NewBitWriter()
	w2.WriteF32(0.0)
	buf2 := w2.Finish()
	if bytesEqual(buf, buf2) {
		t.Fatal("negative zero should differ from positive zero")
	}
}

func TestWriteF64NegativeZeroPreserved(t *testing.T) {
	w := NewBitWriter()
	negZero := math.Float64frombits(0x8000000000000000)
	w.WriteF64(negZero)
	assertEqual(t, w.Finish(), []byte{0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80})
}

func TestWriteLeb128(t *testing.T) {
	w := NewBitWriter()
	w.WriteLeb128(300)
	assertEqual(t, w.Finish(), []byte{0xAC, 0x02})
}

func TestWriteLeb128Zero(t *testing.T) {
	w := NewBitWriter()
	w.WriteLeb128(0)
	assertEqual(t, w.Finish(), []byte{0x00})
}

func TestWriteLeb128_127(t *testing.T) {
	w := NewBitWriter()
	w.WriteLeb128(127)
	assertEqual(t, w.Finish(), []byte{0x7F})
}

func TestWriteLeb128_128(t *testing.T) {
	w := NewBitWriter()
	w.WriteLeb128(128)
	assertEqual(t, w.Finish(), []byte{0x80, 0x01})
}

func TestWriteZigZagNeg1(t *testing.T) {
	w := NewBitWriter()
	w.WriteZigZag(-1, 64)
	assertEqual(t, w.Finish(), []byte{0x01})
}

func TestWriteZigZagPos1(t *testing.T) {
	w := NewBitWriter()
	w.WriteZigZag(1, 64)
	assertEqual(t, w.Finish(), []byte{0x02})
}

func TestWriteZigZagZero(t *testing.T) {
	w := NewBitWriter()
	w.WriteZigZag(0, 64)
	assertEqual(t, w.Finish(), []byte{0x00})
}

func TestWriteString(t *testing.T) {
	w := NewBitWriter()
	w.WriteString("hi")
	assertEqual(t, w.Finish(), []byte{0x02, 0x68, 0x69})
}

func TestWriteStringEmpty(t *testing.T) {
	w := NewBitWriter()
	w.WriteString("")
	assertEqual(t, w.Finish(), []byte{0x00})
}

func TestWriteBytes(t *testing.T) {
	w := NewBitWriter()
	w.WriteBytes([]byte{0xDE, 0xAD})
	assertEqual(t, w.Finish(), []byte{0x02, 0xDE, 0xAD})
}

func TestWriteRawBytes(t *testing.T) {
	w := NewBitWriter()
	w.WriteRawBytes([]byte{0xCA, 0xFE})
	assertEqual(t, w.Finish(), []byte{0xCA, 0xFE})
}

func TestEmptyFlushProducesZeroByte(t *testing.T) {
	w := NewBitWriter()
	w.FlushToByteBoundary()
	assertEqual(t, w.Finish(), []byte{0x00})
}

func TestEmptyFinishProducesZeroByte(t *testing.T) {
	w := NewBitWriter()
	assertEqual(t, w.Finish(), []byte{0x00})
}

func TestRecursionDepthIncrementDecrement(t *testing.T) {
	w := NewBitWriter()
	if err := w.EnterRecursive(); err != nil {
		t.Fatal(err)
	}
	if err := w.EnterRecursive(); err != nil {
		t.Fatal(err)
	}
	w.LeaveRecursive()
	w.LeaveRecursive()
}

func TestRecursionDepthMax64Succeeds(t *testing.T) {
	w := NewBitWriter()
	for i := 0; i < 64; i++ {
		if err := w.EnterRecursive(); err != nil {
			t.Fatalf("depth %d: %v", i+1, err)
		}
	}
}

func TestRecursionDepth65ExceedsLimit(t *testing.T) {
	w := NewBitWriter()
	for i := 0; i < 64; i++ {
		if err := w.EnterRecursive(); err != nil {
			t.Fatalf("depth %d: %v", i+1, err)
		}
	}
	if err := w.EnterRecursive(); err == nil {
		t.Fatal("expected error at depth 65")
	}
}

// Sub-byte compliance vectors
func TestSubByteU3U5Packed(t *testing.T) {
	w := NewBitWriter()
	w.WriteBits(5, 3)  // a=101
	w.WriteBits(18, 5) // b=10010
	assertEqual(t, w.Finish(), []byte{0x95})
}

func TestSubByteU3U5U6CrossByte(t *testing.T) {
	w := NewBitWriter()
	w.WriteBits(7, 3)  // a=111
	w.WriteBits(31, 5) // b=11111
	w.WriteBits(63, 6) // c=111111
	assertEqual(t, w.Finish(), []byte{0xFF, 0x3F})
}

func TestSubByteU1One(t *testing.T) {
	w := NewBitWriter()
	w.WriteBits(1, 1) // v=1
	assertEqual(t, w.Finish(), []byte{0x01})
}

// helper functions

func assertEqual(t *testing.T, got, want []byte) {
	t.Helper()
	if !bytesEqual(got, want) {
		t.Fatalf("got %X, want %X", got, want)
	}
}

func bytesEqual(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}
