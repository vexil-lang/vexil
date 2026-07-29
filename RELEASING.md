# Releasing

Releases are cut locally with [`cargo-release`](https://github.com/crate-ci/cargo-release) —
there is no bot or server involved, and no standing publish credentials live
anywhere but your own machine.

## One-time setup

```
cargo install cargo-release git-cliff --locked
cargo login   # crates.io token, stored in ~/.cargo/credentials.toml
```

## Cutting a release

```
cargo release <patch|minor|major> -p <crate-name>
```

This always dry-runs first — review the output (version bump, dependent
crates whose version requirements get updated, the changelog section
`git-cliff` generates from conventional-commit messages). When it looks
right, re-run with `--execute`.

What it does, per crate:

- Bumps the crate's version in `Cargo.toml`.
- Updates any workspace crate that depends on it to require the new version.
- Runs `git-cliff` (config: `cliff.toml`) scoped to that crate's path and
  inserts the new section into `crates/<crate>/CHANGELOG.md`.
- Commits, tags (`<crate>-v<version>`, matching existing history), and pushes.
- Runs `cargo publish`.

## What happens after the tag is pushed

CI picks it up from there — nothing further to do by hand:

- `vexilc-v*` tags → `release.yml` (cargo-dist) builds and publishes the
  GitHub Release with prebuilt binaries/installers.
- `vexil-runtime-ts-v*` tags → `npm-publish.yml` publishes to npm via OIDC
  (no npm token needed locally). `vexil-runtime`'s crates.io release doesn't
  push this tag automatically — bump `packages/runtime-ts/package.json` and
  tag it yourself when the TS runtime needs a matching release.
- `packages/runtime-go/v*` tags → nothing to run; the Go proxy picks it up
  directly from the tag, same as any Go module.

## vexil-codegen-py

This crate has never been published to crates.io, but `vexilc` now depends
on it (`crates/vexilc/Cargo.toml`). `vexilc`'s next `cargo publish` will fail
until `vexil-codegen-py` is released at least once — `cargo release -p
vexil-codegen-py` first (a first release has no previous tag to diff
against; `git-cliff` will fall back to full history for that crate's path).

## Workspace-wide releases

`cargo release <level> --workspace` considers every publishable crate at
once, in dependency order, skipping anything without changes since its last
tag. `crates/vexil-bench` is excluded automatically (`publish = false` in
its own `Cargo.toml`).
