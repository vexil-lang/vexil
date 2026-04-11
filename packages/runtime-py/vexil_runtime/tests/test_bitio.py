"""Tests for bitio.py - BitWriter and BitReader.

These tests verify byte-for-byte identical output to the Rust vexil-runtime.
"""

import struct
import sys
import math
from vexil_runtime.bitio import BitWriter, BitReader, EncodeError, DecodeError


def to_hex(bytes_buf):
    """Convert bytes to hex string for easy comparison."""
    return bytes_buf.hex()


class TestBitWriterBits:
    """Tests for BitWriter.write_bits and write_bool."""

    def test_write_single_bit_true(self):
        w = BitWriter()
        w.write_bool(True)
        assert to_hex(w.finish()) == "01"

    def test_write_single_bit_false(self):
        w = BitWriter()
        w.write_bool(False)
        assert to_hex(w.finish()) == "00"

    def test_write_bits_lsb_first(self):
        w = BitWriter()
        w.write_bits(5, 3)  # 101
        w.write_bits(19, 5)  # 10011
        # LSB-first: byte = 10011_101 = 0x9D
        assert to_hex(w.finish()) == "9d"

    def test_write_bits_cross_byte_boundary(self):
        w = BitWriter()
        w.write_bits(5, 3)
        w.write_bits(19, 5)
        w.write_bits(42, 6)  # 101010
        # Byte 0: 0x9D, Byte 1: 00_101010 = 0x2A
        assert to_hex(w.finish()) == "9d2a"

    def test_write_bits_zero_count(self):
        w = BitWriter()
        w.write_bits(42, 0)
        w.write_u8(0xAB)
        assert to_hex(w.finish()) == "ab"

    def test_write_bits_exact_byte(self):
        w = BitWriter()
        w.write_bits(0xFF, 8)
        assert to_hex(w.finish()) == "ff"


class TestBitWriterFlush:
    """Tests for BitWriter.flush_to_byte_boundary."""

    def test_flush_pads_zeros(self):
        w = BitWriter()
        w.write_bits(0b101, 3)
        w.flush_to_byte_boundary()
        w.write_bits(0xFF, 8)
        assert to_hex(w.finish()) == "05ff"

    def test_empty_flush_produces_zero_byte(self):
        w = BitWriter()
        w.flush_to_byte_boundary()
        assert to_hex(w.finish()) == "00"

    def test_empty_finish_produces_zero_byte(self):
        w = BitWriter()
        assert to_hex(w.finish()) == "00"


class TestBitWriterIntegers:
    """Tests for unsigned integer writes."""

    def test_write_u8(self):
        w = BitWriter()
        w.write_u8(0xAB)
        assert to_hex(w.finish()) == "ab"

    def test_write_u8_after_bool(self):
        w = BitWriter()
        w.write_bool(True)
        w.write_u8(0xAB)
        assert to_hex(w.finish()) == "01ab"

    def test_write_u16_le(self):
        w = BitWriter()
        w.write_u16(0x0102)
        assert to_hex(w.finish()) == "0201"

    def test_write_u32_le(self):
        w = BitWriter()
        w.write_u32(0x01020304)
        assert to_hex(w.finish()) == "04030201"

    def test_write_u64_le(self):
        w = BitWriter()
        w.write_u64(0x0102030405060708)
        assert to_hex(w.finish()) == "0807060504030201"


class TestBitWriterSignedIntegers:
    """Tests for signed integer writes."""

    def test_write_i8(self):
        w = BitWriter()
        w.write_i8(-1)
        assert to_hex(w.finish()) == "ff"

    def test_write_i16_negative(self):
        w = BitWriter()
        w.write_i16(-1)
        assert to_hex(w.finish()) == "ffff"

    def test_write_i32_negative(self):
        w = BitWriter()
        w.write_i32(-1)
        assert to_hex(w.finish()) == "ffffffff"

    def test_write_i64_negative(self):
        w = BitWriter()
        w.write_i64(-1)
        assert to_hex(w.finish()) == "ffffffffffffffff"

    def test_write_i16_positive(self):
        w = BitWriter()
        w.write_i16(0x1234)
        assert to_hex(w.finish()) == "3412"

    def test_write_i32_positive(self):
        w = BitWriter()
        w.write_i32(-42)
        # -42 in little-endian two's complement
        expected = struct.pack('<i', -42)
        assert to_hex(w.finish()) == expected.hex()


