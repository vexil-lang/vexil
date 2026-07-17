# Privileged Readiness and Fail-Closed Procedures

This runbook is generated from [`release/privileged/operations-contract.json`](../privileged/operations-contract.json). It records controlled replacement procedures for privileged and policy responsibilities; it is not a Manifest, approval, credential, workflow, release, or provider configuration. Every recorded operation is currently **blocked**.

## Non-authority rule

Historical bot configuration, historical behavior, green CI, tags, provider approval settings, CODEOWNERS, and private planning artifacts are not release authority. Dependency ordering and release preparation must use a current Manifest and typed Release Unit Catalog edges when those later controls exist; until then this runbook remains a visible blocking procedure.

## Universal pre-effect gate

No tag, GitHub release, package, deployment, environment, protected-branch, or credential effect is permitted unless an exact approved Manifest digest, verified Release Steward approval bound to that digest, target-specific protected identity, verified future Epic 2 controls, and immutable later candidate inputs all exist and match. Absence, uncertainty, staleness, or mismatch stops before the first effect and produces no effect event or external effect.

Advisory stages receive no privileged environment or credential. A separately scoped privileged stage may consume only approved immutable inputs after every required gate is verified. Broad or long-lived personal access tokens are rejected. Supported targets require OIDC or provider trusted publishing; a different route would require a separately approved, target-scoped, expiring, revocable, and auditable bootstrap exception.

## Current owned blocking procedures

| ID | Responsibility | Owner assertion | Target | Minimum permissions | Visible blockers | Fallback |
|---|---|---|---|---|---|---|
<a id="rbr-003"></a>
<a id="rbr-004"></a>
<a id="rbr-008"></a>
<a id="rbr-009"></a>
| privileged-operation-rbr-003 | RBR-003 | assignment-release-steward-2026-07-14 | release-manifest-bound-tag | contents:write:refs/tags/exact-approved-manifest-tag | unresolved continuity gate; Epic 2 verified external controls absent; later immutable candidate inputs absent | Release Steward retains the visible blocking procedure; no fallback may create, move, or delete a tag. |
| privileged-operation-rbr-004 | RBR-004 | assignment-release-steward-2026-07-14 | release-unit:exact-approved-manifest-entry | publish:exact-approved-release-unit | unresolved continuity gate; Epic 2 verified external controls absent; Epic 5 authorization evidence absent; later immutable candidate inputs absent | Release Steward retains the visible blocking procedure; no fallback may publish a package, release, artifact, or deployment. |
| privileged-operation-rbr-008 | RBR-008 | assignment-security-steward-2026-07-14 | repository:vexil-lang/RFC-and-wire-policy | repository-metadata:read | Epic 2 verified external controls absent; later immutable candidate policy evidence absent | Security Steward retains the visible blocking procedure and routes compatibility concerns through GOVERNANCE.md without asserting release authority. |
| privileged-operation-rbr-009 | RBR-009 | assignment-repository-administrator-2026-07-14 | repository:vexil-lang/manual-fail-closed-knowledge | repository-metadata:read | Epic 2 verified external controls absent; later immutable candidate evidence absent | Repository Administrator retains the visible blocking procedure; current CI, tags, provider settings, or historical configuration cannot replace it. |

## Procedure boundary

Each row is an owned fail-closed procedure with exactly one responsibility ID. It requires the current Manifest and typed catalog edges rather than `.vexilbot.toml` or historical behavior. The runbook does not make any procedure operationally ready: later Epic 2 external controls, later authorization/candidate evidence, and the unresolved continuity gate remain explicit blockers. A green test or workflow cannot complete a blocked operation.

For compatibility and policy decisions, follow [GOVERNANCE.md](../../GOVERNANCE.md); this runbook neither changes nor bypasses its BDFL, RFC, or breaking-change commitments.

## Validation

```sh
cargo run --manifest-path release/validator/Cargo.toml --offline -- --root .
```

The command validates this public contract offline and fails closed. It does not change a workflow, environment, credential, tag, registry, provider, or release.
