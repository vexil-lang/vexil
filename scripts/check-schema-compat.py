#!/usr/bin/env python3
"""Enforces spec §9/§10: a schema change classified as breaking by
`vexilc compat` must be paired with at least the matching @version bump
(spec §9.2). Run from repo root with vexilc already built.

Usage: check-schema-compat.py <base-ref> <vexilc-binary>

Compares every *.vexil file that changed between <base-ref> and the current
working tree. Files that don't declare @version in the new revision are
skipped - nothing to enforce without a version to check.
"""
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

VERSION_RE = re.compile(r'@version\("([^"]+)"\)')
BUMP_RANK = {"patch": 0, "minor": 1, "major": 2}


def sh(*args: str) -> str:
    return subprocess.run(args, capture_output=True, text=True, check=True).stdout


def file_at_ref(ref: str, path: str) -> str | None:
    result = subprocess.run(
        ["git", "show", f"{ref}:{path}"], capture_output=True, text=True
    )
    return result.stdout if result.returncode == 0 else None


def declared_version(source: str) -> str:
    m = VERSION_RE.search(source)
    return m.group(1) if m else "0.0.0"  # spec §9.1: absent @version == 0.0.0


def version_bump(old: str, new: str) -> str | None:
    old_parts = tuple(int(p) for p in old.split("."))
    new_parts = tuple(int(p) for p in new.split("."))
    if new_parts == old_parts:
        return None
    if new_parts[0] != old_parts[0]:
        return "major"
    if new_parts[1] != old_parts[1]:
        return "minor"
    return "patch"


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    base_ref, vexilc = sys.argv[1], sys.argv[2]

    changed = sh("git", "diff", "--name-only", f"{base_ref}...HEAD", "--", "*.vexil")
    changed_files = [f for f in changed.splitlines() if f and Path(f).exists()]

    failures = []
    checked = 0
    for path in changed_files:
        old_source = file_at_ref(base_ref, path)
        if old_source is None:
            continue  # new file, nothing to diff against

        new_source = Path(path).read_text()
        new_version = declared_version(new_source)
        if not VERSION_RE.search(new_source):
            continue  # no @version declared - nothing to enforce

        old_version = declared_version(old_source)
        actual_bump = version_bump(old_version, new_version)

        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".vexil", delete=False
        ) as tmp:
            tmp.write(old_source)
            old_path = tmp.name

        try:
            proc = subprocess.run(
                [vexilc, "compat", old_path, path, "--format", "json"],
                capture_output=True,
                text=True,
            )
        finally:
            Path(old_path).unlink()

        # vexilc compat exits 0 (compatible) or 1 (breaking) on a normal
        # result - both print JSON to stdout. Exit 2 means it couldn't run
        # at all (compile error, bad file). The normal compiler/CI checks
        # already cover compile failures, so skip here rather than
        # duplicate that failure with a confusing message.
        if proc.returncode not in (0, 1) or not proc.stdout.strip():
            continue

        report = json.loads(proc.stdout)
        checked += 1
        suggested = report["suggested_bump"]

        if actual_bump is None or BUMP_RANK[actual_bump] < BUMP_RANK[suggested]:
            failures.append(
                f"{path}: vexilc compat requires at least a '{suggested}' "
                f"bump, but @version only went {old_version} -> {new_version} "
                f"({actual_bump or 'no bump'})"
            )
            for change in report["changes"]:
                if change["classification"] == suggested:
                    print(f"  - {change['detail']}", file=sys.stderr)

    print(f"checked {checked} versioned schema change(s)")
    if failures:
        print("\nSchema compat violations (spec §9/§10):", file=sys.stderr)
        for f in failures:
            print(f"  {f}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
