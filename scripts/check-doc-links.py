#!/usr/bin/env python3
"""Validate local links and anchors in public Markdown documentation."""

from __future__ import annotations

from collections import defaultdict
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
PRIVATE_PREFIXES = ("internal/", "docs/superpowers/")
PRIVATE_FILES = {
    "AGENTS.md",
    "CLAUDE.md",
    "MAINTAINER_HANDBOOK.md",
    "RELEASING.md",
}
MARKDOWN_LINK = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
HTML_LINK = re.compile(r"(?:href|src)=[\"']([^\"']+)[\"']", re.IGNORECASE)
HEADING = re.compile(r"^#{1,6}\s+(.+?)\s*$")
EXPLICIT_ID = re.compile(r"\bid=[\"']([^\"']+)[\"']", re.IGNORECASE)


def public_markdown_files() -> list[Path]:
    output = subprocess.run(
        ["git", "ls-files", "*.md"],
        cwd=ROOT,
        check=True,
        text=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
    ).stdout
    files: list[Path] = []
    for relative in output.splitlines():
        normalized = relative.replace("\\", "/")
        if normalized in PRIVATE_FILES or normalized.startswith(PRIVATE_PREFIXES):
            continue
        files.append(ROOT / relative)
    return files


def slug(text: str) -> str:
    text = re.sub(r"<[^>]+>", "", text)
    text = re.sub(r"[`*_~]", "", text).strip().lower()
    text = re.sub(r"[^\w\- ]", "", text, flags=re.UNICODE)
    return re.sub(r"\s+", "-", text)


def anchors(path: Path) -> set[str]:
    found: set[str] = set()
    duplicates: defaultdict[str, int] = defaultdict(int)
    for line in path.read_text(encoding="utf-8").splitlines():
        heading = HEADING.match(line)
        if heading:
            base = slug(heading.group(1))
            index = duplicates[base]
            duplicates[base] += 1
            found.add(base if index == 0 else f"{base}-{index}")
        found.update(EXPLICIT_ID.findall(line))
    return found


def markdown_target(source: Path, raw: str) -> tuple[Path | None, str | None]:
    target = raw.strip()
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1]
    if target.startswith(("http://", "https://", "mailto:", "data:")):
        return None, None
    target = target.split(maxsplit=1)[0]
    path_text, _, fragment = target.partition("#")
    fragment = unquote(fragment) if fragment else None
    if not path_text:
        return source, fragment
    decoded = unquote(path_text)
    candidate = (
        (ROOT / decoded.lstrip("/"))
        if decoded.startswith("/")
        else (source.parent / decoded)
    ).resolve()
    if candidate.suffix == ".html" and not candidate.exists():
        candidate = candidate.with_suffix(".md")
    if candidate.is_dir():
        readme = candidate / "README.md"
        if readme.exists():
            candidate = readme
    return candidate, fragment


def main() -> int:
    errors: list[str] = []
    cache: dict[Path, set[str]] = {}
    for source in public_markdown_files():
        text = source.read_text(encoding="utf-8")
        links = MARKDOWN_LINK.findall(text) + HTML_LINK.findall(text)
        for raw in links:
            target, fragment = markdown_target(source, raw)
            if target is None:
                continue
            relative_source = source.relative_to(ROOT)
            if not target.exists():
                errors.append(f"{relative_source}: missing target {raw}")
                continue
            if fragment and target.suffix.lower() == ".md":
                available = cache.setdefault(target, anchors(target))
                if fragment not in available:
                    errors.append(f"{relative_source}: missing anchor {raw}")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"documentation links: {len(public_markdown_files())} files checked")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
