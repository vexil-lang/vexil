"""Bit-level I/O for Vexil binary format.

This module provides BitWriter and BitReader classes that implement
the Vexil wire format specification (LSB-first bit packing, LEB128
varints, ZigZag encoding, little-endian multi-byte integers).
"""

from __future__ import annotations

import struct
from typing import Optional


class EncodeError(Exception):
    """Raised when encoding fails."""
    pass


class DecodeError(Exception):
    """Raised when decoding fails."""
    pass


MAX_RECURSION_DEPTH = 64
MAX_BYTES_LENGTH = 64 * 1024 * 1024  # 64 MiB


class BitWriter:
    """Write bits and bytes to a binary buffer in Vexil wire format.
    
    Sub-byte fields accumulate LSB-first within a byte. Multi-byte writes
    first align to a byte boundary, then append little-endian bytes.
    """
    
    def __init__(self):
        self._buf: list[int] = []
        self._current_byte: int = 0
        self._bit_offset: int = 0
        self._recursion_depth: int = 0
    
    def write_bits(self, value: int, count: int) -> None:
        """Write `count` bits from `value`, LSB first.
        
        Args:
            value: Integer value to write (lowest `count` bits are used)
            count: Number of bits to write (0-64)
        """
        if count == 0:
            return
        
        value = int(value)
        count = int(count)
        
        remaining = 8 - self._bit_offset
        
        # Fast path: value fits entirely in current byte
        if count <= remaining:
            mask = (1 << count) - 1
            self._current_byte |= (value & mask) << self._bit_offset
            self._bit_offset += count
            if self._bit_offset == 8:
                self._buf.append(self._current_byte)
                self._current_byte = 0
                self._bit_offset = 0
            return
        
        # Slow path: value spans byte boundaries
        # Write remaining bits in current byte
        if self._bit_offset > 0:
            bits_in_current = remaining
            mask = (1 << bits_in_current) - 1
            self._current_byte |= (value & mask) << self._bit_offset
            self._buf.append(self._current_byte)
            self._current_byte = 0
            value >>= bits_in_current
            count -= bits_in_current
            self._bit_offset = 0
        
        # Write full bytes
        while count >= 8:
            self._buf.append(value & 0xFF)
            value >>= 8
            count -= 8
        
        # Write remaining bits in new current byte
        if count > 0:
            self._current_byte = value & ((1 << count) - 1)
            self._bit_offset = count
    
    def write_bool(self, value: bool) -> None:
        """Write a single bit as a boolean."""
        self.write_bits(1 if value else 0, 1)
    
    def flush_to_byte_boundary(self) -> None:
        """Flush any partial byte to the buffer.
        
        Special case per spec: if nothing has been written (bit_offset == 0
        and buffer is empty), push a zero byte anyway.
        """
        if self._bit_offset == 0:
            if not self._buf:
                self._buf.append(0)
        else:
            self._buf.append(self._current_byte)
            self._current_byte = 0
            self._bit_offset = 0
    
    def _align(self) -> None:
        """Align to next byte boundary, flushing any partial byte."""
        if self._bit_offset > 0:
            self._buf.append(self._current_byte)
            self._current_byte = 0
            self._bit_offset = 0
    
    def write_u8(self, value: int) -> None:
        """Write an unsigned 8-bit value, byte-aligned."""
        self._align()
        self._buf.append(value & 0xFF)
    
    def write_u16(self, value: int) -> None:
        """Write an unsigned 16-bit value (little-endian), byte-aligned."""
        self._align()
        self._buf.extend(struct.pack('<H', value & 0xFFFF))
    
    def write_u32(self, value: int) -> None:
        """Write an unsigned 32-bit value (little-endian), byte-aligned."""
        self._align()
        self._buf.extend(struct.pack('<I', value & 0xFFFFFFFF))
    
    def write_u64(self, value: int) -> None:
        """Write an unsigned 64-bit value (little-endian), byte-aligned."""
        self._align()
        self._buf.extend(struct.pack('<Q', value & 0xFFFFFFFFFFFFFFFF))
    
    def write_i8(self, value: int) -> None:
        """Write a signed 8-bit value, byte-aligned."""
        self._align()
        self._buf.append(value & 0xFF)
    
    def write_i16(self, value: int) -> None:
        """Write a signed 16-bit value (little-endian), byte-aligned."""
        self.write_u16(value & 0xFFFF)
    
    def write_i32(self, value: int) -> None:
        """Write a signed 32-bit value (little-endian), byte-aligned."""
        self.write_u32(value & 0xFFFFFFFF)
    
    def write_i64(self, value: int) -> None:
        """Write a signed 64-bit value (little-endian), byte-aligned."""
        self.write_u64(value & 0xFFFFFFFFFFFFFFFF)
    
    def write_f32(self, value: float) -> None:
        """Write a 32-bit float (little-endian), byte-aligned.
        
        NaN values are canonicalized to 0x7FC00000.
        """
        self._align()
        if value != value:  # NaN check
            bits = 0x7FC00000
        else:
            bits = struct.unpack('<I', struct.pack('<f', value))[0]
        self._buf.extend(struct.pack('<I', bits))
    
    def write_f64(self, value: float) -> None:
        """Write a 64-bit float (little-endian), byte-aligned.
        
        NaN values are canonicalized to 0x7FF8000000000000.
        """
        self._align()
        if value != value:  # NaN check
            bits = 0x7FF8000000000000
        else:
            bits = struct.unpack('<Q', struct.pack('<d', value))[0]
        self._buf.extend(struct.pack('<Q', bits))
    
    def write_leb128(self, value: int) -> None:
        """Write an unsigned LEB128-encoded integer, byte-aligned."""
        self._align()
        value = int(value)
        while True:
            byte = value & 0x7F
            value >>= 7
            if value != 0:
                byte |= 0x80
            self._buf.append(byte)
            if value == 0:
                break
    
    def write_leb128_signed(self, value: int) -> None:
        """Write a signed LEB128-encoded integer, byte-aligned."""
        self._align()
        value = int(value)
        while True:
            byte = value & 0x7F
            value >>= 7
            # Sign-extend if needed
            if (byte & 0x40) != 0:
                more_bits = -1
            else:
                more_bits = 0
            if value != more_bits:
                byte |= 0x80
            self._buf.append(byte)
            if byte & 0x80 == 0:
                break
    
    def write_zigzag(self, value: int) -> None:
        """Write a ZigZag-encoded signed integer, byte-aligned.
        
        ZigZag maps signed integers to unsigned: 0 -> 0, -1 -> 1, 1 -> 2, -2 -> 3, etc.
        Uses formula: (value << 1) ^ (value >> 31) for 32-bit, extended for 64-bit.
        """
        if value < 0:
            encoded = (-value) * 2 - 1
        else:
            encoded = value * 2
        self.write_leb128(encoded)
    
    def write_string(self, value: str) -> None:
        """Write a UTF-8 string with LEB128 length prefix, byte-aligned."""
        self._align()
        data = value.encode('utf-8')
        self.write_leb128(len(data))
        self._buf.extend(data)
    
    def write_bytes(self, data: bytes) -> None:
        """Write raw bytes with LEB128 length prefix, byte-aligned."""
        self._align()
        self.write_leb128(len(data))
        self._buf.extend(data)
    
    def write_raw_bytes(self, data: bytes, length: int) -> None:
        """Write exactly `length` bytes from data, byte-aligned.
        
        Args:
            data: Source bytes
            length: Number of bytes to write (must be <= len(data))
        """
        self._align()
        if len(data) < length:
            raise EncodeError(f"write_raw_bytes: need {length} bytes, got {len(data)}")
        self._buf.extend(data[:length])
    
    def enter_nested(self) -> None:
        """Increment recursion depth for nested type encoding.
        
        Raises EncodeError if depth exceeds MAX_RECURSION_DEPTH.
        """
        self._recursion_depth += 1
        if self._recursion_depth > MAX_RECURSION_DEPTH:
            raise EncodeError(f"recursive type nesting exceeded {MAX_RECURSION_DEPTH} levels")
    
    def leave_nested(self) -> None:
        """Decrement recursion depth."""
        if self._recursion_depth > 0:
            self._recursion_depth -= 1
    
    def finish(self) -> bytes:
        """Finish writing and return the byte buffer."""
        self.flush_to_byte_boundary()
        return bytes(self._buf)


