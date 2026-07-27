# External Control Boundaries

> Public operational guidance for repository workflows. This page describes committed boundaries and required provider evidence; it is not provider configuration, a Release Manifest, or approval to perform an external effect.

## Current scope

Vexil has completed a manually protected tag for
[`vexil-runtime-go` `v0.1.1`](../../../../release/closeouts/runtime-go-0-1-1-manual-protected-tag-2026-07-26.json),
with a matching [public Go proxy observation](../../../../release/history/observations/observation-go-runtime-v0-1-1-publication-2026-07-26.json).
That retained outcome is scoped to the exact Go module tag. It does not claim a
GitHub Release, registry upload, deployment, bot action, workflow Run, or
provider readiness for another target.

## Committed workflow boundary

The release and registry workflows are advisory or rehearsal-only. The documentation workflow is a narrowly scoped exception: it builds every documented change, while only a `main` push may deploy the rendered book through the protected `github-pages` environment. That documentation deployment path has no release, registry, tag, or package-publication authority.

| Workflow | Classification | Committed boundary |
|---|---|---|
| `ci.yml` | advisory | Build and test only; each job is restricted to `contents: read`. |
| `docs.yml` | documentation deployment | Pull requests build only with `contents: read`; a successful `main` build may deploy its artifact through `github-pages` with `pages: write` and `id-token: write`. |
| `npm-publish.yml` | rehearsal | Performs the npm build and tests only; publication and trusted-publishing access are disabled. |
| `release.yml` | rehearsal | Records a blocked release boundary only; it cannot create a tag, release, artifact, or package publication. |

Untrusted pull-request and fork code must remain outside protected environments and must not receive secrets, OIDC tokens, registry identities, release permissions, or Pages deployment authority before it runs. The documentation workflow grants Pages authority only to its `main`-push deployment job after a separate build job succeeds.

## Provider-only controls and evidence

The following are controlled by GitHub or a registry provider and cannot be proved by committed YAML alone. A read-only, target-specific observation must record their state before any privileged path is enabled:

- Protected environment reviewers, self-review prohibition, administrator-bypass policy, wait timer, and branch/tag deployment policy for each target.
- Actions default token policy, repository and organization restrictions, and the effective permissions of every workflow job.
- The protected, target-isolated identity for GitHub releases, crates.io, npm, PyPI, documentation deployment, and the Go canonical-tag boundary. No identity, credential, or environment may be shared across targets.
- OIDC subject and audience restrictions or registry trusted-publishing bindings, plus revocation and emergency-stop ownership.
- Immutable full-commit provenance for every third-party action in a privileged job. Mutable tags and branches are not accepted there.

Provider environment approval is only an execution gate. It never substitutes for the required detached, Manifest-bound Release Steward approval, and the independent Manifest, registry, security, and external-control gates remain blockers.

The [Release Unit Catalog](./catalog.md) is a source inventory, not provider evidence. A catalog target name or category does not prove registry identity, authorization, publication eligibility, release ordering, or that any unit has been published.

## Reusable workflow boundary

The reusable release and registry workflows intentionally perform no live
release, registry, protected-branch, tag, credential, or provider-configuration
write. The sole committed workflow live-write path is the main-only Pages
artifact deployment described above. It remains distinct from release
publication and does not prove provider configuration. For a future target,
missing, inaccessible, stale, broader-than-expected, or ambiguous provider
evidence is `unknown` or `noncompliant` and keeps that target's path blocked.

For the canonical fail-closed procedure, see [Privileged and Policy Operations](./privileged-operations.md). For advisory fallbacks, see [Advisory Automation and Manual Fallbacks](./advisory-automation.md).
## Owner-authorized credential exception

Control observation normally uses a credential that cannot write. A Repository Administrator may approve a documented exception for a write-capable credential only for GET-only observation. The resulting evidence names that least-privilege was not enforced at credential level; it does not authorize a provider change, publication, tag operation, deployment, registry action, or credential change.
