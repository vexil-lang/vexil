# Release Unit Catalog

> Generated view of [`release/catalog.json`](../../../../release/catalog.json). The JSON catalog is canonical; this Markdown is non-authoritative and parity-checked.

This is a source-led inventory, not a Release Manifest, publication assertion, provider-identity claim, release-order decision, or version-selection decision. Formal specifications and documentation govern semantics; pinned code and tests establish the executable baseline. Historical tags, changelog headings, and registry observations remain evidence in [Release History](./history.md).

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
| `vexil-runtime-go` | `packages/runtime-go` | go-module `github.com/vexil-lang/vexil/packages/runtime-go` | `blocked-missing-version-source` | `none (required file absent)` in `packages/runtime-go/VERSION` | `packages/runtime-go/v<semver>` |
| `vexil-runtime-py` | `packages/runtime-py` | python-project `vexil_runtime` | `candidate-unreleased` | `0.1.0` in `packages/runtime-py/pyproject.toml` | `vexil-runtime-py-v<semver>` |
| `vexil-runtime-ts` | `packages/runtime-ts` | npm-package `@vexil-lang/runtime` | `source-inventory-only` | `0.4.1` in `packages/runtime-ts/package.json` | `vexil-runtime-ts-v<semver>` |
| `vexil-store` | `crates/vexil-store` | cargo-package `vexil-store` | `source-inventory-only` | `0.4.2` in `crates/vexil-store/Cargo.toml` | `vexil-store-v<semver>` |
| `vexilc` | `crates/vexilc` | cargo-package `vexilc`; cargo-binary `vexilc` | `source-inventory-only` | `0.5.1` in `crates/vexilc/Cargo.toml` | `vexilc-v<semver>` |

## Boundary and validation

`candidate-unreleased` means the Python source unit is planned work, not a PyPI availability claim. `blocked-missing-version-source` means the Go module has no checked-in `VERSION` source; its `go.mod` module path is not a version. `non-publishable` roots are deliberately cataloged so they cannot be silently mistaken for releases.

Dependency-edge entries are provisional until the typed graph is established. A catalog entry or target category never establishes authorization, registry identity, publication eligibility, release ordering, or Release Set membership. Root project-wide `v<semver>` tags remain prohibited during recovery.

```sh
cargo run --manifest-path release/validator/Cargo.toml --offline -- --root .
```

The offline command validates source paths, direct declaration observations, unique unit identities, the Go blocker, and byte-exact generated-view parity. It performs no provider query or release effect.
