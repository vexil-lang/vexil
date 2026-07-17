# Stewardship Continuity Tabletop Exercises

> Generated public view of [`release/exercises/tabletop-stewardship-continuity-2026-07-14.json`](../../../../release/exercises/tabletop-stewardship-continuity-2026-07-14.json). The JSON record is canonical; this page is parity-checked and non-authoritative.

These are tabletop-only, non-mutating exercises, not Release Runs. The unresolved continuity gate still blocks Manifest approval and privileged publication.

## Record

Record `STE-2026-07-14-01` was exercised at `2026-07-14T18:00:00Z`. Evidence is retained as a version-controlled public record with no secrets.

## Scenarios

| Scenario | Procedure | Allowed boundary | Disposition |
|---|---|---|---|
| `unavailable-owner` | `release-continuity-runbook` | stop, contain, activate-succession | `blocked-pending-epic-2-controls` |
| `suspected-credential-or-automation-compromise` | `emergency-stop-runbook` | stop, revoke, contain | `blocked-pending-epic-2-controls` |
| `advisory-failure` | `advisory-manual-fallback-runbook` | perform-manually, defer | `blocked-pending-epic-2-controls` |
| `missing-provider-control` | `trust-revocation-runbook` | stop, revoke, contain, activate-succession | `blocked-pending-epic-2-controls` |

## Public runbooks

- [Stewardship succession](../../../../release/runbooks/stewardship-succession.md)
- [Unavailable owner](../../../../release/runbooks/unavailable-owner.md)
- [Emergency stop](../../../../release/runbooks/emergency-stop.md)
- [Trust revocation](../../../../release/runbooks/trust-revocation.md)
- [Advisory manual fallback](../../../../release/runbooks/advisory-manual-fallback.md)

Every provider-specific action is an **unverified Epic 2 blocker**. This evidence identifies future control categories; it does not test, configure, revoke, stop, publish, deploy, approve, or mutate any provider state.

## Offline validation

```sh
cargo run --manifest-path release/validator/Cargo.toml --offline -- --root .
```

The validator checks canonical assignment linkage, action boundaries, explicit Epic 2 blockers, public persistence, no secrets, required decision fields, and runbook safety. It does not invoke provider controls.
