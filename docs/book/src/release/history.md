# Release History

Vexil records release-history evidence under [`release/history/`](../../../../release/history/). Canonical JSON is authoritative; [`ledger.md`](../../../../release/history/ledger.md) is a deterministic, non-authoritative view.

The Historical Tag baseline is ratified from a read-only remote enumeration and two digest-bound role assertions. Local refs, a changelog line, a tag, a registry response, or a green workflow cannot replace those records.

Language-spec labels, wire-format identifiers, package versions, and coordinated-project releases are separate namespaces. During recovery, a project-wide root `v<semver>` tag is prohibited. The root changelog `v1.0.0` claim is retained as an audited anomaly; it does not establish a coordinated historical release.

The separately retained `vexil-runtime-go` `v0.1.1` protected-tag closeout and
public Go proxy observation record a completed, target-scoped release outcome.
They do not turn historical evidence into a project-wide release claim. See
[Current Release Status](./current-status.md) for the exact boundary.

For a source-led view of maintained components, direct version declarations, and current release-unit status, see the [Release Unit Catalog](./catalog.md). The catalog distinguishes what the pinned source declares from historical evidence; neither page selects a current or future release version from tags, changelog headings, registries, or other history.

Additive repair may document an anomaly, supersede an interpretation, or publish a new approved immutable identity. It may never move, delete, force-update, recreate, reuse, overwrite, or replace a Historical Tag or immutable released artifact.
