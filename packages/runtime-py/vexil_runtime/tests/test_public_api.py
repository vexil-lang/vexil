from dataclasses import dataclass

import vexil_runtime


@dataclass
class ByteValue:
    value: int

    def pack(self, writer: vexil_runtime.BitWriter) -> None:
        writer.write_u8(self.value)

    @classmethod
    def unpack(cls, reader: vexil_runtime.BitReader) -> "ByteValue":
        return cls(reader.read_u8())


def test_documented_public_exports_exist() -> None:
    for name in vexil_runtime.__all__:
        assert hasattr(vexil_runtime, name), name


def test_top_level_pack_and_unpack_round_trip() -> None:
    encoded = vexil_runtime.pack(ByteValue(42))
    assert encoded == b"\x2a"
    assert vexil_runtime.unpack(ByteValue, encoded) == ByteValue(42)
