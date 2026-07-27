# First Recovered Release Set

> Generated view of [`release/decisions/first-recovered-release-set-2026-07-26.json`](../../../../release/decisions/first-recovered-release-set-2026-07-26.json). The JSON selection is canonical; this Markdown is non-authoritative and parity-checked.

## Preserved pre-effect decision

Selection `first-recovered-release-set-2026-07-26` is `rehearsal-selected` at exact source commit `99e2afef4d48ab35ec61e8b23a9c0d6c210e275f`. It records the small rehearsal selection that preceded the separately retained Go closeout; it is not rewritten to claim that later outcome. The selection itself is not a Release Manifest, approval, Run, tag, registry action, deployment, or publication assertion.

## Included unit

| Unit | Source commit | Version | Canonical tag | Mandatory target | Clean-consumer plan |
|---|---|---|---|---|---|
| `vexil-runtime-go` | `99e2afef4d48ab35ec61e8b23a9c0d6c210e275f` | `0.1.1` | `packages/runtime-go/v0.1.1` | `go-module` `github.com/vexil-lang/vexil/packages/runtime-go` | After the approved Run, retrieve github.com/vexil-lang/vexil/packages/runtime-go@v0.1.1 from a clean Go module cache, run go test, verify the resolved module version, and record proxy propagation separately. |

## Explicitly excluded publishable units

| Unit | Reason | Next action |
|---|---|---|
| `vexil-codegen-go` | crates.io identity and custody are not established for this exact package. | Establish target-scoped custody and a separately evidenced rationale. |
| `vexil-codegen-py` | crates.io identity and custody are not established for this exact package. | Establish target-scoped custody and a separately evidenced rationale. |
| `vexil-codegen-rust` | crates.io identity and custody are not established for this exact package. | Establish target-scoped custody and a separately evidenced rationale. |
| `vexil-codegen-ts` | crates.io identity and custody are not established for this exact package. | Establish target-scoped custody and a separately evidenced rationale. |
| `vexil-lang` | crates.io identity and custody are not established for this exact package. | Establish target-scoped custody and a separately evidenced rationale. |
| `vexil-runtime` | crates.io identity and custody are not established for this exact package. | Establish target-scoped custody and a separately evidenced rationale. |
| `vexil-runtime-py` | Exact PyPI project identity and trusted-publisher custody remain unresolved. | Resolve the PyPI identity before selecting a Python target. |
| `vexil-runtime-ts` | The checked-in version and public npm history remain divergent. | Resolve source/public version drift and target custody. |
| `vexil-store` | crates.io identity and dependency release order are not established for this exact package. | Establish target custody and dependency evidence. |
| `vexilc` | crates.io identity and every publish-before dependency target are not established. | Establish target custody and dependency evidence. |

## Effect boundary

`selection-only; no Manifest, approval, Run, tag, registry, deployment, or publication authority`

Offline validation confirms structural bindings and this generated view only. It does not prove live external controls or authorize an effect. For the later, target-scoped outcome, see [Current Release Status](./current-status.md).
