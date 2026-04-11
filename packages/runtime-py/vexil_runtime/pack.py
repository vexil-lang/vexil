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
