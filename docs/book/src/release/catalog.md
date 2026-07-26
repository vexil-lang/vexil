# Release Unit Catalog

> Generated view of [`release/catalog.json`](../../../../release/catalog.json). The JSON catalog is canonical; this Markdown is non-authoritative and parity-checked.

This is a source-led inventory and typed structural dependency graph, not a Release Manifest, publication assertion, provider-identity claim, Release Set decision, or version-selection decision. Formal specifications and documentation govern semantics; pinned code and tests establish the executable baseline. Historical tags, changelog headings, and registry observations remain evidence in [Release History](./history.md).

## Units

| Unit | Source root | Targets | Status | Source version observation | Canonical tag policy |
|---|---|---|---|---|---|
| `command-protocol-example` | `examples/command-protocol` | example `command-protocol-example` | `non-publishable` | `0.1.0` in `examples/command-protocol/Cargo.toml` | `not applicable (non-publishable)` |
| `cross-language-rust-device-example` | `examples/cross-language/rust-device` | example `cross-language-rust-device` | `non-publishable` | `0.1.0` in `examples/cross-language/rust-device/Cargo.toml` | `not applicable (non-publishable)` |
| `multi-file-project-example` | `examples/multi-file-project` | example `multi-file-project-example` | `non-publishable` | `0.1.0` in `examples/multi-file-project/Cargo.toml` | `not applicable (non-publishable)` |
| `sensor-packet-example` | `examples/sensor-packet` | example `sensor-packet-example` | `non-publishable` | `0.1.0` in `examples/sensor-packet/Cargo.toml` | `not applicable (non-publishable)` |
| `system-monitor-example` | `examples/system-monitor` | example `system-monitor` | `non-publishable` | `0.1.0` in `examples/system-monitor/Cargo.toml` | `not applicable (non-publishable)` |
| `vexil-bench` | `crates/vexil-bench` | internal-tool `vexil-bench` | `non-publishable` | `0.1.0` in `crates/vexil-bench/Cargo.toml` | `not applicable (non-publishable)` |
| `vexil-codegen-go` | `crates/vexil-codegen-go` | cargo-package `vexil-codegen-go` | `source-inventory-only` | `0.4.3` in `crates/vexil-codegen-go/Cargo.toml` | `vexil-codegen-go-v<semver>` |
| `vexil-codegen-py` | `crates/vexil-codegen-py` | cargo-package `vexil-codegen-py` | `source-inventory-only` | `0.4.3` in `crates/vexil-codegen-py/Cargo.toml` | `vexil-codegen-py-v<semver>` |
| `vexil-codegen-rust` | `crates/vexil-codegen-rust` | cargo-package `vexil-codegen-rust` | `source-inventory-only` | `0.4.3` in `crates/vexil-codegen-rust/Cargo.toml` | `vexil-codegen-rust-v<semver>` |
| `vexil-codegen-ts` | `crates/vexil-codegen-ts` | cargo-package `vexil-codegen-ts` | `source-inventory-only` | `0.4.3` in `crates/vexil-codegen-ts/Cargo.toml` | `vexil-codegen-ts-v<semver>` |
| `vexil-lang` | `crates/vexil-lang` | cargo-package `vexil-lang` | `source-inventory-only` | `0.4.3` in `crates/vexil-lang/Cargo.toml` | `vexil-lang-v<semver>` |
| `vexil-release-governance-validator` | `release/validator` | internal-tool `vexil-release-governance-validator` | `non-publishable` | `0.1.0` in `release/validator/Cargo.toml` | `not applicable (non-publishable)` |
| `vexil-runtime` | `crates/vexil-runtime` | cargo-package `vexil-runtime` | `source-inventory-only` | `0.5.1` in `crates/vexil-runtime/Cargo.toml` | `vexil-runtime-v<semver>` |
| `vexil-runtime-go` | `packages/runtime-go` | go-module `github.com/vexil-lang/vexil/packages/runtime-go` | `source-inventory-only` | `0.1.1` in `packages/runtime-go/VERSION` | `packages/runtime-go/v<semver>` |
| `vexil-runtime-py` | `packages/runtime-py` | python-project `vexil_runtime` | `candidate-unreleased` | `0.1.0` in `packages/runtime-py/pyproject.toml` | `vexil-runtime-py-v<semver>` |
| `vexil-runtime-ts` | `packages/runtime-ts` | npm-package `@vexil-lang/runtime` | `source-inventory-only` | `0.4.1` in `packages/runtime-ts/package.json` | `vexil-runtime-ts-v<semver>` |
| `vexil-store` | `crates/vexil-store` | cargo-package `vexil-store` | `source-inventory-only` | `0.4.2` in `crates/vexil-store/Cargo.toml` | `vexil-store-v<semver>` |
| `vexilc` | `crates/vexilc` | cargo-package `vexilc`; cargo-binary `vexilc` | `source-inventory-only` | `0.5.1` in `crates/vexilc/Cargo.toml` | `vexilc-v<semver>` |

