"""Verify generated non-exhaustive union preservation and bounds."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType


WORKSPACE_ROOT = Path(__file__).resolve().parents[3]
RUNTIME_ROOT = WORKSPACE_ROOT / "packages" / "runtime-py"
GOLDEN_PATH = (
    Path(__file__).resolve().parent
    / "golden"
    / "050_non_exhaustive_union_unknown_collision.py"
)


def load_generated_module() -> ModuleType:
    sys.path.insert(0, str(RUNTIME_ROOT))
    spec = importlib.util.spec_from_file_location("generated_union_probe", GOLDEN_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load generated module from {GOLDEN_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main() -> None:
    generated = load_generated_module()
    value = generated.Event__VexilUnknown(9, b"\xde\xad")
    encoded = value.encode()
    assert encoded == b"\x09\x02\xde\xad"

    decoded = generated.decode_Event(encoded)
    assert isinstance(decoded, generated.Event__VexilUnknown)
    assert decoded.discriminant == 9
    assert decoded.data == b"\xde\xad"
    assert decoded.encode() == encoded

    try:
        generated.decode_Event(b"\x09\x81\x80\x80\x20")
    except generated.DecodeError as error:
        assert "exceeds limit" in str(error)
    else:
        raise AssertionError("over-limit union payload was accepted")


if __name__ == "__main__":
    main()