class TestBitWriterFloat:
    """Tests for floating point writes."""

    def test_write_f32_nan_canonicalized(self):
        w = BitWriter()
        w.write_f32(float('nan'))
        # 0x7FC00000 in little-endian
        assert to_hex(w.finish()) == "0000c07f"

    def test_write_f32_negative_zero_preserved(self):
        w = BitWriter()
        w.write_f32(-0.0)
        expected = struct.pack('<f', -0.0)
        assert to_hex(w.finish()) == expected.hex()
        # Ensure it's different from positive zero
        assert to_hex(w.finish()) != struct.pack('<f', 0.0).hex()

    def test_write_f32_pi(self):
        w = BitWriter()
        pi_f32 = struct.pack('<f', struct.unpack('<f', struct.pack('<f', math.pi))[0])
        w.write_f32(struct.unpack('<f', struct.pack('<f', math.pi))[0])
        # PI as f32 = 0x40490FDB, LE = DB 0F 49 40
        assert to_hex(w.finish()) == "db0f4940"

    def test_write_f64_nan_canonicalized(self):
        w = BitWriter()
        w.write_f64(float('nan'))
        # 0x7FF8000000000000 in little-endian
        assert to_hex(w.finish()) == "000000000000f87f"

    def test_write_f64_negative_zero(self):
        w = BitWriter()
        w.write_f64(-0.0)
        expected = struct.pack('<d', -0.0)
        assert to_hex(w.finish()) == expected.hex()


class TestBitWriterLeb128:
    """Tests for LEB128 encoding."""

    def test_write_leb128_zero(self):
        w = BitWriter()
        w.write_leb128(0)
        assert to_hex(w.finish()) == "00"

    def test_write_leb128_127(self):
        w = BitWriter()
        w.write_leb128(127)
        assert to_hex(w.finish()) == "7f"

    def test_write_leb128_128(self):
        w = BitWriter()
        w.write_leb128(128)
        # 128 = 0x80, needs continuation byte
        assert to_hex(w.finish()) == "8001"

    def test_write_leb128_300(self):
        w = BitWriter()
        w.write_leb128(300)
        # 300 = 0x12C -> LEB128: 0xAC (continuation), 0x02
        assert to_hex(w.finish()) == "ac02"


class TestBitWriterZigzag:
    """Tests for ZigZag encoding."""

    def test_write_zigzag_zero(self):
        w = BitWriter()
        w.write_zigzag(0)
        assert to_hex(w.finish()) == "00"

    def test_write_zigzag_neg1(self):
        w = BitWriter()
        w.write_zigzag(-1)
        # ZigZag(-1) = 1
        assert to_hex(w.finish()) == "01"

    def test_write_zigzag_pos1(self):
        w = BitWriter()
        w.write_zigzag(1)
        # ZigZag(1) = 2
        assert to_hex(w.finish()) == "02"

    def test_write_zigzag_neg2(self):
        w = BitWriter()
        w.write_zigzag(-2)
        # ZigZag(-2) = 3
        assert to_hex(w.finish()) == "03"


class TestBitWriterStringBytes:
    """Tests for string and bytes writes."""

    def test_write_string_hi(self):
        w = BitWriter()
        w.write_string("hi")
        # LEB128(2) + "hi"
        assert to_hex(w.finish()) == "026869"

    def test_write_string_hello(self):
        w = BitWriter()
        w.write_string("hello")
        # LEB128(5) + "hello"
        assert to_hex(w.finish()) == "0568656c6c6f"

    def test_write_string_empty(self):
        w = BitWriter()
        w.write_string("")
        # LEB128(0)
        assert to_hex(w.finish()) == "00"

    def test_write_bytes(self):
        w = BitWriter()
        w.write_bytes(b'\xde\xad')
        # LEB128(2) + 0xDE 0xAD
        assert to_hex(w.finish()) == "02dead"

    def test_write_raw_bytes(self):
        w = BitWriter()
        w.write_raw_bytes(b'\xca\xfe', 2)
        assert to_hex(w.finish()) == "cafe"


