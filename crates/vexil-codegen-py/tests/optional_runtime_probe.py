"""Verify generated optional codecs against the normative bit layout."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType


WORKSPACE_ROOT = Path(__file__).resolve().parents[3]
RUNTIME_ROOT = WORKSPACE_ROOT / "packages" / "runtime-py"
GOLDEN_PATH = Path(__file__).resolve().parent / "golden" / "021_empty_optionals.py"


def load_generated_module() -> ModuleType:
    sys.path.insert(0, str(RUNTIME_ROOT))
    spec = importlib.util.spec_from_file_location("generated_optional_probe", GOLDEN_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load generated module from {GOLDEN_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main() -> None:
    generated = load_generated_module()

    absent = generated.NestedOptional(None)
    present_empty = generated.NestedOptional((None,))
    present_value = generated.NestedOptional((42,))

    assert absent.encode() == b"\x00"
    assert present_empty.encode() == b"\x01"
    assert present_value.encode() == b"\x03\x2a\x00\x00\x00"

    assert generated.NestedOptional.decode(absent.encode()).inner is None
    assert generated.NestedOptional.decode(present_empty.encode()).inner == (None,)
    assert generated.NestedOptional.decode(present_value.encode()).inner == (42,)

    # Adjacent absent presence bits stay packed into one byte.
    all_empty = generated.AllEmpty(None, None, None)
    assert all_empty.encode() == b"\x00"
    assert generated.AllEmpty.decode(all_empty.encode()) == all_empty


if __name__ == "__main__":
    main()
