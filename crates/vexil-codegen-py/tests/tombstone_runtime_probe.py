"""Verify typed tombstones cause no generated Python codec operations."""

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


def current_payload() -> bytes:
    from vexil_runtime import BitWriter

    writer = BitWriter()
    writer.write_u32(0x12345678)
    return writer.finish()


def main() -> None:
    generated = load_generated_module()
    expected = current_payload()
    decoded = generated.LegacyShapes.decode(expected)
    assert decoded.current == 0x12345678
    assert decoded.unknown == b""
    assert decoded.encode() == expected


if __name__ == "__main__":
    main()
