# Vexil Governance

Vexil is currently maintained by one lead developer. The process below keeps
decisions visible without pretending the project has a larger organization than
it does.

## Everyday changes

Bug fixes, documentation improvements, tests, and bounded implementation work
can be proposed directly as pull requests. Maintainers review and merge them.

Architectural choices should record the problem and rationale in a GitHub issue
labelled `decision`. Community input is welcome through issues and pull requests.

## Breaking changes

A proposed breaking change uses an issue labelled `breaking-change`. When there
is external participation, the issue remains open for at least 14 days after the
last substantive comment so affected users have a real opportunity to respond.
With no external engagement, the maintainer may proceed without waiting on an
empty clock, while still documenting the decision.

## Language and wire changes

Changes to syntax, semantics, canonical form, compatibility classification, the
conformance contract, or binary representation require an RFC issue before
implementation.

An RFC should describe:

1. the concrete problem;
2. the proposed contract;
3. alternatives considered;
4. compatibility and migration effects;
5. required specification, corpus, vector, target, and documentation work.

Use the `rfc` label. The same conditional 14-day comment period applies when
there are participants to hear from. The lead maintainer makes the final
decision and records the rationale in an issue labelled `decision`.

## Maintainer

| Name | GitHub | Area |
| --- | --- | --- |
| Furkan Mamuk | @furkanmamuk | Project-wide |

Additional maintainers may be invited after sustained, high-quality
contributions. There is no formal application process today.

## Community standards

Participation follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
Contributors retain copyright in their contributions. The project does not
require a contributor licence agreement.

Vexil is licensed under MIT OR Apache-2.0.
