# External Control Boundaries

> Public operational guidance for repository workflows. This page describes committed boundaries and required provider evidence; it is not provider configuration, a Release Manifest, or approval to perform an external effect.

## Committed workflow boundary

The current workflows are advisory or rehearsal-only. They use repository-read permission, do not reference a protected environment, and do not receive registry credentials, OIDC tokens, deployment authority, release tokens, or write-capable GitHub tokens.

| Workflow | Classification | Committed boundary |
|---|---|---|
| `ci.yml` | advisory | Build and test only; each job is restricted to `contents: read`. |
| `docs.yml` | advisory | Builds the book only; GitHub Pages deployment is disabled. |
| `npm-publish.yml` | rehearsal | Performs the npm build and tests only; publication and trusted-publishing access are disabled. |
| `release.yml` | rehearsal | Records a blocked release boundary only; it cannot create a tag, release, artifact, or package publication. |

Untrusted pull-request and fork code must remain outside protected environments and must not receive secrets, OIDC tokens, registry identities, or release permissions before it runs. An advisory workflow cannot hand authority to a later job through an environment name, inherited token default, artifact, or reusable-workflow input.

## Provider-only controls and evidence

The following are controlled by GitHub or a registry provider and cannot be proved by committed YAML alone. A read-only, target-specific observation must record their state before any privileged path is enabled:

- Protected environment reviewers, self-review prohibition, administrator-bypass policy, wait timer, and branch/tag deployment policy for each target.
- Actions default token policy, repository and organization restrictions, and the effective permissions of every workflow job.
- The protected, target-isolated identity for GitHub releases, crates.io, npm, PyPI, documentation deployment, and the Go canonical-tag boundary. No identity, credential, or environment may be shared across targets.
- OIDC subject and audience restrictions or registry trusted-publishing bindings, plus revocation and emergency-stop ownership.
- Immutable full-commit provenance for every third-party action in a privileged job. Mutable tags and branches are not accepted there.

Provider environment approval is only an execution gate. It never substitutes for the required detached, Manifest-bound Release Steward approval, and the unresolved continuity gate remains a blocker.

## No live writes

This repository state intentionally performs no live release, registry, Pages, deployment, protected-branch, tag, credential, or provider-configuration write. Missing, inaccessible, stale, broader-than-expected, or ambiguous provider evidence is `unknown` or `noncompliant` and keeps every release path blocked.

For the canonical fail-closed procedure, see [Privileged and Policy Operations](./privileged-operations.md). For advisory fallbacks, see [Advisory Automation and Manual Fallbacks](./advisory-automation.md).
## Owner-authorized credential exception

Control observation normally uses a credential that cannot write. A Repository Administrator may approve a documented exception for a write-capable credential only for GET-only observation. The resulting evidence names that least-privilege was not enforced at credential level; it does not authorize a provider change, publication, tag operation, deployment, registry action, or credential change.
