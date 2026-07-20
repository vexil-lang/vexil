# Emergency Stop Tabletop Runbook

**Procedure ID:** `emergency-stop-runbook`
**Mode:** tabletop-only, non-mutating. This page identifies public control surfaces; it contains no executable provider command.

## Identified control surfaces

| Surface | Exact public identifier | Owner assertion | Required provider evidence | Current state |
|---|---|---|---|---|
| Release workflow | `.github/workflows/release.yml` | `repository-administrator` | verified workflow stop control | unverified external-control blocker |
| npm publication workflow | `.github/workflows/npm-publish.yml` | `repository-administrator` | verified workflow and trusted-publishing control | unverified external-control blocker |
| Documentation workflow and environment | `.github/workflows/docs.yml`, `github-pages` | `repository-administrator` | verified workflow and environment control | unverified external-control blocker |
| Release automation identity | No active release credential route is committed in `release.yml`; future privileged identity remains provider evidence | `repository-administrator` | verified identity selection and revocation control | unverified external-control blocker |
| npm publication identity | npm trusted publishing in `npm-publish.yml` | `repository-administrator` | verified registry identity and revocation control | unverified external-control blocker |

| Decision point | Asserted role | Allowed action | Prohibited action | Evidence destination | Prerequisite | Stop condition | Follow-up owner | Tabletop-only |
|---|---|---|---|---|---|---|---|---|
| Identify suspected compromised surface and owner | `repository-administrator` | contain | release or package publication, deployment, tag movement | `release/exercises/tabletop-stewardship-continuity-2026-07-14.json` | Public surface inventory above | Identity or target cannot be identified | `repository-administrator` | yes |
| Model the emergency boundary | `repository-administrator` | stop, revoke, contain | alter workflows, revoke a real credential, mutate approvals, artifacts, or ledger history | same public exercise record | Explicit role assertion | Any provider operation would be required | `repository-administrator` | yes |
| Escalate a missing live control | `repository-administrator` | activate-succession | declare the control tested or compliant | same public exercise record | external-control evidence | Evidence is absent, stale, or mismatched | `repository-administrator` | yes |

Actual provider-side stopping or revocation is deliberately not exercised here. It requires verified external controls and remains blocked.
