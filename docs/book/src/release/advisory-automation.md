# Advisory Automation and Manual Fallbacks

This runbook records how Vexil uses advisory automation and human fallback. It is public guidance rather than a release decision or provider configuration; every entry is an offline declaration with no live release effect.

## Operating boundary

Advice may identify, triage, label, comment, or report. Release scope, risk acceptance, Manifest approval, protected changes, credentials, and publication remain deliberate maintainer decisions with their own records. If an advisory mechanism is unavailable, its named owner follows the stated manual fallback or defers and records the result.

## Advisory dispositions

| ID | Disposition | Owner role assertion | Minimum permissions | Failure behavior | Manual fallback |
|---|---|---|---|---|---|
<a id="rbr-001"></a>
<a id="rbr-002"></a>
<a id="rbr-003"></a>
<a id="rbr-004"></a>
<a id="rbr-005"></a>
<a id="rbr-006"></a>
<a id="rbr-007"></a>
<a id="rbr-008"></a>
<a id="rbr-009"></a>
| `RBR-001` | owned-manual-procedure | `release-run-coordinator` (`assignment-release-run-coordinator-2026-07-14`) |  | Record the unavailable advisory checklist and defer rather than infer approval from CI or historical automation. | defer by `assignment-repository-administrator-2026-07-14` |
| `RBR-002` | owned-manual-procedure | `release-run-coordinator` (`assignment-release-run-coordinator-2026-07-14`) |  | Record unavailable ordering advice and defer; do not derive membership or order from retired configuration. | defer by `assignment-repository-administrator-2026-07-14` |
| `RBR-005` | maintained-replacement | `repository-administrator` (`assignment-repository-administrator-2026-07-14`) | issues:write, pull-requests:write | If the contract is unavailable, the Repository Administrator manually triages or defers with public evidence. | perform-manually by `assignment-repository-administrator-2026-07-14` |
| `RBR-006` | maintained-replacement | `repository-administrator` (`assignment-repository-administrator-2026-07-14`) | issues:write, pull-requests:write | If the contract is unavailable, the Repository Administrator manually labels or defers with public evidence. | perform-manually by `assignment-repository-administrator-2026-07-14` |
| `RBR-007` | approved-retirement | `repository-administrator` (`assignment-repository-administrator-2026-07-14`) |  | There is no automatic acknowledgement; the owner may manually respond or defer with public evidence. | defer by `assignment-repository-administrator-2026-07-14` |

## Retirement evidence

- `RBR-007`: decision `advisory-automation-disposition-2026-07-14` is **accepted** at `docs/book/src/release/advisory-automation.md`; approver `github:furkanmamuk`. Lost behavior: Automatic first-issue and first-pull-request welcome messages. Residual risk: A delayed human response may reduce contributor clarity, but cannot be mistaken for approval.

## Verification

```sh
cargo run --manifest-path release/validator/Cargo.toml --offline -- --root .
```

This validation is deterministic and self-contained. It does not inspect or mutate providers.
