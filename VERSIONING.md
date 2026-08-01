# Versioning

Vexil has two independent version axes: the **spec** (what "Vexil" means)
and **implementations** (the crates and packages that implement it). They
move on different schedules and don't need to match.

## Spec versions

- [`spec/language.md`](spec/language.md) — the language specification
  (syntax, type system). Currently `1.0.0-draft`: still evolving.
- [`spec/wire-format.md`](spec/wire-format.md) — the binary wire format. Currently
  `STABILIZING`: unchanged since April 2026, and breaking it would mean
  moving to a new format generation — but it hasn't been exercised by an
  external implementation or independently audited, so it isn't an ironclad
  guarantee yet.

"1.0" in either document names the target generation, not a maturity claim
about any published crate or package. A future breaking change to the wire
format would be a new spec generation (e.g. "Vexil 2.0 Binary Format"),
independent of whatever version numbers the crates happen to be at when
that happens.

## Implementation versions

Every crate and package has its own independent semver, bumped only when it
actually changes:

| Component | Where | Versioned independently because |
|---|---|---|
| `vexil-lang`, `vexil-runtime`, `vexil-store`, `vexilc`, codegen crates | crates.io | each has its own rate of change |
| `@vexil-lang/runtime` (TS) | npm | own release cadence |
| Go runtime | Go module proxy, tag-only | newest, least proven target — its low version number (`v0.1.x`) is an honest signal of that, not a gap to close |
| `vexil-runtime` (Python) | PyPI | own release cadence and representative generated-code coverage |

There is no single "Vexil version" number, and no attempt to keep crate
version numbers in lockstep with each other or with the spec version. If
you need to know what a specific crate/package actually guarantees, check
that component's own version and changelog — not the spec status, and not
any other component's version.

See [`spec/wire-format.md`](spec/wire-format.md) and
[`FAQ.md`](FAQ.md#is-vexil-production-ready) for what the spec's current
status does and doesn't promise.
