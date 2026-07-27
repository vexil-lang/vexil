# Vexil Governance

## Current Model

Vexil is maintained by a single lead developer (BDFL model).

## Release stewardship authority

The canonical, machine-checkable definition of release role types and authority
boundaries is [`release/stewardship.json`](./release/stewardship.json). Its
checked, non-authoritative public view is available in the
[Stewardship Authority Model](./docs/book/src/release/stewardship.md).

This contract supports Vexil's own reproducible release practice; it does not
replace this BDFL decision model, the 14-day breaking-change comment period,
or the RFC process below. It also does not prove live workflow or provider
enforcement.

Vexil has completed and publicly observed the target-scoped
[`packages/runtime-go/v0.1.1`](./release/closeouts/runtime-go-0-1-1-manual-protected-tag-2026-07-26.json)
manual protected-tag release. That outcome is not a GitHub Release, registry
upload, deployment, bot action, workflow Run, or readiness finding for any
other target. The [current release status](./docs/book/src/release/current-status.md)
names the retained evidence and the separate prerequisites that remain for
future targets and reusable automated procedures.

## Decision Making

### Architectural decisions

Made by the project lead. Significant decisions are documented in
GitHub issues labeled `decision` with rationale. Community input is welcome via
GitHub issues labeled `rfc`.

### Bug fixes and small improvements

Any contributor can submit a PR. Maintainers review and merge.

### Breaking changes

Require a GitHub issue labeled `breaking-change`, open for at least
**14 days** for community feedback before any PR is merged.

### Protocol changes (VNP wire format, Vexil schema language)

Require an explicit RFC (see below). Protocol stability is a
first-class concern — changes that affect wire compatibility or
the language specification are held to a higher bar than code changes.

## Maintainers

| Name | GitHub | Area |
|------|--------|------|
| Furkan Mamuk | @furkanmamuk | Everything |

## Becoming a Maintainer

Maintainers are invited based on consistent, quality contributions
over time. There is no formal application process at this stage.

## RFC Process

1. Open a GitHub issue with the `rfc` label
2. Describe: the problem, the proposed solution, and alternatives considered
3. **14-day comment period** — the community may raise concerns or propose amendments
4. The project lead makes the final decision and documents the rationale in a GitHub issue labeled `decision`

RFCs are required for: new language features, changes to encoding semantics,
wire format modifications, and changes to the conformance corpus contract.

## Code of Conduct

This project follows the Contributor Covenant v2.1.
See [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).

## License

Licensed under MIT OR Apache-2.0. Contributors retain copyright in their
contributions. The project does not require a CLA at this time.