class BitReader:
    """Read bits and bytes from a binary buffer in Vexil wire format.
    
    Sub-byte reads extract bits LSB-first from the current byte.
    Multi-byte reads first align to a byte boundary, then interpret
    bytes as little-endian integers.
    """
    
    def __init__(self, data: bytes):
        self._data = data
        self._byte_pos = 0
        self._bit_offset = 0
        self._recursion_depth = 0
    
    def read_bits(self, count: int) -> int:
        """Read `count` bits LSB-first into an integer.
        
        Args:
            count: Number of bits to read (0-64)
            
        Returns:
            Integer value with lowest `count` bits populated
            
        Raises:
            DecodeError: If reading past end of buffer
        """
        if count == 0:
            return 0
        
        count = int(count)
        
        if self._byte_pos >= len(self._data):
            raise DecodeError("Unexpected end of data")
        
        remaining = 8 - self._bit_offset
        
        # Fast path: all bits in current byte
        if count <= remaining:
            byte = self._data[self._byte_pos]
            mask = (1 << count) - 1
            result = (byte >> self._bit_offset) & mask
            self._bit_offset += count
            if self._bit_offset == 8:
                self._bit_offset = 0
                self._byte_pos += 1
            return result
        
        # Slow path: bits span byte boundaries
        result = 0
        bits_read = 0
        
        while bits_read < count:
            if self._byte_pos >= len(self._data):
                raise DecodeError("Unexpected end of data")
            
            byte = self._data[self._byte_pos]
            available = 8 - self._bit_offset
            to_read = min(available, count - bits_read)
            
            mask = (1 << to_read) - 1
            bits = (byte >> self._bit_offset) & mask
            result |= bits << bits_read
            
            bits_read += to_read
            self._bit_offset += to_read
            
            if self._bit_offset == 8:
                self._bit_offset = 0
                self._byte_pos += 1
        
        return result
    
    def read_bool(self) -> bool:
        """Read a single bit as boolean."""
        return self.read_bits(1) != 0
    
    def flush_to_byte_boundary(self) -> None:
        """Advance to next byte boundary, discarding any remaining bits."""
        if self._bit_offset > 0:
            self._byte_pos += 1
            self._bit_offset = 0
    
    def _align(self) -> None:
        """Align to next byte boundary."""
        if self._bit_offset > 0:
            self._byte_pos += 1
            self._bit_offset = 0
    
    def _check_available(self, n: int) -> None:
        """Check that n bytes are available from current position."""
        if self._byte_pos + n > len(self._data):
            raise DecodeError("Unexpected end of data")
    
    def read_u8(self) -> int:
        """Read an unsigned 8-bit value, byte-aligned."""
        self._align()
        if self._byte_pos >= len(self._data):
            raise DecodeError("Unexpected end of data")
        value = self._data[self._byte_pos]
        self._byte_pos += 1
        return value
    
    def read_u16(self) -> int:
        """Read an unsigned 16-bit value (little-endian), byte-aligned."""
        self._align()
        self._check_available(2)
        value = struct.unpack('<H', bytes(self._data[self._byte_pos:self._byte_pos+2]))[0]
        self._byte_pos += 2
        return value
    
    def read_u32(self) -> int:
        """Read an unsigned 32-bit value (little-endian), byte-aligned."""
        self._align()
        self._check_available(4)
        value = struct.unpack('<I', bytes(self._data[self._byte_pos:self._byte_pos+4]))[0]
        self._byte_pos += 4
        return value
    
    def read_u64(self) -> int:
        """Read an unsigned 64-bit value (little-endian), byte-aligned."""
        self._align()
        self._check_available(8)
        value = struct.unpack('<Q', bytes(self._data[self._byte_pos:self._byte_pos+8]))[0]
        self._byte_pos += 8
        return value
    
    def read_i8(self) -> int:
        """Read a signed 8-bit value, byte-aligned."""
        value = self.read_u8()
        if value >= 0x80:
            value -= 0x100
        return value
    
    def read_i16(self) -> int:
        """Read a signed 16-bit value (little-endian), byte-aligned."""
        value = self.read_u16()
        if value >= 0x8000:
            value -= 0x10000
        return value
    
    def read_i32(self) -> int:
        """Read a signed 32-bit value (little-endian), byte-aligned."""
        value = self.read_u32()
        if value >= 0x80000000:
            value -= 0x100000000
        return value
    
    def read_i64(self) -> int:
        """Read a signed 64-bit value (little-endian), byte-aligned."""
        value = self.read_u64()
        if value >= 0x8000000000000000:
            value -= 0x10000000000000000
        return value
    
    def read_f32(self) -> float:
        """Read a 32-bit float (little-endian), byte-aligned."""
        self._align()
        self._check_available(4)
        bits = struct.unpack('<I', bytes(self._data[self._byte_pos:self._byte_pos+4]))[0]
        self._byte_pos += 4
        return struct.unpack('<f', struct.pack('<I', bits))[0]
    
    def read_f64(self) -> float:
        """Read a 64-bit float (little-endian), byte-aligned."""
        self._align()
        self._check_available(8)
        bits = struct.unpack('<Q', bytes(self._data[self._byte_pos:self._byte_pos+8]))[0]
        self._byte_pos += 8
        return struct.unpack('<d', struct.pack('<Q', bits))[0]
    
    def read_leb128(self) -> int:
        """Read an unsigned LEB128-encoded integer, byte-aligned."""
        self._align()
        result = 0
        shift = 0
        while True:
            if self._byte_pos >= len(self._data):
                raise DecodeError("Unexpected end of data")
            byte = self._data[self._byte_pos]
            self._byte_pos += 1
            result |= (byte & 0x7F) << shift
            if byte & 0x80 == 0:
                break
            shift += 7
        return result
    
    def read_leb128_signed(self) -> int:
        """Read a signed LEB128-encoded integer, byte-aligned."""
        self._align()
        result = 0
        shift = 0
        while True:
            if self._byte_pos >= len(self._data):
                raise DecodeError("Unexpected end of data")
            byte = self._data[self._byte_pos]
            self._byte_pos += 1
            result |= (byte & 0x7F) << shift
            if byte & 0x80 == 0:
                break
            shift += 7
        # Sign-extend
        if shift < 64 and (result & (1 << (shift - 1))) != 0:
            result |= - (1 << shift)
        return result
    
    def read_zigzag(self) -> int:
        """Read a ZigZag-encoded signed integer, byte-aligned."""
        unsigned = self.read_leb128()
        # Decode ZigZag: if even, unsigned/2; if odd, -(unsigned+1)/2
        if unsigned & 1:
            return -int((unsigned + 1) // 2)
        else:
            return int(unsigned // 2)
    
    def read_string(self) -> str:
        """Read a UTF-8 string with LEB128 length prefix, byte-aligned."""
        self._align()
        length = self.read_leb128()
        self._check_available(length)
        value = bytes(self._data[self._byte_pos:self._byte_pos+length]).decode('utf-8')
        self._byte_pos += length
        return value
    
    def read_bytes(self, length: int) -> bytes:
        """Read exactly `length` raw bytes, byte-aligned."""
        self._align()
        self._check_available(length)
        value = bytes(self._data[self._byte_pos:self._byte_pos+length])
        self._byte_pos += length
        return value
    
    def enter_nested(self) -> None:
        """Increment recursion depth for nested type decoding.
        
        Raises DecodeError if depth exceeds MAX_RECURSION_DEPTH.
        """
        self._recursion_depth += 1
        if self._recursion_depth > MAX_RECURSION_DEPTH:
            raise DecodeError(f"recursive type nesting exceeded {MAX_RECURSION_DEPTH} levels")
    
    def leave_nested(self) -> None:
        """Decrement recursion depth."""
        if self._recursion_depth > 0:
            self._recursion_depth -= 1
    
    def remaining(self) -> int:
        """Return number of bytes remaining in buffer."""
        if self._byte_pos >= len(self._data):
            return 0
        return len(self._data) - self._byte_pos


# Aliases for generated code compatibility
_BitWriter = BitWriter
_BitReader = BitReader


def pack(obj, writer: BitWriter) -> None:
    """Pack an object to the BitWriter using its pack() method."""
    obj.pack(writer)


def unpack(cls, data: bytes, reader: BitReader) -> object:
    """Unpack bytes to an object using cls.unpack()."""
    return cls.unpack(data)