# Security scan coverage

`release/security/inventory.toml` is the authoritative local inventory. It
enumerates Cargo, npm (including development dependencies), Python, Go, and
GitHub Actions. A `blocking-unknown` surface is deliberately not a passing
security result and prevents a release claim until retained scanner evidence
is available.

The TypeScript runtime has current retained scanner evidence at
`release/security/scans/npm-runtime-ts-2026-07-25.json`. The Cargo, Python,
Go, and GitHub Actions rows are coverage gaps, not exclusions: each has an
owner, update path, cadence, scanner command, and review date. Python's
unlocked build backend and Go's absent module checksum are visible candidate
and security blockers; neither is treated as an empty dependency graph.
