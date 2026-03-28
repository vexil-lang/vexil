package vexil

import (
	"math"
	"unicode/utf8"
)

// BitReader reads fields LSB-first at the bit level from a byte slice.
//
// Sub-byte reads pull individual bits from the current byte. Multi-byte reads
// first align to the next byte boundary, then interpret the bytes as little-endian.
type BitReader struct {
	data    []byte
	bytePos int
	bitOff  uint8
	depth   uint32
}

// NewBitReader creates a new BitReader over the given byte slice.
func NewBitReader(data []byte) *BitReader {
	return &BitReader{data: data}
}

// remaining returns the number of bytes from bytePos to end.
func (r *BitReader) remaining() int {
	rem := len(r.data) - r.bytePos
	if rem < 0 {
		return 0
	}
	return rem
}

// ReadBits reads count bits LSB-first into a uint64.
func (r *BitReader) ReadBits(count uint8) (uint64, error) {
	var result uint64
	for i := uint8(0); i < count; i++ {
		if r.bytePos >= len(r.data) {
			return 0, ErrUnexpectedEOF
		}
		bit := uint64((r.data[r.bytePos] >> r.bitOff) & 1)
		result |= bit << i
		r.bitOff++
		if r.bitOff == 8 {
			r.bytePos++
			r.bitOff = 0
		}
	}
	return result, nil
}

// ReadBool reads a single bit as a bool.
func (r *BitReader) ReadBool() (bool, error) {
	v, err := r.ReadBits(1)
	if err != nil {
		return false, err
	}
	return v != 0, nil
}

// FlushToByteBoundary advances to the next byte boundary, discarding remaining bits.
func (r *BitReader) FlushToByteBoundary() {
	if r.bitOff > 0 {
		r.bytePos++
		r.bitOff = 0
	}
}

// ReadU8 reads a uint8, aligning to a byte boundary first.
func (r *BitReader) ReadU8() (uint8, error) {
	r.FlushToByteBoundary()
	if r.remaining() < 1 {
		return 0, ErrUnexpectedEOF
	}
	v := r.data[r.bytePos]
	r.bytePos++
	return v, nil
}

// ReadU16 reads a little-endian uint16, aligning first.
func (r *BitReader) ReadU16() (uint16, error) {
	r.FlushToByteBoundary()
	if r.remaining() < 2 {
		return 0, ErrUnexpectedEOF
	}
	v := uint16(r.data[r.bytePos]) | uint16(r.data[r.bytePos+1])<<8
	r.bytePos += 2
	return v, nil
}

// ReadU32 reads a little-endian uint32, aligning first.
func (r *BitReader) ReadU32() (uint32, error) {
	r.FlushToByteBoundary()
	if r.remaining() < 4 {
		return 0, ErrUnexpectedEOF
	}
	v := uint32(r.data[r.bytePos]) |
		uint32(r.data[r.bytePos+1])<<8 |
		uint32(r.data[r.bytePos+2])<<16 |
		uint32(r.data[r.bytePos+3])<<24
	r.bytePos += 4
	return v, nil
}

// ReadU64 reads a little-endian uint64, aligning first.
func (r *BitReader) ReadU64() (uint64, error) {
	r.FlushToByteBoundary()
	if r.remaining() < 8 {
		return 0, ErrUnexpectedEOF
	}
	v := uint64(r.data[r.bytePos]) |
		uint64(r.data[r.bytePos+1])<<8 |
		uint64(r.data[r.bytePos+2])<<16 |
		uint64(r.data[r.bytePos+3])<<24 |
		uint64(r.data[r.bytePos+4])<<32 |
		uint64(r.data[r.bytePos+5])<<40 |
		uint64(r.data[r.bytePos+6])<<48 |
		uint64(r.data[r.bytePos+7])<<56
	r.bytePos += 8
	return v, nil
}

// ReadI8 reads an int8, aligning first.
func (r *BitReader) ReadI8() (int8, error) {
	v, err := r.ReadU8()
	return int8(v), err
}

// ReadI16 reads a little-endian int16, aligning first.
func (r *BitReader) ReadI16() (int16, error) {
	v, err := r.ReadU16()
	return int16(v), err
}

// ReadI32 reads a little-endian int32, aligning first.
func (r *BitReader) ReadI32() (int32, error) {
	v, err := r.ReadU32()
	return int32(v), err
}

// ReadI64 reads a little-endian int64, aligning first.
func (r *BitReader) ReadI64() (int64, error) {
	v, err := r.ReadU64()
	return int64(v), err
}

