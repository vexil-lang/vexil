"""Verify the public file contract for vexil-runtime distributions."""

from __future__ import annotations

import sys
import tarfile
import zipfile
from pathlib import Path


PACKAGE_FILES = {
    "vexil_runtime/__init__.py",
    "vexil_runtime/bitio.py",
    "vexil_runtime/pack.py",
}
SDIST_FILES = {
    *PACKAGE_FILES,
    ".gitignore",
    "LICENSE-APACHE",
    "LICENSE-MIT",
    "PKG-INFO",
    "README.md",
    "pyproject.toml",
}


def wheel_files(path: Path) -> set[str]:
    with zipfile.ZipFile(path) as archive:
        return set(archive.namelist())


def sdist_files(path: Path) -> set[str]:
    with tarfile.open(path, "r:gz") as archive:
        files = set()
        for member in archive.getmembers():
            if not member.isfile():
                continue
            _, separator, relative = member.name.partition("/")
            if separator and relative:
                files.add(relative)
        return files


def verify_wheel(path: Path) -> list[str]:
    files = wheel_files(path)
    metadata_entries = sorted(name for name in files if name.endswith(".dist-info/METADATA"))
    if len(metadata_entries) != 1:
        return [f"{path.name}: expected one dist-info METADATA file"]

    dist_info = metadata_entries[0].removesuffix("/METADATA")
    expected = {
        *PACKAGE_FILES,
        f"{dist_info}/METADATA",
        f"{dist_info}/RECORD",
        f"{dist_info}/WHEEL",
        f"{dist_info}/licenses/LICENSE-APACHE",
        f"{dist_info}/licenses/LICENSE-MIT",
    }
    return manifest_errors(path, files, expected)


def verify_sdist(path: Path) -> list[str]:
    files = sdist_files(path)
    return manifest_errors(path, files, SDIST_FILES)


def manifest_errors(path: Path, actual: set[str], expected: set[str]) -> list[str]:
    errors = [f"{path.name}: missing {name}" for name in sorted(expected - actual)]
    errors.extend(
        f"{path.name}: unexpected {name}" for name in sorted(actual - expected)
    )
    return errors


def main(arguments: list[str]) -> int:
    paths = [Path(argument) for argument in arguments]
    wheels = [path for path in paths if path.suffix == ".whl"]
    sdists = [path for path in paths if path.name.endswith(".tar.gz")]
    recognized = set(wheels) | set(sdists)

    errors = [f"unexpected artifact input: {path}" for path in paths if path not in recognized]
    if len(wheels) != 1:
        errors.append(f"expected one wheel, found {len(wheels)}")
    if len(sdists) != 1:
        errors.append(f"expected one source distribution, found {len(sdists)}")
    for path in wheels:
        errors.extend(verify_wheel(path))
    for path in sdists:
        errors.extend(verify_sdist(path))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(f"Verified Python distribution contents: {wheels[0].name}, {sdists[0].name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
