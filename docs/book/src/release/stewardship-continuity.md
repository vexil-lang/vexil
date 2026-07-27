# Named Stewardship Continuity

> Generated view of [`release/stewardship/assignments.json`](../../../../release/stewardship/assignments.json). The JSON assignment record is canonical; this Markdown is non-authoritative and parity-checked.

## Reviewed public decision

Decision `sole-maintainer-governance-2026-07-23` is effective from 2026-07-23 and has status **sole-maintainer-governance**. Its authoritative review evidence is [GitHub issue #75](https://github.com/vexil-lang/vexil/issues/75).

## Current primary assignments

| Role | Primary | Scope |
|---|---|---|
| release-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `release-manifest-lifecycle` |
| repository-administrator | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `.` |
| security-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `.` |
| release-run-coordinator | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `release-run-execution` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `crates/vexil-lang` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `crates/vexilc` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `crates/vexil-runtime` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `crates/vexil-codegen-rust` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `crates/vexil-codegen-ts` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `crates/vexil-codegen-go` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `crates/vexil-codegen-py` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `crates/vexil-store` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `packages/runtime-ts` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `packages/runtime-py` |
| package-steward | Furkan Mamuk ([github.com/furkanmamuk](https://github.com/furkanmamuk), mamukfurkan@outlook.com) | `packages/runtime-go` |

Each row is an independently auditable role assertion. Combining these assignments does not union permissions: every action remains constrained by the explicit role assertion in the [Stewardship Authority Model](./stewardship.md).

## Sole-maintainer policy

No distinct recovery custodian is designated by the reviewed sole-maintainer policy. The unavailable-owner route is containment or documented succession only: it may stop, revoke, contain, or activate succession, but cannot create release authority, move tags, overwrite artifacts, rewrite evidence, accept risk, or declare completion.

## Recovery contact route

No successor is currently designated. Record containment and request a reviewed successor through [the public decision route](https://github.com/vexil-lang/vexil/issues/new/choose); this route grants no recovery, Manifest, or publication authority.

## Future target readiness

**Manifest approval: blocked. Privileged publication: blocked.** Vexil has completed a separately retained, target-scoped runtime-go v0.1.1 protected-tag release. For other targets and future privileged operations, external controls, registry identity and custody, Release Set selection, security disposition, rehearsal, and closeout evidence remain unresolved; the sole-maintainer policy is not a release gate.

These statuses describe future targets and reusable privileged procedures, not the retained Go closeout; see [Current Release Status](./current-status.md). If a second qualified Release Steward is recorded, detached approval by an identity distinct from the Manifest approver becomes mandatory; provider self-review settings alone are not evidence. A future [release-continuity-runbook](#future-runbook) is reserved for the unavailable-owner and succession procedure.

## Future runbook

The stable identifier `release-continuity-runbook` is reserved for the public unavailable-owner and succession runbook. It does not create a custodian or authorize a release.

## Validation

From a clean public checkout, run:

```sh
cargo run --manifest-path release/validator/Cargo.toml --offline -- --root .
```

The validator checks the authority contract, public role assignments, every currently maintained Package Steward root, documentation parity, and the remaining fail-closed publication gates. It does not change provider settings or create a release.

This decision preserves the BDFL, RFC, and breaking-change rules in [GOVERNANCE.md](../../../../GOVERNANCE.md).
