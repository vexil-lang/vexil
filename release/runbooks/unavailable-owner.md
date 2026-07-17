# Unavailable Owner Tabletop Runbook

**Procedure ID:** `unavailable-owner-runbook`
**Mode:** tabletop-only, non-mutating. Follow this from a clean checkout; no historical `vexil-bot` knowledge is required.

**Recovery contact:** No distinct recovery custodian is approved. Record containment and request a reviewed successor through the [public decision route](https://github.com/vexil-lang/vexil/issues/new/choose); it grants no recovery, Manifest, or publication authority.

| Decision point | Asserted role | Allowed action | Prohibited action | Evidence destination | Prerequisite | Stop condition | Follow-up owner | Tabletop-only |
|---|---|---|---|---|---|---|---|---|
| Locate the affected assignment and responsibility | `repository-administrator` | contain | publication, approval, tag repair, evidence rewrite | `release/exercises/tabletop-stewardship-continuity-2026-07-14.json` | `assignment-repository-administrator-2026-07-14`, `stewardship-continuity-2026-07-14`, and the responsibility inventory | Any canonical record is absent or stale | `repository-administrator` | yes |
| Record an administrative freeze of new privileged work | `repository-administrator` | stop, contain | workflow mutation, credential use, package or release effect | same public exercise record | Affected authority is identified | A real provider control would be needed | `repository-administrator` | yes |
| Request reviewed succession | `repository-administrator` | activate-succession | infer a successor's publication authority | same public exercise record | Existing `GOVERNANCE.md` review route | No reviewed successor and no non-publishing custodian | `repository-administrator` | yes |

The only emergency actions are **stop, revoke, contain, and activate succession**. Actual repository ownership, protection, or credential recovery remains an **unverified Epic 2 blocker**.
