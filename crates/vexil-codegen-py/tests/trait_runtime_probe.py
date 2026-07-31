"""Import a generated trait whose type parameter matches a typing helper."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType


GOLDEN_PATH = (
    Path(__file__).resolve().parent
    / "golden"
    / "generic_trait_type_param_conflicts.py"
)


def load_generated_module() -> ModuleType:
    spec = importlib.util.spec_from_file_location("generated_trait_probe", GOLDEN_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load generated module from {GOLDEN_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main() -> None:
    generated = load_generated_module()

    class ImplementsWrapper:
        value = 7

    assert isinstance(ImplementsWrapper(), generated.Wrapper)


if __name__ == "__main__":
    main()
