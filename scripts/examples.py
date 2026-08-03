#!/usr/bin/env python3
"""Regenerate and verify the curated public examples."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ("quickstart", "project-evolution", "cross-language", "live-telemetry")


def executable(name: str) -> str:
    resolved = shutil.which(name)
    if resolved is None:
        raise RuntimeError(f"required executable not found: {name}")
    return resolved


def run(
    command: list[str],
    *,
    cwd: Path = ROOT,
    env: dict[str, str] | None = None,
    expected: tuple[int, ...] = (0,),
) -> str:
    display = " ".join(command)
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if completed.returncode not in expected:
        raise RuntimeError(
            f"command failed with {completed.returncode}: {display}\n{completed.stdout}"
        )
    return completed.stdout


def vexilc(*arguments: str) -> list[str]:
    return [
        executable("cargo"),
        "run",
        "--quiet",
        "-p",
        "vexilc",
        "--",
        *arguments,
    ]


def compare_file(actual: Path, expected: Path) -> None:
    if actual.read_bytes() != expected.read_bytes():
        raise RuntimeError(
            f"generated artifact is stale: {expected.relative_to(ROOT)}\n"
            f"run: python scripts/examples.py regenerate all"
        )


def generate_quickstart(output: Path) -> None:
    run(
        vexilc(
            "codegen",
            "examples/quickstart/telemetry.vexil",
            "--target",
            "rust",
            "--output",
            str(output),
        )
    )


def generate_project_evolution(output: Path) -> None:
    run(
        vexilc(
            "build",
            "examples/project-evolution/schemas/project/messages.vexil",
            "--include",
            "examples/project-evolution/schemas",
            "--output",
            str(output),
            "--target",
            "rust",
        )
    )


def generate_cross_language(output: Path) -> None:
    targets = {
        "rust": output / "generated.rs",
        "typescript": output / "generated.ts",
        "go": output / "generated.go",
        "python": output / "generated.py",
    }
    for target, path in targets.items():
        run(
            vexilc(
                "codegen",
                "examples/cross-language/schema/telemetry.vexil",
                "--target",
                target,
                "--output",
                str(path),
            )
        )


def generate_live_telemetry(output: Path) -> None:
    run(
        vexilc(
            "codegen",
            "examples/live-telemetry/schema/telemetry.vexil",
            "--target",
            "rust",
            "--output",
            str(output / "generated.rs"),
        )
    )
    run(
        vexilc(
            "codegen",
            "examples/live-telemetry/schema/telemetry.vexil",
            "--target",
            "typescript",
            "--output",
            str(output / "generated.ts"),
        )
    )
    node_modules = ROOT / "examples/live-telemetry/node_modules"
    if not node_modules.exists():
        raise RuntimeError(
            "live-telemetry Node dependencies are missing; run npm ci in "
            "examples/live-telemetry"
        )
    run(
        [
            executable("npm"),
            "exec",
            "--",
            "esbuild",
            "ts/index.ts",
            "--bundle",
            "--format=esm",
            f"--outfile={output / 'bundle.js'}",
        ],
        cwd=ROOT / "examples/live-telemetry",
    )


def regenerate(name: str) -> None:
    if name == "quickstart":
        generate_quickstart(ROOT / "examples/quickstart/src/generated.rs")
    elif name == "project-evolution":
        generate_project_evolution(ROOT / "examples/project-evolution/src")
    elif name == "cross-language":
        with tempfile.TemporaryDirectory(prefix="vexil-cross-regenerate-") as raw:
            output = Path(raw)
            generate_cross_language(output)
            destinations = {
                "generated.rs": ROOT / "examples/cross-language/rust-device/src/generated.rs",
                "generated.ts": ROOT / "examples/cross-language/node-dashboard/src/generated.ts",
                "generated.go": ROOT / "examples/cross-language/go-gateway/telemetry/generated.go",
                "generated.py": ROOT / "examples/cross-language/python-client/generated.py",
            }
            for filename, destination in destinations.items():
                shutil.copyfile(output / filename, destination)
    elif name == "live-telemetry":
        with tempfile.TemporaryDirectory(prefix="vexil-live-regenerate-") as raw:
            output = Path(raw)
            generate_live_telemetry(output)
            shutil.copyfile(output / "generated.rs", ROOT / "examples/live-telemetry/src/generated.rs")
            shutil.copyfile(output / "generated.ts", ROOT / "examples/live-telemetry/ts/generated.ts")
            shutil.copyfile(output / "bundle.js", ROOT / "examples/live-telemetry/static/bundle.js")
    else:
        raise RuntimeError(f"unknown example: {name}")


def check_generated(name: str) -> None:
    with tempfile.TemporaryDirectory(prefix=f"vexil-{name}-") as raw:
        output = Path(raw)
        if name == "quickstart":
            generated = output / "generated.rs"
            generate_quickstart(generated)
            compare_file(generated, ROOT / "examples/quickstart/src/generated.rs")
        elif name == "project-evolution":
            generate_project_evolution(output)
            expected_root = ROOT / "examples/project-evolution/src"
            generated_files = (
                Path("mod.rs"),
                Path("project/messages.rs"),
                Path("project/mod.rs"),
                Path("project/types.rs"),
            )
            actual_files = sorted(path.relative_to(output) for path in output.rglob("*") if path.is_file())
            if actual_files != sorted(generated_files):
                raise RuntimeError("generated project file set is stale")
            for relative in generated_files:
                compare_file(output / relative, expected_root / relative)
        elif name == "cross-language":
            generate_cross_language(output)
            destinations = {
                "generated.rs": ROOT / "examples/cross-language/rust-device/src/generated.rs",
                "generated.ts": ROOT / "examples/cross-language/node-dashboard/src/generated.ts",
                "generated.go": ROOT / "examples/cross-language/go-gateway/telemetry/generated.go",
                "generated.py": ROOT / "examples/cross-language/python-client/generated.py",
            }
            for filename, destination in destinations.items():
                compare_file(output / filename, destination)
        elif name == "live-telemetry":
            generate_live_telemetry(output)
            compare_file(output / "generated.rs", ROOT / "examples/live-telemetry/src/generated.rs")
            compare_file(output / "generated.ts", ROOT / "examples/live-telemetry/ts/generated.ts")
            compare_file(output / "bundle.js", ROOT / "examples/live-telemetry/static/bundle.js")


def check_quickstart() -> None:
    output = run(
        [
            executable("cargo"),
            "run",
            "--quiet",
            "--manifest-path",
            "examples/quickstart/Cargo.toml",
            "--locked",
        ]
    )
    for marker in ("schema:", "wire:", "round trip:"):
        if marker not in output:
            raise RuntimeError(f"quickstart output is missing {marker!r}\n{output}")


def check_project_evolution() -> None:
    run(
        [
            executable("cargo"),
            "run",
            "--quiet",
            "--manifest-path",
            "examples/project-evolution/Cargo.toml",
            "--locked",
        ]
    )
    compatible = run(
        vexilc(
            "compat",
            "examples/project-evolution/evolution/v1.vexil",
            "examples/project-evolution/evolution/v1_1.vexil",
        )
    )
    if "COMPATIBLE" not in compatible:
        raise RuntimeError(f"compatible evolution was not accepted\n{compatible}")
    breaking = run(
        vexilc(
            "compat",
            "examples/project-evolution/evolution/v1.vexil",
            "examples/project-evolution/evolution/breaking.vexil",
        ),
        expected=(1,),
    )
    if "BREAKING" not in breaking:
        raise RuntimeError(f"breaking evolution was not rejected\n{breaking}")


def parse_contract(output: str, target: str) -> tuple[str, str]:
    values: dict[str, str] = {}
    for line in output.splitlines():
        if "=" in line:
            key, value = line.strip().split("=", 1)
            values[key] = value
    if values.get("round-trip") != "ok" or "schema" not in values or "wire" not in values:
        raise RuntimeError(f"{target} did not emit the example contract\n{output}")
    return values["schema"], values["wire"]


def check_cross_language() -> None:
    node_dir = ROOT / "examples/cross-language/node-dashboard"
    if not (node_dir / "node_modules").exists():
        raise RuntimeError(
            "cross-language Node dependencies are missing; run npm ci in "
            "examples/cross-language/node-dashboard"
        )
    run([executable("npm"), "run", "--silent", "typecheck"], cwd=node_dir)
    outputs = {
        "Rust": run(
            [
                executable("cargo"),
                "run",
                "--quiet",
                "--manifest-path",
                "examples/cross-language/rust-device/Cargo.toml",
                "--locked",
            ]
        ),
        "TypeScript": run([executable("npm"), "run", "--silent", "check"], cwd=node_dir),
        "Go": run([executable("go"), "run", "."], cwd=ROOT / "examples/cross-language/go-gateway"),
    }
    python_env = os.environ.copy()
    python_env["PYTHONPATH"] = str(ROOT / "packages/runtime-py")
    outputs["Python"] = run(
        [sys.executable, "main.py"],
        cwd=ROOT / "examples/cross-language/python-client",
        env=python_env,
    )
    contracts = {target: parse_contract(output, target) for target, output in outputs.items()}
    if len(set(contracts.values())) != 1:
        details = "\n".join(f"{target}: {contract}" for target, contract in contracts.items())
        raise RuntimeError(f"cross-language outputs differ\n{details}")
    print("cross-language: Rust, TypeScript, Go, and Python agree")


def check_live_telemetry() -> None:
    output = run(
        [
            executable("cargo"),
            "run",
            "--quiet",
            "--manifest-path",
            "examples/live-telemetry/Cargo.toml",
            "--locked",
            "--",
            "--self-test",
        ]
    )
    if "self-test:" not in output:
        raise RuntimeError(f"live telemetry self-test did not complete\n{output}")


def check(name: str) -> None:
    check_generated(name)
    if name == "quickstart":
        check_quickstart()
    elif name == "project-evolution":
        check_project_evolution()
    elif name == "cross-language":
        check_cross_language()
    elif name == "live-telemetry":
        check_live_telemetry()
    print(f"{name}: ok")


def selections(name: str) -> tuple[str, ...]:
    return EXAMPLES if name == "all" else (name,)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("check", "regenerate"))
    parser.add_argument("example", choices=("all", *EXAMPLES), nargs="?", default="all")
    arguments = parser.parse_args()
    try:
        for name in selections(arguments.example):
            if arguments.action == "check":
                check(name)
            else:
                regenerate(name)
                print(f"{name}: regenerated")
    except RuntimeError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
