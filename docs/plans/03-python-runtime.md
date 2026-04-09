# Python Runtime Library Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this task.

**Goal:** Create and publish `vexil_runtime` Python package on PyPI for Python codegen support.

**Architecture:** Lightweight Python package with Pack/Unpack using struct module, dataclass support helpers.

**Tech Stack:** Python, setuptools, PyPI

---

## Current State

- Python codegen produces @dataclass code using only stdlib
- No PyPI package exists for runtime helpers
- Users must manually handle encoding or use raw struct module

## Target State

- `pip install vexil_runtime` works
- Package provides BitWriter/BitReader, Pack/Unpack protocols
- Dataclass helpers for common operations

---

## Task 1: Create Python Runtime Package Structure

**Objective:** Set up the package structure in a new repo or subdirectory.

**Files:**
- Create: `runtimes/python/vexil_runtime/__init__.py`
- Create: `runtimes/python/vexil_runtime/bitio.py`
- Create: `runtimes/python/vexil_runtime/pack.py`
- Create: `runtimes/python/pyproject.toml`

**Step 1: Create directory structure**

```bash
mkdir -p runtimes/python/vexil_runtime
```

**Step 2: Create __init__.py**

```python
"""Vexil runtime for Python - binary serialization support."""

__version__ = "0.1.0"

from .bitio import BitWriter, BitReader
from .pack import Pack, Unpack, pack, unpack

__all__ = [
    "BitWriter",
    "BitReader",
    "Pack",
    "Unpack",
    "pack",
    "unpack",
]
```

**Step 3: Create bitio.py**

```python
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
        byte_pos = len(self._buffer)
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
```

**Step 4: Create pack.py**

```python
"""Pack/Unpack protocol for Vexil types."""

from typing import Protocol, runtime_checkable, TypeVar
from .bitio import BitWriter, BitReader

T = TypeVar('T')

@runtime_checkable
class Pack(Protocol):
    """Protocol for types that can be packed to bytes."""
    
    def pack(self, writer: BitWriter) -> None:
        ...

@runtime_checkable  
class Unpack(Protocol[T]):
    """Protocol for types that can be unpacked from bytes."""
    
    @classmethod
    def unpack(cls, reader: BitReader) -> T:
        ...

def pack(obj: Pack) -> bytes:
    """Pack an object to bytes."""
    w = BitWriter()
    obj.pack(w)
    return w.finish()

def unpack(cls: type[T], data: bytes) -> T:
    """Unpack bytes to an object of the given class."""
    r = BitReader(data)
    return cls.unpack(r)
```

**Step 5: Create pyproject.toml**

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "vexil_runtime"
version = "0.1.0"
description = "Vexil runtime for Python - binary serialization support"
readme = "README.md"
license = {text = "MIT"}
requires-python = ">=3.10"
classifiers = [
    "Development Status :: 3 - Alpha",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Programming Language :: Python :: 3.13",
    "Topic :: Software Development :: Libraries",
]

[project.urls]
Homepage = "https://github.com/vexil-lang/vexil"
Documentation = "https://vexil-lang.github.io/vexil/"
Repository = "https://github.com/vexil-lang/vexil.git"
"Bug Tracker" = "https://github.com/vexil-lang/vexil/issues"

[tool.hatch.build.targets.wheel]
packages = ["vexil_runtime"]
```

**Step 6: Test locally**

```bash
cd runtimes/python
pip install -e .
python -c "from vexil_runtime import BitWriter, BitReader; print('OK')"
```

**Step 7: Commit**

```bash
git add runtimes/python/
git commit -m "feat: add Python runtime package with BitWriter/BitReader"
```

---

## Task 2: Publish to PyPI

**Objective:** Build and publish the package.

**Step 1: Build distribution**

```bash
cd runtimes/python
pip install build twine
python -m build
```

**Step 2: Upload to PyPI**

```bash
python -m twine upload dist/*
```

**Step 3: Verify installation**

```bash
pip install vexil_runtime
python -c "import vexil_runtime; print(vexil_runtime.__version__)"
```

---

**Summary:** Create Python runtime package with BitWriter/BitReader and Pack/Unpack protocols, publish to PyPI.
