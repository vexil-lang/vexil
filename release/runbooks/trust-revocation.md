# Trust Revocation Tabletop Runbook

**Procedure ID:** `trust-revocation-runbook`
**Mode:** tabletop-only, non-mutating. It records a decision path, never a credential, secret, token, or provider operation.

| Decision point | Asserted role | Allowed action | Prohibited action | Evidence destination | Prerequisite | Stop condition | Follow-up owner | Tabletop-only |
|---|---|---|---|---|---|---|---|---|
| Classify a suspected trust failure | `repository-administrator` | contain | accept security risk, approve publication, rewrite evidence | `release/exercises/tabletop-stewardship-continuity-2026-07-14.json` | `release/stewardship.json` emergency boundary | The affected identity is unknown | `repository-administrator` | yes |
| Identify the public provider-control category | `repository-administrator` | stop, revoke, contain | use a credential, change an environment, mutate a workflow | same public exercise record | Exact surface listed in `emergency-stop-runbook` | verified control evidence is missing | `repository-administrator` | yes |
| Open a reviewed recovery path | `repository-administrator` | activate-succession | restore trust, publish, repair a tag, declare completion | same public exercise record | Existing security and governance policy | No reviewed custodian or provider evidence | `repository-administrator` | yes |

All GitHub Actions, environment, registry, and administrator recovery controls are **unverified external-control blockers**. This tabletop record must not be used to claim revocation was tested or compliant.
