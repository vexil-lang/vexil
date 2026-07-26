# Go runtime vulnerability scan — 2026-07-26

Scope: `github.com/vexil-lang/vexil/packages/runtime-go` (`./...`).

Command:

```text
govulncheck ./...
```

Result: `No vulnerabilities found.`

Scanner: `govulncheck@v1.6.0` with Go `go1.26.5 windows/amd64`.
The scanner used `https://vuln.go.dev`, last updated `2026-07-24 18:35:55 UTC`.

This is source-mode, symbol-level evidence for the Go module. It does not
assert security for unrelated workspace, Python, npm, or GitHub Actions
surfaces.
