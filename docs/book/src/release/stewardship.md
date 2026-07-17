# Stewardship Authority Model

> Generated view of [`release/stewardship.json`](../../../../release/stewardship.json). The JSON record is canonical; this Markdown is non-authoritative and parity-checked.

## Authority boundary

Only an explicit **Release Steward** role assertion bound to an approved Release Manifest identity and digest can authorize privileged effects. Tags, bots, workflows, green CI, registries, provider approvals, and private build artifacts are non-authoritative evidence or tooling.

| Role | Decision scope | Permitted actions |
|---|---|---|
| Release Steward | Release Manifest lifecycle and closeout. | approve-release-manifest, authorize-privileged-release, close-release-manifest |
| Repository Administrator | Repository protections, applications, credentials, emergency stop, containment, revocation, and succession activation. | stop, revoke, contain, activate-succession |
| Security Steward | Vulnerability disposition, disclosure/remediation policy, and time-bounded security exceptions. | disposition-vulnerability, set-disclosure-remediation-policy, grant-time-bounded-security-exception |
| Package Steward | Correctness and public verification obligations for assigned Release Units. | verify-assigned-release-unit, verify-namespace-health, verify-packaging-health |
| Release Run Coordinator | Sequence Release Run events and execute already-authorized actions. | sequence-release-run, execute-authorized-release-action |

## Boundaries and continuity

Advisory automation may validate, triage, label, advise on dependencies, and rehearse only. It has no release, package, deployment, protected-branch, environment, credential, version-selection, Release Set scope-selection, or risk-acceptance authority. A Repository Administrator may only stop, revoke, contain, and activate succession in an emergency; it may not move tags, overwrite artifacts, rewrite evidence, accept security risk, approve publication, or declare completion.

Roles may be combined, but permissions never union implicitly: each action requires an explicit asserted role. Role assignments are deliberately absent here and belong to Story 1.2. Contract validation does not prove live workflow or provider enforcement. Publication remains blocked until Story 1.2 resolves assignments and continuity and Epic 2 corrects and verifies external controls.

## Offline validation

From the repository root, run the repository-local validator without network access:

```sh
cargo run --manifest-path release/validator/Cargo.toml --offline -- --root .
```

It validates schema syntax, the canonical record, semantic authority invariants, documentation parity, and the public/private boundary.

## Compatibility governance

This contract does not replace the BDFL, RFC, public-review, or breaking-change rules in [the governance policy](../../../../GOVERNANCE.md). Language, wire-format, compiler, generator, runtime, corpus/conformance, and public API changes continue through that existing route.
