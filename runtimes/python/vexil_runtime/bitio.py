"""Bit-level I/O for Vexil binary format."""

import struct
from typing import Optional


class BitWriter:
    """Write bits and bytes to a binary buffer."""
    
    def __init__(self):
        self._buffer = bytearray()
        self._bit_pos = 0  # Current bit position in last byte
    
    def write_u8(self, value: int) -> None:
        """Write an unsigned 8-bit value."""
        self._align_to_byte()
        self._buffer.append(value & 0xFF)
    
    def write_u16(self, value: int) -> None:
        """Write an unsigned 16-bit value (little-endian)."""
        self._align_to_byte()
        self._buffer.extend(struct.pack("<H", value))
    
    def write_u32(self, value: int) -> None:
        """Write an unsigned 32-bit value (little-endian)."""
        self._align_to_byte()
        self._buffer.extend(struct.pack("<I", value))
    
    def write_u64(self, value: int) -> None:
        """Write an unsigned 64-bit value (little-endian)."""
        self._align_to_byte()
        self._buffer.extend(struct.pack("<Q", value))
    
    def write_f32(self, value: float) -> None:
        """Write a 32-bit float (little-endian)."""
        self._align_to_byte()
        self._buffer.extend(struct.pack("<f", value))
    
    def write_f64(self, value: float) -> None:
        """Write a 64-bit float (little-endian)."""
        self._align_to_byte()
        self._buffer.extend(struct.pack("<d", value))
    
    def write_bytes(self, data: bytes) -> None:
        """Write raw bytes."""
        self._align_to_byte()
        self._buffer.extend(data)
    
    def write_bool(self, value: bool) -> None:
        """Write a single bit as boolean."""
        if self._bit_pos == 0:
            self._buffer.append(0)
        
        if value:
            self._buffer[-1] |= (1 << self._bit_pos)
        
        self._bit_pos = (self._bit_pos + 1) % 8
    
    def _align_to_byte(self) -> None:
        """Align to next byte boundary if not already aligned."""
        if self._bit_pos != 0:
            self._bit_pos = 0
    
    def finish(self) -> bytes:
        """Return the written bytes."""
        return bytes(self._buffer)


class BitReader:
    """Read bits and bytes from a binary buffer."""
    
    def __init__(self, data: bytes):
        self._buffer = data
        self._byte_pos = 0
        self._bit_pos = 0
    
    def read_u8(self) -> int:
        """Read an unsigned 8-bit value."""
        self._align_to_byte()
        if self._byte_pos >= len(self._buffer):
            raise EOFError("Unexpected end of data")
        value = self._buffer[self._byte_pos]
        self._byte_pos += 1
        return value
    
    def read_u16(self) -> int:
        """Read an unsigned 16-bit value (little-endian)."""
        self._align_to_byte()
        if self._byte_pos + 2 > len(self._buffer):
            raise EOFError("Unexpected end of data")
        value = struct.unpack("<H", self._buffer[self._byte_pos:self._byte_pos+2])[0]
        self._byte_pos += 2
        return value
    
    def read_u32(self) -> int:
        """Read an unsigned 32-bit value (little-endian)."""
        self._align_to_byte()
        if self._byte_pos + 4 > len(self._buffer):
            raise EOFError("Unexpected end of data")
        value = struct.unpack("<I", self._buffer[self._byte_pos:self._byte_pos+4])[0]
        self._byte_pos += 4
        return value
    
    def read_u64(self) -> int:
        """Read an unsigned 64-bit value (little-endian)."""
        self._align_to_byte()
        if self._byte_pos + 8 > len(self._buffer):
            raise EOFError("Unexpected end of data")
        value = struct.unpack("<Q", self._buffer[self._byte_pos:self._byte_pos+8])[0]
        self._byte_pos += 8
        return value
    
    def read_f32(self) -> float:
        """Read a 32-bit float (little-endian)."""
        self._align_to_byte()
        if self._byte_pos + 4 > len(self._buffer):
            raise EOFError("Unexpected end of data")
        value = struct.unpack("<f", self._buffer[self._byte_pos:self._byte_pos+4])[0]
        self._byte_pos += 4
        return value
    
    def read_f64(self) -> float:
        """Read a 64-bit float (little-endian)."""
        self._align_to_byte()
        if self._byte_pos + 8 > len(self._buffer):
            raise EOFError("Unexpected end of data")
        value = struct.unpack("<d", self._buffer[self._byte_pos:self._byte_pos+8])[0]
        self._byte_pos += 8
        return value
    
    def read_bytes(self, n: int) -> bytes:
        """Read n raw bytes."""
        self._align_to_byte()
        if self._byte_pos + n > len(self._buffer):
            raise EOFError("Unexpected end of data")
        value = self._buffer[self._byte_pos:self._byte_pos+n]
        self._byte_pos += n
        return value
    
    def read_bool(self) -> bool:
        """Read a single bit as boolean."""
        if self._byte_pos >= len(self._buffer):
            raise EOFError("Unexpected end of data")
        
        byte = self._buffer[self._byte_pos]
        value = (byte >> self._bit_pos) & 1
        
        self._bit_pos += 1
        if self._bit_pos == 8:
            self._bit_pos = 0
            self._byte_pos += 1
        
        return bool(value)
    
    def _align_to_byte(self) -> None:
        """Align to next byte boundary if not already aligned."""
        if self._bit_pos != 0:
            self._bit_pos = 0
            self._byte_pos += 1
