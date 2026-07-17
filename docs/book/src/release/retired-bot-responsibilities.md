# Retired-Bot Responsibility Inventory

> Generated view of [`release/stewardship/responsibilities.json`](../../../../release/stewardship/responsibilities.json). The JSON inventory is canonical; this Markdown is non-authoritative and parity-checked.

The retired [`.vexilbot.toml`](../../../../.vexilbot.toml) is historical evidence only: it is **not an order or Release Unit membership source**. Advisory responsibilities have exactly one public disposition; privileged and policy responsibilities have exactly one owned fail-closed procedure and remain blocked pending later controls.

## Inventory

| ID | Responsibility | Privilege class | Failure impact | Decision owner | Status |
|---|---|---|---|---|---|
| `RBR-001` | Prepare a release candidate, changelog input, and CI preconditions before a human approval route. | advisory | A release can be prepared without recorded checks or a coherent changelog input. | github:furkanmamuk | owned-manual-procedure |
| `RBR-002` | Identify and communicate package dependency ordering for a prospective release without deriving authoritative membership from retired configuration. | advisory | A later release can silently omit or incorrectly order a maintained unit. | github:furkanmamuk | owned-manual-procedure |
| `RBR-003` | Interpret release tag formats and prevent unreviewed tag-driven release effects. | privileged | An incorrect tag may initiate an unintended build or public announcement path. | github:furkanmamuk | owned-fail-closed-procedure |
| `RBR-004` | Coordinate registry and release publication only after separately authorized approval and external-control gates. | privileged | A package, artifact, or release record can be published without complete custody and recovery proof. | github:furkanmamuk | owned-fail-closed-procedure |
| `RBR-005` | Triage incoming issues and pull requests into a maintainer review path. | advisory | Contributor reports can remain untriaged or be handled without an accountable route. | github:furkanmamuk | maintained-replacement |
| `RBR-006` | Apply consistent path and keyword labels to issues and pull requests as advisory metadata. | advisory | Work can lose routing context and maintainers can miss affected ownership surfaces. | github:furkanmamuk | maintained-replacement |
| `RBR-007` | Acknowledge first pull requests and issues without representing an approval or support commitment. | advisory | New contributors receive no expected-review or reproduction guidance. | github:furkanmamuk | approved-retirement |
| `RBR-008` | Warn when proposed changes touch RFC-required or wire-format-sensitive paths. | policy | A compatibility-sensitive change can bypass the public RFC and review route. | github:furkanmamuk | owned-fail-closed-procedure |
| `RBR-009` | Preserve the human-operable knowledge needed when retired automation, CI, or provider state cannot be trusted as authority. | policy | A maintainer may confuse historical automation or green CI with authority and lack a documented manual fallback. | github:furkanmamuk | owned-fail-closed-procedure |

## Manifest comparison

Current publishable manifest units are compared with the retired configuration without treating that configuration as authority.

| Mismatch ID | Unit | Observed historical gap |
|---|---|---|
| `RBR-MISMATCH-001` | `crates/vexil-codegen-py` | The workspace manifest declares the publishable vexil-codegen-py crate, but .vexilbot.toml has no release.crates.vexil-codegen-py entry. |
| `RBR-MISMATCH-002` | `packages/runtime-go` | packages/runtime-go has a Go module manifest, but .vexilbot.toml has no package or dependency-order entry for it. |
| `RBR-MISMATCH-003` | `packages/runtime-py` | packages/runtime-py has a Python project manifest, but .vexilbot.toml has no package or dependency-order entry for it. |

## Evidence and use

Each canonical item carries source-attributed observed behavior and affected public surfaces. The inventory is offline, deterministic, and does not inspect or change provider state. Validation rejects non-public workspace evidence, missing known responsibility classes, duplicate stable IDs, missing evidence or decision owner, unapproved advisory dispositions, forbidden permissions, configuration-as-authority claims, and advisory authority claims.

For the advisory-only operations view, see [Advisory Automation and Manual Fallbacks](./advisory-automation.md). For privileged and policy blockers, see [Privileged and Policy Operations](./privileged-operations.md).

## Validation

```sh
cargo run --manifest-path release/validator/Cargo.toml --offline -- --root .
```

The command validates the canonical inventory and its generated mdBook view without network access or provider effects.
