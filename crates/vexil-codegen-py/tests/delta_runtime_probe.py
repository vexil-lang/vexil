"""Exercise generated delta codecs against the real Python runtime."""

from __future__ import annotations

import importlib.util
import math
import sys
from pathlib import Path
from types import ModuleType


WORKSPACE_ROOT = Path(__file__).resolve().parents[3]
RUNTIME_ROOT = WORKSPACE_ROOT / "packages" / "runtime-py"
GOLDEN_PATH = (
    WORKSPACE_ROOT
    / "crates"
    / "vexil-codegen-py"
    / "tests"
    / "golden"
    / "027_delta_on_message.py"
)


def load_generated_module() -> ModuleType:
    sys.path.insert(0, str(RUNTIME_ROOT))
    spec = importlib.util.spec_from_file_location("generated_delta_probe", GOLDEN_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load generated module from {GOLDEN_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def assert_telemetry(actual: object, expected: object) -> None:
    assert actual.timestamp == expected.timestamp
    assert math.isclose(actual.value, expected.value, rel_tol=1e-6, abs_tol=1e-6)
    assert actual.label == expected.label
    assert actual.count == expected.count


def main() -> None:
    generated = load_generated_module()
    first = generated.Telemetry(
        timestamp=1_000, value=1.25, label="first", count=7
    )
    second = generated.Telemetry(
        timestamp=1_009, value=1.75, label="second", count=11
    )

    encoder = generated.TelemetryEncoder()
    decoder = generated.TelemetryDecoder()

    first_bytes = encoder.encode(first)
    second_bytes = encoder.encode(second)
    assert_telemetry(decoder.decode(first_bytes), first)
    assert_telemetry(decoder.decode(second_bytes), second)

    encoder.reset()
    decoder.reset()
    reset_bytes = encoder.encode(first)
    assert reset_bytes == first_bytes
    assert_telemetry(decoder.decode(reset_bytes), first)


if __name__ == "__main__":
    main()
