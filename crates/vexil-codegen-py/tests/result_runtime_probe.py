"""Verify generated Python Result discriminants against the wire contract."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType


WORKSPACE_ROOT = Path(__file__).resolve().parents[3]
RUNTIME_ROOT = WORKSPACE_ROOT / "packages" / "runtime-py"
GOLDEN_PATH = Path(__file__).resolve().parent / "golden" / "005_parameterized.py"


def load_generated_module() -> ModuleType:
    sys.path.insert(0, str(RUNTIME_ROOT))
    spec = importlib.util.spec_from_file_location("generated_result_probe", GOLDEN_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load generated module from {GOLDEN_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main() -> None:
    generated = load_generated_module()
    ok = generated.Basic(None, [], {}, (True, 42))
    ok_bytes = ok.encode()
    assert ok_bytes == b"\x00\x00\x00\x01\x2a\x00\x00\x00"
    assert generated.Basic.decode(ok_bytes).d == (True, 42)

    failed = generated.Basic(None, [], {}, (False, "oops"))
    failed_bytes = failed.encode()
    assert failed_bytes == b"\x00\x00\x00\x00\x04oops"
    assert generated.Basic.decode(failed_bytes).d == (False, "oops")


if __name__ == "__main__":
    main()