class TestBitWriterRecursion:
    """Tests for recursion depth tracking."""

    def test_recursion_depth_64_succeeds(self):
        w = BitWriter()
        for _ in range(64):
            w.enter_nested()
        # Should not raise

    def test_recursion_depth_65_fails(self):
        w = BitWriter()
        for _ in range(64):
            w.enter_nested()
        try:
            w.enter_nested()
            assert False, "Expected EncodeError"
        except EncodeError as e:
            assert "recursion" in str(e).lower() or "64" in str(e)

    def test_leave_allows_reentry(self):
        w = BitWriter()
        for _ in range(64):
            w.enter_nested()
        w.leave_nested()
        w.enter_nested()  # Should not raise


class TestBitReaderBits:
    """Tests for BitReader.read_bits and read_bool."""

    def test_read_bool_true(self):
        r = BitReader(b'\x01')
        assert r.read_bool() is True

    def test_read_bool_false(self):
        r = BitReader(b'\x00')
        assert r.read_bool() is False

    def test_read_sub_byte_lsb_first(self):
        # 0x9D = 10011101 -> LSB-first: bits[0..3] = 101 = 5, bits[3..8] = 10011 = 19
        r = BitReader(b'\x9d')
        assert r.read_bits(3) == 5
        assert r.read_bits(5) == 19

    def test_read_across_byte_boundary(self):
        r = BitReader(b'\x9d\x2a')
        assert r.read_bits(3) == 5
        assert r.read_bits(5) == 19
        assert r.read_bits(6) == 42


class TestBitReaderFlush:
    """Tests for BitReader.flush_to_byte_boundary."""

    def test_flush_skips_remaining_bits(self):
        w = BitWriter()
        w.write_bits(0b101, 3)
        w.flush_to_byte_boundary()
        w.write_u8(0xab)
        buf = w.finish()

        r = BitReader(buf)
        assert r.read_bits(3) == 5
        r.flush_to_byte_boundary()
        assert r.read_u8() == 0xab


class TestBitReaderIntegers:
    """Tests for unsigned integer reads."""

    def test_read_u8(self):
        r = BitReader(b'\xff')
        assert r.read_u8() == 255

    def test_read_u16_le(self):
        r = BitReader(b'\x02\x01')
        assert r.read_u16() == 258

    def test_read_u32_le(self):
        r = BitReader(b'\x78\x56\x34\x12')
        assert r.read_u32() == 0x12345678

    def test_read_u64_le(self):
        r = BitReader(b'\x08\x07\x06\x05\x04\x03\x02\x01')
        assert r.read_u64() == 0x0102030405060708


class TestBitReaderSignedIntegers:
    """Tests for signed integer reads."""

    def test_read_i8_negative(self):
        r = BitReader(b'\xff')
        assert r.read_i8() == -1

    def test_read_i16_negative(self):
        r = BitReader(b'\xff\xff')
        assert r.read_i16() == -1

    def test_read_i32_negative(self):
        r = BitReader(b'\xff\xff\xff\xff')
        assert r.read_i32() == -1

    def test_read_i64_negative(self):
        r = BitReader(b'\xff\xff\xff\xff\xff\xff\xff\xff')
        assert r.read_i64() == -1


class TestBitReaderFloat:
    """Tests for floating point reads."""

    def test_read_f32_nan_canonical(self):
        r = BitReader(b'\x00\x00\xc0\x7f')
        v = r.read_f32()
        assert math.isnan(v)

    def test_read_f64_nan_canonical(self):
        r = BitReader(b'\x00\x00\x00\x00\x00\x00\xf8\x7f')
        v = r.read_f64()
        assert math.isnan(v)

    def test_read_f32_negative_zero(self):
        r = BitReader(b'\x00\x00\x00\x80')
        v = r.read_f32()
        assert math.copysign(1.0, v) < 0  # Negative zero

    def test_read_f64_negative_zero(self):
        r = BitReader(b'\x00\x00\x00\x00\x00\x00\x00\x80')
        v = r.read_f64()
        assert math.copysign(1.0, v) < 0


