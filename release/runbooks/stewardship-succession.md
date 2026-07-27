# Stewardship Succession Tabletop Runbook

**Procedure ID:** `release-continuity-runbook`
**Mode:** tabletop-only, non-mutating. This procedure helps Vexil rehearse continuity; it does not change a Release Run or provider state.

## Canonical records

- Authority: [`release/stewardship.json`](../stewardship.json)
- Assignment and sole-maintainer policy: [`release/stewardship/assignments.json`](../stewardship/assignments.json), decision `sole-maintainer-governance-2026-07-23`
- Responsibility inventory: [`release/stewardship/responsibilities.json`](../stewardship/responsibilities.json)
- Advisory and privileged dispositions: [`release/advisory/automation-contract.json`](../advisory/automation-contract.json) and [`release/privileged/operations-contract.json`](../privileged/operations-contract.json)

## Recovery contact route

No successor is currently designated under the sole-maintainer policy. Record containment and request a reviewed successor through the [public decision route](https://github.com/vexil-lang/vexil/issues/new/choose). That route records a future decision; it does not alter an existing release record.

| Decision point | Asserted role | Allowed action | Prohibited action | Evidence destination | Prerequisite | Stop condition | Follow-up owner | Tabletop-only |
|---|---|---|---|---|---|---|---|---|
| Identify the unavailable authority and its assigned scope | `repository-administrator` | contain | approve a Manifest, authorize privileged work, approve publication, alter tags or evidence | `release/exercises/tabletop-stewardship-continuity-2026-07-14.json` | Current assignment record | Assignment, custody contact, or scope is missing | `repository-administrator` | yes |
| Freeze new privileged work and record the continuity gap | `repository-administrator` | stop, contain | publish, deploy, accept risk, declare completion | same public exercise record | Explicit role assertion | A provider-side stop would be required; do not perform it | `repository-administrator` | yes |
| Start reviewed succession | `repository-administrator` | activate-succession | create a Release Steward, approve a Manifest, grant publication authority | same public exercise record | Public governance review under `GOVERNANCE.md` | No reviewed successor exists | `repository-administrator` | yes |

The current sole-maintainer policy has no designated successor. Provider-side administrator recovery is an **unverified external-control blocker**, not an exercised control. Future Manifest approval and privileged publication each retain their independent evidence gates.
