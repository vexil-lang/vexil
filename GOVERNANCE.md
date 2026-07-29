# Vexil Governance

## Current Model

Vexil is maintained by a single lead developer (BDFL model).

## Decision Making

### Architectural decisions

Made by the project lead. Significant decisions are documented in
GitHub issues labeled `decision` with rationale. Community input is welcome via
GitHub issues labeled `rfc`.

### Bug fixes and small improvements

Any contributor can submit a PR. Maintainers review and merge.

### Breaking changes

Require a GitHub issue labeled `breaking-change`. The 14-day window below
applies when there's actual external engagement to wait for, with no
outside comments, the project lead isn't required to sit out a clock with
nothing on it. If someone does comment, the full window restarts from their
last substantive input.

### Protocol changes (VNP wire format, Vexil schema language)

Require an explicit RFC (see below). Protocol stability is a
first-class concern: changes that affect wire compatibility or
the language specification is held to a higher bar than code changes.

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
3. **14-day comment period** if there's anyone to comment - see the note on
   breaking changes above.
4. The project lead makes the final decision and documents the rationale in a GitHub issue labeled `decision`

RFCs are required for: new language features, changes to encoding semantics,
wire format modifications, and changes to the conformance corpus contract.
Right now, that mostly means writing the rationale down before acting, the process is there so it's
ready the moment external contributors do.

## Code of Conduct

This project follows the Contributor Covenant v2.1.
See [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).

## License

Licensed under MIT OR Apache-2.0. Contributors retain copyright in their
contributions. The project does not require a CLA at this time.