class TestBitReaderLeb128:
    """Tests for LEB128 reading."""

    def test_read_leb128_zero(self):
        r = BitReader(b'\x00')
        assert r.read_leb128() == 0

    def test_read_leb128_127(self):
        r = BitReader(b'\x7f')
        assert r.read_leb128() == 127

    def test_read_leb128_128(self):
        r = BitReader(b'\x80\x01')
        assert r.read_leb128() == 128

    def test_read_leb128_300(self):
        r = BitReader(b'\xac\x02')
        assert r.read_leb128() == 300


class TestBitReaderStringBytes:
    """Tests for string and bytes reads."""

    def test_read_string_hello(self):
        r = BitReader(b'\x05\x68\x65\x6c\x6c\x6f')
        assert r.read_string() == "hello"

    def test_read_string_empty(self):
        r = BitReader(b'\x00')
        assert r.read_string() == ""

    def test_read_bytes(self):
        r = BitReader(b'\xde\xad')
        assert r.read_bytes(2) == b'\xde\xad'

    def test_read_raw_bytes(self):
        r = BitReader(b'\xca\xfe')
        assert r.read_bytes(2) == b'\xca\xfe'


class TestBitReaderRecursion:
    """Tests for recursion depth tracking."""

    def test_recursion_depth_64_succeeds(self):
        r = BitReader(b'')
        for _ in range(64):
            r.enter_nested()
        # Should not raise

    def test_recursion_depth_65_fails(self):
        r = BitReader(b'')
        for _ in range(64):
            r.enter_nested()
        try:
            r.enter_nested()
            assert False, "Expected DecodeError"
        except DecodeError as e:
            assert "recursion" in str(e).lower() or "64" in str(e)


class TestBitReaderErrors:
    """Tests for error handling."""

    def test_unexpected_eof_read_u8(self):
        r = BitReader(b'')
        try:
            r.read_u8()
            assert False, "Expected DecodeError"
        except DecodeError as e:
            assert "end" in str(e).lower()

    def test_unexpected_eof_read_bits(self):
        r = BitReader(b'')
        try:
            r.read_bits(1)
            assert False, "Expected DecodeError"
        except DecodeError as e:
            assert "end" in str(e).lower()

    def test_unexpected_eof_read_u16(self):
        r = BitReader(b'\x01')
        try:
            r.read_u16()
            assert False, "Expected DecodeError"
        except DecodeError as e:
            assert "end" in str(e).lower()


class TestBitReaderRemaining:
    """Tests for remaining bytes tracking."""

    def test_remaining_initial(self):
        r = BitReader(b'\x01\x02\x03')
        assert r.remaining() == 3

    def test_remaining_after_read(self):
        r = BitReader(b'\x01\x02\x03')
        r.read_u8()
        assert r.remaining() == 2


