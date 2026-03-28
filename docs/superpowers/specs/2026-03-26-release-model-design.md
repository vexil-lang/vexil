# Release Model Design

> **Scope:** Versioning, branching, publishing, and release automation for the Vexil schema language project. Covers all artifacts (Rust crates, npm packages, prebuilt binaries). Does NOT cover package management for Vexil schemas themselves (that's a future milestone).

**Goal:** Establish a simple, automated release pipeline that publishes all artifacts from a single git tag, with lockstep versioning across the workspace.

**Architecture:** Trunk-based development on `main`. Tag-triggered CI publishes to crates.io, npm, and GitHub Releases. All crates share a single version number.

---

## 1. Branching Strategy

**Trunk-based on `main`.** No `dev`/`stable` split.

- All development happens on `main` (direct commits or short-lived feature branches)
- Releases are tagged on `main`: `v0.1.0`, `v0.2.0`, etc.
- Hotfixes: branch from release tag, cherry-pick fix, tag patch release (`v0.1.1`)
- No long-lived release branches — too much overhead for a pre-1.0 project

### Hotfix flow

```
main:     A — B — C — D (v0.2.0) — E — F
                          \
hotfix/v0.1.1:             G (cherry-pick fix) → tag v0.1.1
```

Hotfix branches are short-lived — created from tag, fix applied, tagged, deleted.

---

## 2. Versioning

**Scheme:** `v0.MILESTONE.PATCH`

| Version | Meaning |
|---|---|
| `v0.1.0` | SDK architecture (CodegenBackend trait, crate rename) |
| `v0.2.0` | TypeScript backend + `@vexil-lang/runtime` |
| `v0.3.0` | LSP / editor tooling |
| `v0.x.1` | Patch release (bug fix, no new features) |
| `v1.0.0` | Language spec and wire format considered stable |

### Rules

- **Lockstep versioning:** All workspace crates share the same version number. No independent crate versions until post-1.0.
- **Milestone = minor bump:** Each completed milestone increments the minor version.
- **Patch = bug fix only:** No new features, no breaking changes.
- **Pre-1.0 = no stability guarantees:** Minor versions may include breaking changes. Tier 1 API is "stable by intent" but not by semver contract until 1.0.
- **Post-1.0:** Semver applies strictly. Breaking changes require major version bump.

---

## 3. Published Artifacts

| Artifact | Registry | From Version | Purpose |
|---|---|---|---|
| `vexilc` | crates.io (binary) | v0.1.0 | CLI: compile, codegen, build, lsp |
| `vexil-lang` | crates.io (library) | v0.1.0 | Compiler SDK for third-party tooling |
| `vexil-codegen-rust` | crates.io (library) | v0.1.0 | Rust codegen backend |
| `vexil-codegen-ts` | crates.io (library) | v0.2.0 | TypeScript codegen backend |
| `vexil-lsp` | crates.io (library) | v0.3.0 | LSP library (binary via `vexilc lsp`) |
| `@vexil-lang/runtime` | npm | v0.2.0 | TypeScript wire format runtime |
| `vexilc` binaries | GitHub Releases | v0.1.0 | Prebuilt for linux-x86_64, macos-arm64, windows-x86_64 |

### Installation methods

- `cargo install vexilc` — CLI with all compiled-in backends
- `npm install @vexil-lang/runtime` — for TypeScript projects consuming generated code
- GitHub Releases — prebuilt binaries for users without Rust toolchain

### Publish order (dependency graph)

```
1. vexil-lang           (no workspace deps)
2. vexil-codegen-rust   (depends on vexil-lang)
3. vexil-codegen-ts     (depends on vexil-lang)
4. vexil-lsp            (depends on vexil-lang)
5. vexilc               (depends on all above)
6. @vexil-lang/runtime       (npm, independent)
```

Order is enforced by CI. Each crate published only after its dependencies are confirmed on crates.io.

---

## 4. Release Automation

**Fully automated via release-plz + cargo-dist.**

### Pipeline

```
Push to main
  → release-plz detects unreleased changes (compares against crates.io registry)
  → Opens/updates Release PR with version bumps + CHANGELOG
  → Developer reviews and merges Release PR
  → release-plz publishes crates to crates.io (dependency order, automatic retries)
  → release-plz creates git tag (v0.x.0)
  → Git tag triggers cargo-dist workflow
  → cargo-dist builds binaries for 5 targets:
    - x86_64-unknown-linux-gnu
    - aarch64-unknown-linux-gnu
    - x86_64-pc-windows-msvc
    - aarch64-apple-darwin
    - x86_64-apple-darwin
  → cargo-dist creates GitHub Release with:
    - Tarballs (Linux/macOS) and zips (Windows)
    - SHA256 checksums
    - Shell + PowerShell install scripts
```

### Design properties

- **Release PR is the human gate.** Review version bumps and changelog before merging.
- **No manual tag pushing.** release-plz creates tags after successful crate publish.
- **Publish order automatic.** release-plz resolves the workspace dependency graph.
- **Cross-platform binaries built in CI.** cargo-dist handles the build matrix.
- **Reproducible.** Same commit, same toolchain, same flags.
- **Idempotent.** release-plz compares against crates.io — re-running is safe.

### Configuration files

| File | Purpose |
|---|---|
| `release-plz.toml` | Version groups, changelog config, publish settings |
| `cliff.toml` | Changelog template (conventional commits → markdown) |
| `dist-workspace.toml` | Binary targets, installers, CI config |
| `.github/workflows/release-plz.yml` | Version management + crates.io publishing |
| `.github/workflows/release.yml` | Binary builds + GitHub Release (cargo-dist managed) |

### Required secrets

| Secret | Purpose |
|---|---|
| `RELEASE_PLZ_TOKEN` | Fine-grained PAT for creating tags + Release PRs |
| `CARGO_REGISTRY_TOKEN` | crates.io publish token |

---

## 5. Changelog

Auto-generated by git-cliff (via release-plz) from conventional commits.

```
feat(parser): add union type support    → "Features" section
fix(codegen): correct enum bit width    → "Bug Fixes" section
perf(runtime): optimize varint decode   → "Performance" section
chore(release): v0.3.0                  → skipped (release commits filtered)
```

Configuration in `cliff.toml`. Changelog committed to `CHANGELOG.md` and included in GitHub Release body.

---

## 6. Decision Log

### Branching: trunk-based vs gitflow vs release branches

**Chosen:** Trunk-based on `main`.

**Rejected alternatives:**
- **Gitflow (`dev`/`main`/`release`):** Too much overhead for a pre-1.0 single-developer project. Merge ceremonies slow iteration.
- **Long-lived release branches:** Only needed when maintaining multiple major versions simultaneously. Not applicable until post-1.0.

**Rationale:** Simplest model that works. All code goes to `main`, releases are tags. Hotfixes are rare and handled via short-lived branches from tags.

### Versioning: lockstep vs independent crate versions

**Chosen:** Lockstep — all crates share the same version.

**Rejected:** Independent per-crate semver.

**Rationale:** The crates are tightly coupled (backends depend on `vexil-lang` IR types). Independent versioning creates compatibility matrices ("which `vexil-codegen-rust` works with which `vexil-lang`?"). Lockstep eliminates this. Independent versioning can be introduced post-1.0 if the crates diverge in stability.

### Release process: manual vs semi-automated vs fully automated

**Chosen:** Fully automated, tag-triggered.

**Rejected alternatives:**
- **Manual:** Error-prone. Wrong publish order, forgotten crates, inconsistent binaries. Doesn't scale.
- **Semi-automated (draft release + manual approval):** Adds a human step that provides no value if CI is green. The tag push IS the human decision.

**Rationale:** The tag is the gate. If CI passes, the release is valid. Adding manual approval after CI is ceremony, not safety. Automation ensures correct publish order, cross-platform binaries, and reproducibility.

### Publishing: binary-only vs library + binary

**Chosen:** Both. `vexilc` binary for end users, `vexil-lang` library for third-party tooling.

**Rejected:** Binary-only (don't publish `vexil-lang` as a library).

**Rationale:** Publishing the compiler as a library enables third-party linters, formatters, and alternative backends without forking. The cost is minimal (it's already a library crate). The Tier 1 API provides the stability boundary.

### npm scope: `@vexil-lang/runtime` vs `vexil-runtime`

**Chosen:** `@vexil-lang/runtime` (scoped package).

**Rejected:** `vexil-runtime` (unscoped).

**Rationale:** Scoped packages avoid name collisions, signal organizational ownership, and allow future expansion (`@vexil/codegen`, `@vexil/cli`). Standard practice for language tooling packages.
