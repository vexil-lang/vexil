"""Verify generated recursive codecs enforce the normative depth limit."""

from __future__ import annotations

import importlib.util
import struct
import sys
from pathlib import Path
from types import ModuleType


WORKSPACE_ROOT = Path(__file__).resolve().parents[3]
RUNTIME_ROOT = WORKSPACE_ROOT / "packages" / "runtime-py"
GOLDEN_PATH = Path(__file__).resolve().parent / "golden" / "023_recursive_depth.py"


def load_generated_module() -> ModuleType:
    sys.path.insert(0, str(RUNTIME_ROOT))
    spec = importlib.util.spec_from_file_location("generated_recursion_probe", GOLDEN_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load generated module from {GOLDEN_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def linked_list(generated: ModuleType, depth: int) -> object:
    value = None
    for item in reversed(range(depth)):
        value = generated.LinkedList(item, value)
    return value


def linked_list_bytes(depth: int) -> bytes:
    parts = []
    for item in range(depth):
        parts.append(struct.pack("<q", item))
        parts.append(b"\x01" if item + 1 < depth else b"\x00")
    return b"".join(parts)


def main() -> None:
    generated = load_generated_module()
    from vexil_runtime import DecodeError, EncodeError

    depth_64 = linked_list(generated, 64)
    encoded_64 = depth_64.encode()
    assert encoded_64 == linked_list_bytes(64)
    assert generated.LinkedList.decode(encoded_64).value == 0

    try:
        linked_list(generated, 65).encode()
    except EncodeError:
        pass
    else:
        raise AssertionError("depth 65 encode must fail")

    try:
        generated.LinkedList.decode(linked_list_bytes(65))
    except DecodeError:
        pass
    else:
        raise AssertionError("depth 65 decode must fail")


if __name__ == "__main__":
    main()