class TestRoundTrip:
    """Tests that verify round-trip encoding/decoding matches Rust output."""

    def test_roundtrip_sub_byte(self):
        w = BitWriter()
        w.write_bits(5, 3)
        w.write_bits(19, 5)
        w.write_bits(42, 6)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_bits(3) == 5
        assert r.read_bits(5) == 19
        assert r.read_bits(6) == 42

    def test_roundtrip_u16(self):
        w = BitWriter()
        w.write_u16(0x1234)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_u16() == 0x1234

    def test_roundtrip_u32(self):
        w = BitWriter()
        w.write_u32(0x12345678)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_u32() == 0x12345678

    def test_roundtrip_u64(self):
        w = BitWriter()
        w.write_u64(0x123456789ABCDEF0)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_u64() == 0x123456789ABCDEF0

    def test_roundtrip_i8(self):
        w = BitWriter()
        w.write_i8(-42)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_i8() == -42

    def test_roundtrip_i16(self):
        w = BitWriter()
        w.write_i16(-42)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_i16() == -42

    def test_roundtrip_i32(self):
        w = BitWriter()
        w.write_i32(-42)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_i32() == -42

    def test_roundtrip_i64(self):
        w = BitWriter()
        w.write_i64(-42)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_i64() == -42

    def test_roundtrip_f32(self):
        w = BitWriter()
        w.write_f32(struct.unpack('<f', struct.pack('<f', math.pi))[0])
        buf = w.finish()
        r = BitReader(buf)
        result = r.read_f32()
        # F32 precision - round-trip should match exactly
        expected = struct.unpack('<f', struct.pack('<f', math.pi))[0]
        assert result == expected

    def test_roundtrip_f64(self):
        w = BitWriter()
        w.write_f64(math.pi)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_f64() == math.pi

    def test_roundtrip_f32_nan(self):
        w = BitWriter()
        w.write_f32(float('nan'))
        buf = w.finish()
        r = BitReader(buf)
        v = r.read_f32()
        assert math.isnan(v)

    def test_roundtrip_f64_nan(self):
        w = BitWriter()
        w.write_f64(float('nan'))
        buf = w.finish()
        r = BitReader(buf)
        v = r.read_f64()
        assert math.isnan(v)

    def test_roundtrip_leb128(self):
        w = BitWriter()
        w.write_leb128(300)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_leb128() == 300

    def test_roundtrip_zigzag(self):
        w = BitWriter()
        w.write_zigzag(-42)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_zigzag() == -42

    def test_roundtrip_zigzag_positive(self):
        w = BitWriter()
        w.write_zigzag(123)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_zigzag() == 123

    def test_roundtrip_string(self):
        w = BitWriter()
        w.write_string("hello")
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_string() == "hello"

    def test_roundtrip_string_unicode(self):
        w = BitWriter()
        w.write_string("hello \u4e16\u754c")
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_string() == "hello \u4e16\u754c"

    def test_roundtrip_bytes(self):
        w = BitWriter()
        w.write_bytes(b'\x00\x01\x02\x03')
        buf = w.finish()
        r = BitReader(buf)
        length = r.read_leb128()
        assert r.read_bytes(length) == b'\x00\x01\x02\x03'

    def test_roundtrip_bool_true(self):
        w = BitWriter()
        w.write_bool(True)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_bool() is True

    def test_roundtrip_bool_false(self):
        w = BitWriter()
        w.write_bool(False)
        buf = w.finish()
        r = BitReader(buf)
        assert r.read_bool() is False


class TestRustCompliance:
    """Tests that verify byte-for-byte output matches Rust vexil-runtime."""

    def test_f32_nan_bytes(self):
        """Rust canonicalizes NaN to 0x7FC00000."""
        w = BitWriter()
        w.write_f32(float('nan'))
        buf = w.finish()
        # Expected: 0x7FC00000 in little-endian = 00 00 C0 7F
        assert buf == bytes([0x00, 0x00, 0xC0, 0x7F])

    def test_f64_nan_bytes(self):
        """Rust canonicalizes NaN to 0x7FF8000000000000."""
        w = BitWriter()
        w.write_f64(float('nan'))
        buf = w.finish()
        # Expected: 0x7FF8000000000000 in little-endian
        expected = struct.pack('<Q', 0x7FF8000000000000)
        assert buf == expected

    def test_leb128_300_bytes(self):
        """300 in LEB128 is [0xAC, 0x02]."""
        w = BitWriter()
        w.write_leb128(300)
        assert w.finish() == bytes([0xAC, 0x02])

    def test_zigzag_neg1_bytes(self):
        """ZigZag(-1) = 1 in LEB128."""
        w = BitWriter()
        w.write_zigzag(-1)
        assert w.finish() == bytes([0x01])

    def test_empty_message_zero_byte(self):
        """Empty BitWriter should produce [0x00] per spec."""
        w = BitWriter()
        assert w.finish() == bytes([0x00])

    def test_u16_little_endian(self):
        """u16 0x0102 should be [0x02, 0x01]."""
        w = BitWriter()
        w.write_u16(0x0102)
        assert w.finish() == bytes([0x02, 0x01])

    def test_u32_little_endian(self):
        """u32 0x01020304 should be [0x04, 0x03, 0x02, 0x01]."""
        w = BitWriter()
        w.write_u32(0x01020304)
        assert w.finish() == bytes([0x04, 0x03, 0x02, 0x01])

    def test_string_with_leb128_prefix(self):
        """'hi' should be [0x02, 'h', 'i']."""
        w = BitWriter()
        w.write_string("hi")
        assert w.finish() == bytes([0x02, 0x68, 0x69])


if __name__ == "__main__":
    import pytest
    sys.exit(pytest.main([__file__, "-v"]))