## Typed dependency graph

Each edge is recorded on its dependent unit. `related-before-unit` means the related unit's version must be publicly resolvable before the declaring unit is published. `publish_before` edges cite their checked-in runtime manifest declaration; `compatibility` and `bundle` edges cite an approved public release-dependency-edge decision. The catalog stores edges in stable `edgeType`, related-unit, evidence-kind, path, and location order.

| Dependency | Dependent | Type | Public source evidence |
|---|---|---|---|
| `vexil-lang` | `vexil-codegen-go` | `publish_before` | `crates/vexil-codegen-go/Cargo.toml` `dependencies.vexil-lang` |
| `vexil-lang` | `vexil-codegen-py` | `publish_before` | `crates/vexil-codegen-py/Cargo.toml` `dependencies.vexil-lang` |
| `vexil-lang` | `vexil-codegen-rust` | `publish_before` | `crates/vexil-codegen-rust/Cargo.toml` `dependencies.vexil-lang` |
| `vexil-lang` | `vexil-codegen-ts` | `publish_before` | `crates/vexil-codegen-ts/Cargo.toml` `dependencies.vexil-lang` |
| `vexil-lang` | `vexil-store` | `publish_before` | `crates/vexil-store/Cargo.toml` `dependencies.vexil-lang` |
| `vexil-runtime` | `vexil-store` | `publish_before` | `crates/vexil-store/Cargo.toml` `dependencies.vexil-runtime` |
| `vexil-codegen-go` | `vexilc` | `publish_before` | `crates/vexilc/Cargo.toml` `dependencies.vexil-codegen-go` |
| `vexil-codegen-py` | `vexilc` | `publish_before` | `crates/vexilc/Cargo.toml` `dependencies.vexil-codegen-py` |
| `vexil-codegen-rust` | `vexilc` | `publish_before` | `crates/vexilc/Cargo.toml` `dependencies.vexil-codegen-rust` |
| `vexil-codegen-ts` | `vexilc` | `publish_before` | `crates/vexilc/Cargo.toml` `dependencies.vexil-codegen-ts` |
| `vexil-lang` | `vexilc` | `publish_before` | `crates/vexilc/Cargo.toml` `dependencies.vexil-lang` |
| `vexil-store` | `vexilc` | `publish_before` | `crates/vexilc/Cargo.toml` `dependencies.vexil-store` |

Only `publish_before` participates in structural ordering. `compatibility` requires an approved shared-evidence decision without imposing registry order; `bundle` requires an approved identity decision and never creates a second Release Unit.

## Structural source order

The current all-unit structural order is derived from checked-in manifests and catalog edges only:

`vexil-lang` → `vexil-codegen-go` → `vexil-codegen-py` → `vexil-codegen-rust` → `vexil-codegen-ts` → `vexil-runtime` → `vexil-runtime-go` → `vexil-runtime-ts` → `vexil-store` → `vexilc`

## Boundary and validation

`candidate-unreleased` means the Python source unit is planned work, not a PyPI availability claim. The Go module's checked-in `VERSION` source identifies only its source state; `go.mod` supplies the module target identity, not its version. `non-publishable` roots are deliberately cataloged so they cannot be silently mistaken for releases.

A valid graph does not establish packageability, authorization, registry identity, publication eligibility, Release Set membership, Manifest approval, tags, or publication. Root project-wide `v<semver>` tags remain prohibited during recovery.

```sh
cargo run --manifest-path release/validator/Cargo.toml --offline -- --root .
```

The offline command validates source paths, runtime manifest declarations, typed graph agreement, deterministic structural order, unique unit identities, canonical tag policy, and byte-exact generated-view parity. It performs no provider query or release effect.
