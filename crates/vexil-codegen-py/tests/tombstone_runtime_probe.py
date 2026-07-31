"""Verify generated typed tombstones consume every legacy wire shape."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType


WORKSPACE_ROOT = Path(__file__).resolve().parents[3]
RUNTIME_ROOT = WORKSPACE_ROOT / "packages" / "runtime-py"
GOLDEN_PATH = (
    Path(__file__).resolve().parent / "golden" / "typed_tombstone_shapes.py"
)


def load_generated_module() -> ModuleType:
    sys.path.insert(0, str(RUNTIME_ROOT))
    spec = importlib.util.spec_from_file_location("generated_tombstone_probe", GOLDEN_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load generated module from {GOLDEN_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def legacy_payload() -> bytes:
    from vexil_runtime import BitWriter

    writer = BitWriter()
    writer.write_bytes(b"old")
    writer.write_leb128(2)
    writer.write_u16(10)
    writer.write_u16(20)
    writer.write_u8(1)
    writer.write_u8(2)
    writer.write_u8(3)
    writer.write_f32(1.0)
    writer.write_f32(2.0)
    writer.write_f32(3.0)
    writer.write_bits(0b101, 3)
    writer.write_u32(0x12345678)
    return writer.finish()


def main() -> None:
    generated = load_generated_module()
    decoded = generated.LegacyShapes.decode(legacy_payload())
    assert decoded.current == 0x12345678
    assert decoded.unknown == b""


if __name__ == "__main__":
    main()
