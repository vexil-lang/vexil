# Current Release Status

Vexil's release practice keeps completed work, future work, and reusable
automation distinct. The records below are public evidence; they do not turn a
single outcome into universal publication readiness.

## Completed scoped outcome

`vexil-runtime-go` `v0.1.1` was completed as the manually protected canonical
tag [`packages/runtime-go/v0.1.1`](../../../../release/closeouts/runtime-go-0-1-1-manual-protected-tag-2026-07-26.json).
The retained [closeout](../../../../release/closeouts/runtime-go-0-1-1-manual-protected-tag-2026-07-26.json),
[Manifest](../../../../release/manifests/runtime-go-0-1-1-release-2026-07-26/manifest.json),
and [Go proxy observation](../../../../release/history/observations/observation-go-runtime-v0-1-1-publication-2026-07-26.json)
bind the exact tag, source commit, and public availability.

This outcome does **not** claim a GitHub Release, registry upload credential,
deployment, bot action, workflow Run, or readiness for any other Vexil target.

## Other targets and reusable procedures

| Scope | Current status | What remains separate |
|---|---|---|
| crates.io units | Not selected for this completed Go release. | Each unit needs its own target identity, custody, evidence, and approved release record. |
| npm runtime | Not selected. | Source/public version drift and target custody remain to be resolved. |
| Python runtime | Not selected. | Exact PyPI identity and trusted-publisher custody remain to be resolved. |
| GitHub Releases, deployments, and documentation deployment | Not part of the Go closeout. | They retain their own target-specific authority and evidence boundaries. |
| Reusable release and registry automation | Intentionally effect-disabled. | The named prerequisites in its runbook must be satisfied before a future automated effect. |

The preserved [First Recovered Release Set](./first-recovered-release-set.md)
is pre-effect decision context. It is not rewritten by the later closeout.
For source inventory rather than publication status, see the
[Release Unit Catalog](./catalog.md).
