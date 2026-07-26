# runtime-go 0.1.1 compatibility evidence

Scope: the `vexil-runtime-go` unit selected for rehearsal by
`first-recovered-release-set-2026-07-26`, bound to source commit
`99e2afef4d48ab35ec61e8b23a9c0d6c210e275f`.

This is compatibility evidence for the selected Go runtime only. It is not a
formal language-conformance claim, a Release Manifest, approval, Run, tag,
registry publication, deployment, or assertion that the module is publicly
available at `0.1.1`.

## Inputs

- The formal specification is `spec/vexil-spec.md`; the wire schema is
  `schemas/vexil/schema.vexil`.
- The selected source declares `0.1.1` in `packages/runtime-go/VERSION` and
  `github.com/vexil-lang/vexil/packages/runtime-go` in its `go.mod`.
- Package behavior and limits are documented in `packages/runtime-go/README.md`.
- The release rationale is `vexil-runtime-go-0-1-1`; the source-mode Go
  vulnerability scan is `go-runtime-go-2026-07-26`.

## Observed test result

At the scoped source revision, the following command completed successfully:

```text
go version go1.26.5 windows/amd64
cd packages/runtime-go
go test -count=1 ./...
ok github.com/vexil-lang/vexil/packages/runtime-go 0.814s
```

The test suite executes the primitive, sub-byte, message, optional, enum,
union, arrays/maps, evolution, and delta vector files through
`compliance_test.go` and `delta_compliance_test.go`, alongside runtime API and
handshake tests. `compliance/vectors/v1_types.json` is intentionally not part
of this Go-runtime evidence set because the Go test suite does not execute it;
this record makes no claim for that vector surface.

## Support interpretation

The included evidence supports only the existing Go runtime's documented
Go 1.22+ API and its exercised wire-vector behavior. It does not elevate the
draft language specification to a conformance guarantee, establish support for
unselected Rust, TypeScript, or Python surfaces, or replace an RFC or public
compatibility decision for any behavior change.
