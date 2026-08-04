# Changelog

## [0.5.0] - 2026-08-03


## Unreleased

### Added

- Prepare the Python generator crate for its first crates.io release.

### Fixed

- Reject unsafe or case-colliding project output paths instead of overwriting
  an earlier generated file.
- Propagate decode errors when an optional field's presence bit is missing
  instead of synthesizing `None`.
- Preserve bounded unknown non-exhaustive union payloads with collision-safe APIs.
- Generate valid, strictly typed Python for nested codecs, optionals, constraints,
  traits, newtypes, enums, project imports, and shared compliance vectors.
