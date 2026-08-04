# Changelog

## [0.5.0] - 2026-08-03


## Unreleased

### Fixed

- Propagate unexpected EOF when an optional field's presence bit is missing
  instead of synthesizing an absent value.
- Preserve bounded unknown non-exhaustive union payloads with collision-safe APIs.
- Keep published Result discriminants and correct optional-container access,
  collection keys, geometric values, constraints, and generated file endings.

## [0.4.3] - 2026-03-29

### Fixed

- fix: readRemaining eats sibling array elements + union Pack spurious flush (fixes #40)

## [0.4.1] - 2026-03-28

### Fixed

- fix: code quality polish — remove unwrap, fix rustdoc, add crate docs, add derives

### Documentation

- docs: publication readiness — fix versions, add Go README, fix package names, update changelog

### Other

- chore(release): v0.4.1
