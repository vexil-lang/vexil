# Advisory Manual Fallback Tabletop Runbook

**Procedure ID:** `advisory-manual-fallback-runbook`
**Mode:** tabletop-only, non-mutating. It applies only to advisory responsibilities in the canonical inventory.

| Decision point | Asserted role | Allowed action | Prohibited action | Evidence destination | Prerequisite | Stop condition | Follow-up owner | Tabletop-only |
|---|---|---|---|---|---|---|---|---|
| Identify the unavailable advisory responsibility | `release-run-coordinator` | perform-manually, defer | approve a Manifest, authorize privileged work, publish, deploy | the responsibility's public `auditEvidence` plus `release/exercises/tabletop-stewardship-continuity-2026-07-14.json` | Canonical advisory disposition in `release/stewardship/responsibilities.json` | No named owner or audit destination | `repository-administrator` | yes |
| Perform or defer the manual fallback | `repository-administrator` | perform-manually, defer | access credentials, change protected branches, accept risk, create release authority | same public evidence destination | Explicit role assertion and minimum advisory permissions only | The task needs privileged access or provider control | `repository-administrator` | yes |
| Consider retirement | `repository-administrator` | defer | retire a responsibility through this fallback | separately reviewed public retirement decision required by `advisory-automation.md` | accepted retirement path | No separately reviewed and approved retirement decision | `repository-administrator` | yes |

This fallback never becomes a privileged path. Advisory identity and permission verification remain an **unverified external-control blocker**; retirement is outside this exercise and requires a separate public decision.
