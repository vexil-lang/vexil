# Canonical Release Records

Release intent, reviewed evidence, detached approval, approval disposition,
start authorization, adapter observations, Run events, Run evidence, and
closeout are distinct retained record families. Their Draft 2020-12 contracts
are physically version-addressed under `release/schemas/`; a new dotted
version is a new retained schema file and never replaces an old one.

The release validator dispatches a record only by its exact `(recordKind,
schemaVersion)` and matching schema ID. Canonical records use compact UTF-8
JSON without a BOM or CR, lexicographic object keys, no insignificant
whitespace, and one terminal LF. JSONL Run events use one canonical object per
line, an LF on every line, contiguous sequence numbers, an external payload
digest, and a digest chain over the preceding exact line bytes.

Canonical materializations are public: evidence sets live at
`release/evidence-sets/<evidence-set-id>/evidence-set.json`; Manifests and
their detached approvals/dispositions live under `release/manifests/`; Run
start authorization, events, adapter results, other evidence, and closeout
live under `release/runs/`; History remains independently retained under
`release/history/entries/`. Human Markdown is generated and non-authoritative.

A draft Manifest can be replaced before first approval. From first approval,
its exact bytes are immutable. A Manifest has no self-digest: its identity is
the SHA-256 of its exact canonical bytes, calculated externally. Every
cross-family binding uses that external Manifest ID and digest. Approvals, dispositions, start authorizations,
adapter-result envelopes, Run evidence, and Run events are immutable; only the
Release Run Coordinator sequences events. A withdrawal or revocation is a
separate immutable disposition bound to the original approval ID and digest.
Closeout is owned by the Release Steward and cannot alter Manifest intent or
Run history. Each cross-family reference carries the referenced immutable
identity and digest.

Start authorization retains both assertions: the Release Steward issuer and
the Release Run Coordinator execution principal, each with actor, role, and
assignment, plus exact protected workload and environment identities. It also
retains the selected approval/disposition identities and digests, governance
revision, Historical Tag baseline/snapshot, security findings/exceptions,
candidate bundle/subject/attestation, target-control and permission evidence,
state-schema/reducer identities, allowed operations/units/targets/permissions,
its materialization path. Its SHA-256 identity is external to its canonical
bytes. The reviewed evidence-set is
Release-Steward reviewed, a deterministically ordered inventory with external
SHA-256 identity; it is bound identically by the Manifest, approval, and
authorization, but does not reference the Manifest itself. Detached approvals
also bind the exact payload digest and canonical `release/stewardship.json`
governance digest. Historical state replay resolves the exact retained public
state-schema and reducer bytes by their frozen version and digest, never the
newest implementation.

Run execution, time-based eligibility, state reduction, and effect semantics
are owned by later stories. At this stage, the offline validator verifies that
each Run event, adapter result, Run-evidence record, and closeout binds its
Run start authorization's exact identity and digest plus frozen Manifest and
evidence-set identities. Events also repeat the authorized execution principal
and retained state/reducer identities. Closeout must carry the exact ordered,
digest-checked inventory of that Run's retained adapter results and Run
evidence, with a currently eligible Release Steward at its closeout time.
These checks remain structural and create no Run effect.

Detached-approval construction takes exact canonical Manifest bytes and the
one Manifest-bound evidence-set identity, then binds their SHA-256 digests to
a `governance-revision-v1` digest. That revision is a domain-separated,
length-framed hash of the checked-in stewardship contract, assignments,
retained approval schemas, and any public governance records referenced by
those assignments. At record time and immediately before an effect, the
validator rechecks the Release Steward actor, explicit role, assignment,
scope, effective date, expiry, and the current revision. A second qualified
Release Steward makes a distinct Manifest approver mandatory; provider review
or CI cannot stand in for that identity check. Protected-main merge evidence
must bind the repository, `refs/heads/main`, commit/tree/blob identities,
approval path, observed blob bytes, exact Manifest and evidence-set bytes,
the complete current disposition snapshot, merge/PR identity, collection
time, and collector. Missing or stale evidence remains valid-but-not-canonical
rather than release authority.

The initial retained 1.0 disposition format requires the active Repository
Administrator authority, a non-empty reason, and source-attributed public
evidence. A disposition becomes invalidating at its whole-second UTC effective
time; it never edits the approval or retrospectively erases a tag or other
historical effect.

These schemas and offline validation establish structural contracts only. They
do not approve a Manifest, prove external controls, issue a tag, or authorize
or publish a release.
