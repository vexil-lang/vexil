# Changelog

All notable changes to this project will be documented in this file.



## 0.1.0 (2026-03-27)

Initial release.

- `encode` / `decode` — schema-driven bitpack encoder and decoder for `Value` trees
- `Value` — dynamically typed value covering all Vexil primitives, composites, and semantic types
- `.vx` text format — human-readable parse and format via `parse()` and `format()`
- `.vxb` binary format — typed file header with magic bytes, format version, schema hash, namespace, and schema version; compressed variant `VXBP` supported
- `detect_format()` — identifies `.vx` vs `.vxb` from file content
- `meta_schema()` / `pack_schema()` — pre-compiled `vexil.schema` and `vexil.pack` schemas for encoding Vexil IR as first-class values
- Wire golden tests — committed exact byte sequences prove cross-platform encoding interoperability on linux x86-64, linux arm64, windows, and macOS arm64
- Signed sub-byte encoding correct — `iN` fields use two's complement in exactly N bits