// ReadF32 reads a little-endian float32, aligning first.
func (r *BitReader) ReadF32() (float32, error) {
	bits, err := r.ReadU32()
	if err != nil {
		return 0, err
	}
	return math.Float32frombits(bits), nil
}

// ReadF64 reads a little-endian float64, aligning first.
func (r *BitReader) ReadF64() (float64, error) {
	bits, err := r.ReadU64()
	if err != nil {
		return 0, err
	}
	return math.Float64frombits(bits), nil
}

// ReadLeb128 reads a LEB128-encoded uint64, consuming at most maxBytes bytes.
func (r *BitReader) ReadLeb128(maxBytes uint8) (uint64, error) {
	r.FlushToByteBoundary()
	var result uint64
	var shift uint32
	for i := uint8(0); ; i++ {
		if i >= maxBytes {
			return 0, ErrInvalidVarint
		}
		if r.bytePos >= len(r.data) {
			return 0, ErrUnexpectedEOF
		}
		b := r.data[r.bytePos]
		r.bytePos++
		result |= uint64(b&0x7F) << shift
		shift += 7
		if b&0x80 == 0 {
			// Reject overlong: if not first byte and byte is 0
			if i > 0 && b == 0 {
				return 0, ErrInvalidVarint
			}
			return result, nil
		}
	}
}

// ReadZigZag reads a ZigZag + LEB128 encoded signed integer.
func (r *BitReader) ReadZigZag(typeBits uint8, maxBytes uint8) (int64, error) {
	raw, err := r.ReadLeb128(maxBytes)
	if err != nil {
		return 0, err
	}
	return zigzagDecode(raw), nil
}

// zigzagDecode decodes a ZigZag-encoded unsigned value back to signed.
func zigzagDecode(n uint64) int64 {
	return int64(n>>1) ^ -int64(n&1)
}

// ReadString reads a length-prefixed UTF-8 string.
func (r *BitReader) ReadString() (string, error) {
	r.FlushToByteBoundary()
	length, err := r.ReadLeb128(MaxLengthPrefixBytes)
	if err != nil {
		return "", err
	}
	if length > MaxBytesLength {
		return "", ErrLimitExceeded
	}
	n := int(length)
	if r.remaining() < n {
		return "", ErrUnexpectedEOF
	}
	s := string(r.data[r.bytePos : r.bytePos+n])
	r.bytePos += n
	if !utf8.ValidString(s) {
		return "", ErrInvalidUTF8
	}
	return s, nil
}

// ReadBytes reads a length-prefixed byte slice.
func (r *BitReader) ReadBytes() ([]byte, error) {
	r.FlushToByteBoundary()
	length, err := r.ReadLeb128(MaxLengthPrefixBytes)
	if err != nil {
		return nil, err
	}
	if length > MaxBytesLength {
		return nil, ErrLimitExceeded
	}
	n := int(length)
	if r.remaining() < n {
		return nil, ErrUnexpectedEOF
	}
	result := make([]byte, n)
	copy(result, r.data[r.bytePos:r.bytePos+n])
	r.bytePos += n
	return result, nil
}

// ReadRawBytes reads exactly n raw bytes with no length prefix.
func (r *BitReader) ReadRawBytes(n int) ([]byte, error) {
	r.FlushToByteBoundary()
	if r.remaining() < n {
		return nil, ErrUnexpectedEOF
	}
	result := make([]byte, n)
	copy(result, r.data[r.bytePos:r.bytePos+n])
	r.bytePos += n
	return result, nil
}

// ReadRemaining reads all remaining bytes from the current position.
// Flushes to byte boundary first. Returns an empty slice if no bytes remain.
func (r *BitReader) ReadRemaining() []byte {
	r.FlushToByteBoundary()
	rem := r.remaining()
	if rem == 0 {
		return []byte{}
	}
	result := make([]byte, rem)
	copy(result, r.data[r.bytePos:])
	r.bytePos = len(r.data)
	return result
}

// EnterRecursive increments the recursion depth and returns an error if the limit is exceeded.
func (r *BitReader) EnterRecursive() error {
	r.depth++
	if r.depth > MaxRecursionDepth {
		return ErrRecursionLimit
	}
	return nil
}

// LeaveRecursive decrements the recursion depth.
func (r *BitReader) LeaveRecursive() {
	if r.depth > 0 {
		r.depth--
	}
}
