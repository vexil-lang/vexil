# Stewardship Continuity Tabletop Exercises

> Generated public view of [`release/exercises/tabletop-stewardship-continuity-2026-07-14.json`](../../../../release/exercises/tabletop-stewardship-continuity-2026-07-14.json). The JSON record is canonical; this page is parity-checked and non-authoritative.

These tabletop exercises let Vexil practice continuity without changing provider state. They preserve the historical absence of a distinct custodian; the sole-maintainer policy does not turn that absence into a release gate. Any future target still needs its own external-control and release evidence.

## Record

Record `STE-2026-07-14-01` was exercised at `2026-07-14T18:00:00Z`. Evidence is retained as a version-controlled public record with no secrets.

## Scenarios

| Scenario | Procedure | Allowed boundary | Disposition |
|---|---|---|---|
| `unavailable-owner` | `release-continuity-runbook` | stop, contain, activate-succession | `blocked-pending-external-controls` |
| `suspected-credential-or-automation-compromise` | `emergency-stop-runbook` | stop, revoke, contain | `blocked-pending-external-controls` |
| `advisory-failure` | `advisory-manual-fallback-runbook` | perform-manually, defer | `blocked-pending-external-controls` |
| `missing-provider-control` | `trust-revocation-runbook` | stop, revoke, contain, activate-succession | `blocked-pending-external-controls` |

## Public runbooks

- [Stewardship succession](../../../../release/runbooks/stewardship-succession.md)
- [Unavailable owner](../../../../release/runbooks/unavailable-owner.md)
- [Emergency stop](../../../../release/runbooks/emergency-stop.md)
- [Trust revocation](../../../../release/runbooks/trust-revocation.md)
- [Advisory manual fallback](../../../../release/runbooks/advisory-manual-fallback.md)

A provider-specific action remains an **unverified external-control blocker** until Vexil records the required target evidence. These exercises identify what to establish next; they do not test, configure, revoke, stop, publish, deploy, approve, or mutate provider state.

## Offline validation

```sh
cargo run --manifest-path release/validator/Cargo.toml --offline -- --root .
```

The validator checks canonical assignment linkage, action boundaries, explicit external-control blockers, public persistence, no secrets, required decision fields, and runbook safety. It does not invoke provider controls.
