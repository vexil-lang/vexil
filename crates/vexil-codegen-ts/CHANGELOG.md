# Changelog

## [0.4.3] - 2026-03-29

### Fixed

- fix: readRemaining eats sibling array elements + union Pack spurious flush (fixes #40)

## [0.4.1] - 2026-03-28

### Fixed

- fix: code quality polish — remove unwrap, fix rustdoc, add crate docs, add derives
- fix: rename npm package from @vexil/runtime to @vexil-lang/runtime

### Documentation

- docs: publication readiness — fix versions, add Go README, fix package names, update changelog

### Other

- chore(release): v0.4.1

## [0.3.0] - 2026-03-28

### Added

- feat(codegen): emit decode-and-discard for typed tombstones
- feat(vexil-codegen-ts): emit _unknown field for schema evolution round-tripping
- feat(vexil-codegen-ts): generate delta encoder/decoder classes

### Fixed

- fix: message-level @delta implies varint/zigzag for wire size reduction

### Documentation

- docs: rewrite READMEs, fix stale facts, remove AI patterns

### Other

- chore(release): bump crate versions
- chore(release): bump crate versions

All notable changes to this project will be documented in this file.






