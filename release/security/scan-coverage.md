# Security scan coverage

`release/security/inventory.toml` is the authoritative local inventory. It
enumerates Cargo, npm (including development dependencies), Python, Go, and
GitHub Actions. A `blocking-unknown` surface is deliberately not a passing
security result and prevents a release claim until retained scanner evidence
is available.

The TypeScript runtime has current retained scanner evidence at
`release/security/scans/npm-runtime-ts-2026-07-26.json`. The selected Go
runtime has source-mode, symbol-level `govulncheck` evidence at
`release/security/scans/go-runtime-go-2026-07-26.md`. GitHub Actions has
retained static-analysis evidence at
`release/security/scans/github-actions-2026-07-26.md`.

Cargo and Python remain coverage gaps, not exclusions: each has an owner,
update path, cadence, scanner command, and review date. Python's unlocked
build backend remains a visible candidate and security blocker; it is not
treated as an empty dependency graph.
