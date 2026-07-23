# Release History

Vexil records release-history evidence under [`release/history/`](../../../../release/history/). Canonical JSON is authoritative; [`ledger.md`](../../../../release/history/ledger.md) is a deterministic, non-authoritative view.

The Historical Tag baseline is ratified from a read-only remote enumeration and two digest-bound role assertions. Local refs, a changelog line, a tag, a registry response, or a green workflow cannot replace those records.

Language-spec labels, wire-format identifiers, package versions, and coordinated-project releases are separate namespaces. During recovery, a project-wide root `v<semver>` tag is prohibited. The root changelog `v1.0.0` claim is retained as an audited anomaly; it does not establish a coordinated historical release.

Additive repair may document an anomaly, supersede an interpretation, or publish a new approved immutable identity. It may never move, delete, force-update, recreate, reuse, overwrite, or replace a Historical Tag or immutable released artifact.
