package vexil

import (
	"math"
)

// BitWriter packs fields LSB-first at the bit level into a byte buffer.
//
// Sub-byte fields are accumulated in a single byte; once 8 bits are filled
// the byte is flushed. Multi-byte writes first align to a byte boundary,
// then append little-endian bytes directly.
type BitWriter struct {
	buf         []byte
	currentByte byte
	bitOffset   uint8
	depth       uint32
}

// NewBitWriter creates a new, empty BitWriter.
func NewBitWriter() *BitWriter {
	return &BitWriter{}
}

// align flushes partial bits to align to a byte boundary.
// Unlike FlushToByteBoundary, this does NOT emit a zero byte for empty buffers.
func (w *BitWriter) align() {
	if w.bitOffset > 0 {
		w.buf = append(w.buf, w.currentByte)
		w.currentByte = 0
		w.bitOffset = 0
	}
}

// WriteBool writes a single boolean as 1 bit.
func (w *BitWriter) WriteBool(v bool) {
	if v {
		w.WriteBits(1, 1)
	} else {
		w.WriteBits(0, 1)
	}
}

// WriteBits writes count bits from v, LSB first.
func (w *BitWriter) WriteBits(v uint64, count uint8) {
	val := v
	for i := uint8(0); i < count; i++ {
		bit := byte(val & 1)
		w.currentByte |= bit << w.bitOffset
		w.bitOffset++
		if w.bitOffset == 8 {
			w.buf = append(w.buf, w.currentByte)
			w.currentByte = 0
			w.bitOffset = 0
		}
		val >>= 1
	}
}

// FlushToByteBoundary flushes any partial byte to the buffer.
//
// Special case per spec section 4.1: if nothing has been written at all
// (bitOffset == 0 AND buf is empty), push a zero byte anyway.
// If bitOffset == 0 and buf is non-empty, this is a no-op.
func (w *BitWriter) FlushToByteBoundary() {
	if w.bitOffset == 0 {
		if len(w.buf) == 0 {
			w.buf = append(w.buf, 0x00)
		}
		// else: already aligned and something was written — no-op
	} else {
		w.buf = append(w.buf, w.currentByte)
		w.currentByte = 0
		w.bitOffset = 0
	}
}

// WriteU8 writes a uint8, aligning to a byte boundary first.
func (w *BitWriter) WriteU8(v uint8) {
	w.align()
	w.buf = append(w.buf, v)
}

// WriteU16 writes a uint16 in little-endian byte order, aligning first.
func (w *BitWriter) WriteU16(v uint16) {
	w.align()
	w.buf = append(w.buf, byte(v), byte(v>>8))
}

// WriteU32 writes a uint32 in little-endian byte order, aligning first.
func (w *BitWriter) WriteU32(v uint32) {
	w.align()
	w.buf = append(w.buf, byte(v), byte(v>>8), byte(v>>16), byte(v>>24))
}

// WriteU64 writes a uint64 in little-endian byte order, aligning first.
func (w *BitWriter) WriteU64(v uint64) {
	w.align()
	w.buf = append(w.buf,
		byte(v), byte(v>>8), byte(v>>16), byte(v>>24),
		byte(v>>32), byte(v>>40), byte(v>>48), byte(v>>56),
	)
}

// WriteI8 writes an int8, aligning to a byte boundary first.
func (w *BitWriter) WriteI8(v int8) {
	w.align()
	w.buf = append(w.buf, byte(v))
}

// WriteI16 writes an int16 in little-endian byte order, aligning first.
func (w *BitWriter) WriteI16(v int16) {
	w.WriteU16(uint16(v))
}

// WriteI32 writes an int32 in little-endian byte order, aligning first.
func (w *BitWriter) WriteI32(v int32) {
	w.WriteU32(uint32(v))
}

// WriteI64 writes an int64 in little-endian byte order, aligning first.
func (w *BitWriter) WriteI64(v int64) {
	w.WriteU64(uint64(v))
}

// WriteF32 writes a float32, canonicalizing NaN to 0x7FC00000.
func (w *BitWriter) WriteF32(v float32) {
	w.align()
	var bits uint32
	if math.IsNaN(float64(v)) {
		bits = 0x7FC00000 // canonical quiet NaN
	} else {
		bits = math.Float32bits(v)
	}
	w.buf = append(w.buf, byte(bits), byte(bits>>8), byte(bits>>16), byte(bits>>24))
}

// WriteF64 writes a float64, canonicalizing NaN to 0x7FF8000000000000.
func (w *BitWriter) WriteF64(v float64) {
	w.align()
	var bits uint64
	if math.IsNaN(v) {
		bits = 0x7FF8000000000000 // canonical quiet NaN
	} else {
		bits = math.Float64bits(v)
	}
	w.buf = append(w.buf,
		byte(bits), byte(bits>>8), byte(bits>>16), byte(bits>>24),
		byte(bits>>32), byte(bits>>40), byte(bits>>48), byte(bits>>56),
	)
}

// WriteLeb128 writes a LEB128-encoded unsigned integer, byte-aligned.
func (w *BitWriter) WriteLeb128(v uint64) {
	w.align()
	for {
		b := byte(v & 0x7f)
		v >>= 7
		if v != 0 {
			b |= 0x80
		}
		w.buf = append(w.buf, b)
		if v == 0 {
			break
		}
	}
}

// WriteZigZag writes a ZigZag + LEB128 encoded signed integer.
// typeBits is the bit width of the source type (e.g. 32 for int32, 64 for int64).
func (w *BitWriter) WriteZigZag(v int64, typeBits uint8) {
	encoded := uint64((v << 1) ^ (v >> (uint(typeBits) - 1)))
	w.WriteLeb128(encoded)
}

// WriteString writes a UTF-8 string with a LEB128 length prefix, byte-aligned.
func (w *BitWriter) WriteString(s string) {
	w.align()
	w.WriteLeb128(uint64(len(s)))
	w.buf = append(w.buf, s...)
}

// WriteBytes writes a byte slice with a LEB128 length prefix, byte-aligned.
func (w *BitWriter) WriteBytes(data []byte) {
	w.align()
	w.WriteLeb128(uint64(len(data)))
	w.buf = append(w.buf, data...)
}

// WriteRawBytes writes raw bytes with no length prefix, byte-aligned.
func (w *BitWriter) WriteRawBytes(data []byte) {
	w.align()
	w.buf = append(w.buf, data...)
}

// EnterRecursive increments the recursion depth and returns an error if the limit is exceeded.
func (w *BitWriter) EnterRecursive() error {
	w.depth++
	if w.depth > MaxRecursionDepth {
		return &EncodeError{
			Message: "recursive type nesting exceeded 64 levels",
			Err:     ErrRecursionLimit,
		}
	}
	return nil
}

// LeaveRecursive decrements the recursion depth.
func (w *BitWriter) LeaveRecursive() {
	if w.depth > 0 {
		w.depth--
	}
}

// Finish flushes any partial byte and returns the completed buffer.
func (w *BitWriter) Finish() []byte {
	w.FlushToByteBoundary()
	return w.buf
}
