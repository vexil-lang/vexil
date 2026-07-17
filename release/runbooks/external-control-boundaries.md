# External Control Boundaries Runbook

**Mode:** read-only evidence collection and non-mutating workflow rehearsal. This runbook is not a Manifest, provider configuration, approval, or authorization to release.

## Current safe state

Committed workflows are advisory or rehearsal-only:

- `ci.yml` builds and tests with `contents: read` only.
- `docs.yml` builds documentation but has no Pages deployment permission, OIDC token, or protected environment.
- `npm-publish.yml` builds and tests but has no registry credential, OIDC token, protected environment, or publication command.
- `release.yml` only records the fail-closed boundary and cannot create releases, publish artifacts, or modify tags.

No workflow may run untrusted pull-request or fork code in a privileged `pull_request_target` context. Advisory and rehearsal jobs have no protected environment, production secret, registry identity, release token, tag, release, publication, or deployment authority.

## Required read-only evidence before a privileged path exists

Record a target-specific provider observation for each of the following. If any result is unavailable, ambiguous, stale, or broader than required, record `unknown` or `noncompliant`; do not enable effects.

1. Protected environments: name, required reviewers, self-review rule, administrator-bypass rule, wait timer, and branch/tag deployment policy.
2. Workflow policy: default `GITHUB_TOKEN` permissions, per-job permissions, Actions restrictions, and whether untrusted PR/fork code can reach an environment, secret, OIDC token, or write permission.
3. Trusted identity: one isolated identity for each GitHub release, crates.io, npm, PyPI, documentation deployment, and Go canonical-tag target; record OIDC subject/audience or trusted-publishing binding, revocation owner, and emergency-stop owner.
4. Privileged action provenance: every third-party `uses:` reference must be a reviewed, full immutable commit SHA; a tag or branch is noncompliant.

Provider execution approval is distinct from, and cannot replace, detached Manifest approval. The unresolved stewardship continuity gate also remains a separate blocker.

## Stop condition

Do not create a tag, GitHub release, package publication, deployment, credential, protected-branch change, environment change, or provider configuration change from this runbook. Route a failed observation through the existing fail-closed procedure in [Privileged Readiness and Fail-Closed Procedures](./privileged-readiness-and-fail-closed.md).
## Owner-authorized credential exception

Use a no-write-capable credential for control observation by default. A Repository Administrator may explicitly authorize a write-capable credential for a GET-only audit when the resulting immutable observation records the authorization, says that credential least privilege was not enforced, and retains the GET-only method guard. This is an assurance deviation, not provider-write authority: mutation verbs, auto-remediation, tags, releases, deployments, registry publication, and credential changes remain forbidden.
