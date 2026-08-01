"""Vexil runtime for Python - binary serialization support."""

__version__ = "0.1.0"

from .bitio import (
    BitWriter,
    BitReader,
    _BitWriter,
    _BitReader,
    EncodeError,
    DecodeError,
    MAX_RECURSION_DEPTH,
    MAX_BYTES_LENGTH,
    MAX_LENGTH_PREFIX_BYTES,
)
from .pack import Pack, Unpack, pack, unpack

__all__ = [
    "BitWriter",
    "BitReader",
    "_BitWriter",
    "_BitReader",
    "Pack",
    "Unpack",
    "pack",
    "unpack",
    "EncodeError",
    "DecodeError",
    "MAX_RECURSION_DEPTH",
    "MAX_BYTES_LENGTH",
    "MAX_LENGTH_PREFIX_BYTES",
]
