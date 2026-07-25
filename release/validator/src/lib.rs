use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use toml::{Table as TomlTable, Value as TomlValue};

const REQUIRED_ROLE_IDS: [&str; 5] = [
    "release-steward",
    "repository-administrator",
    "security-steward",
    "package-steward",
    "release-run-coordinator",
];
const ROLE_FIELDS: [&str; 9] = [
    "id",
    "label",
    "decisionScope",
    "permittedActions",
    "prohibitedActions",
    "approvalDuties",
    "auditSurface",
    "continuityRequirement",
    "roleCombinationConstraints",
];
const ROOT_FIELDS: [&str; 10] = [
    "$schema",
    "$id",
    "contractSchema",
    "version",
    "roles",
    "privilegedAuthorization",
    "nonAuthorityClasses",
    "advisoryAutomation",
    "governanceRoute",
    "publicationBlock",
];

struct CanonicalRecord {
    digest: String,
    path: std::path::PathBuf,
    value: Value,
}

/// Reviewed, caller-supplied inputs for deterministic Manifest construction.
/// This constructor is pure: it returns bytes only and never materializes a
/// Manifest, approval, authorization, Run, tag, or provider operation.
pub struct ReleaseManifestRequest<'a> {
    pub manifest: &'a Value,
    pub evidence_set_bytes: &'a [u8],
}

#[derive(Debug, PartialEq, Eq)]
pub struct GeneratedReleaseManifest {
    pub bytes: Vec<u8>,
    pub external_digest: String,
}

/// A validated, non-executing candidate-build plan. It is deliberately only
/// the isolated-build contract: callers still need a separate clean workspace
/// and must not treat this as an artifact, attestation, or release authority.
#[derive(Debug, PartialEq, Eq)]
pub struct IsolatedCandidateBuildPlan {
    pub manifest_id: String,
    pub manifest_digest: String,
    pub base_commit: String,
    pub source_commits: BTreeMap<String, String>,
}

/// A materialized detached checkout for one declared Release Unit. It carries
/// no build result, credential, artifact, attestation, or release authority.
#[derive(Debug, PartialEq, Eq)]
pub struct IsolatedCandidateWorkspace {
    pub unit_id: String,
    pub source_commit: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ManifestGenerationDiagnostic {
    pub requirement: &'static str,
    pub source: String,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ManifestGenerationError {
    pub diagnostics: Vec<ManifestGenerationDiagnostic>,
}

impl fmt::Display for ManifestGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{} [{}]: {}",
                    diagnostic.requirement, diagnostic.source, diagnostic.message
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        formatter.write_str(&diagnostics)
    }
}

impl std::error::Error for ManifestGenerationError {}

/// Exact, caller-supplied inputs for a detached approval. The constructor only
/// returns canonical bytes; it never writes a record or creates release authority.
pub struct DetachedApprovalRequest<'a> {
    pub approval_id: &'a str,
    pub manifest_bytes: &'a [u8],
    pub evidence_set_id: &'a str,
    pub evidence_set_digest: &'a str,
    pub governance_digest: &'a str,
    pub actor: &'a str,
    pub role: &'a str,
    pub assignment_id: &'a str,
    pub manifest_approver_actor: Option<&'a str>,
    pub approved_at: &'a str,
    pub expires_at: &'a str,
    pub provider_audit_reference: Option<&'a str>,
}

/// Source-attributed protected-main evidence supplied by an external collector.
/// This is evidence for an offline decision, never a provider query or release effect.
pub struct ApprovalMergeEvidence<'a> {
    pub repository: &'a str,
    pub reference: &'a str,
    pub commit_id: &'a str,
    pub tree_id: &'a str,
    pub approval_path: &'a str,
    pub blob_id: &'a str,
    pub observed_blob_bytes: &'a [u8],
    pub approval_digest: &'a str,
    pub manifest_bytes: &'a [u8],
    pub evidence_set_bytes: &'a [u8],
    pub merge_or_pr_id: &'a str,
    pub observed_at: &'a str,
    pub collector: &'a str,
    pub manifest_approver_actor: Option<&'a str>,
    pub dispositions: &'a [&'a [u8]],
    pub dispositions_complete: bool,
}

pub struct ApprovalDispositionRequest<'a> {
    pub disposition_id: &'a str,
    pub disposition: &'a str,
    pub effective_at: &'a str,
    pub actor: &'a str,
    pub role: &'a str,
    pub assignment_id: &'a str,
    pub reason: &'a str,
    pub source: &'a str,
    pub observed_at: &'a str,
    pub collector: &'a str,
}

/// Exact immutable approval evidence supplied to the pure Run-start preflight.
/// The embedded merge observation is data only; this type cannot query a
/// provider or materialize a canonical record.
pub struct DetachedApprovalPreflight<'a> {
    pub approval_bytes: &'a [u8],
    pub merge: ApprovalMergeEvidence<'a>,
}

/// Complete public inputs for a deterministic privileged Run-start preflight.
/// `authorization` is the proposed retained record; it is returned as canonical
/// bytes only when every local, byte-exact gate passes.
pub struct PrivilegedRunStartRequest<'a> {
    pub authorization: &'a Value,
    pub manifest_bytes: &'a [u8],
    pub evidence_set_bytes: &'a [u8],
    pub approvals: &'a [DetachedApprovalPreflight<'a>],
    pub historical_tag_baseline: &'a Value,
    pub historical_tag_snapshot: &'a Value,
    pub evaluation_time: &'a str,
}

/// Static inputs a future privileged job must present before dispatch. Lease
/// acquisition and Run sequencing remain owned by Epic 10; this only rejects
/// missing or mismatched proof before any credential or provider access.
pub struct PrivilegedJobPreflight<'a> {
    pub authorization_bytes: &'a [u8],
    pub run_id: &'a str,
    pub actor: &'a str,
    pub role: &'a str,
    pub assignment_id: &'a str,
    pub evaluation_time: &'a str,
    pub coordinator_lease_active: bool,
    pub sequenced_run_context_active: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct GeneratedPrivilegedRunStartAuthorization {
    pub bytes: Vec<u8>,
    pub external_digest: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AuthorizationBlocker {
    pub requirement: &'static str,
    pub source: String,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PrivilegedRunStartError {
    pub blockers: Vec<AuthorizationBlocker>,
}

impl fmt::Display for PrivilegedRunStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            &self
                .blockers
                .iter()
                .map(|blocker| {
                    format!(
                        "{} [{}]: {}",
                        blocker.requirement, blocker.source, blocker.message
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

impl std::error::Error for PrivilegedRunStartError {}

#[derive(Debug, PartialEq, Eq)]
pub enum DetachedApprovalOutcome {
    InvalidStructure,
    ValidButNotCanonical,
    CanonicalButCurrentlyIneligible,
    Eligible,
}

#[derive(Default)]
struct CanonicalRecordIndex {
    records: BTreeMap<(String, String), CanonicalRecord>,
    events: Vec<(std::path::PathBuf, Value)>,
    visited_directories: BTreeSet<std::path::PathBuf>,
}

#[derive(Clone)]
struct ExpectedControl {
    provider: String,
    scope: String,
    method: String,
    path: String,
}
const ACTIONS: [&str; 28] = [
    "approve-release-manifest",
    "authorize-privileged-release",
    "close-release-manifest",
    "stop",
    "revoke",
    "contain",
    "activate-succession",
    "move-tag",
    "overwrite-artifact",
    "rewrite-evidence",
    "accept-security-risk",
    "approve-publication",
    "declare-completion",
    "disposition-vulnerability",
    "set-disclosure-remediation-policy",
    "grant-time-bounded-security-exception",
    "verify-assigned-release-unit",
    "verify-namespace-health",
    "verify-packaging-health",
    "sequence-release-run",
    "execute-authorized-release-action",
    "select-semantic-version",
    "select-release-set-scope",
    "change-protected-branch",
    "publish-package",
    "deploy",
    "access-environment",
    "use-credential",
];
const ASSIGNMENT_ROOT_FIELDS: [&str; 10] = [
    "$schema",
    "$id",
    "assignmentSchema",
    "version",
    "decision",
    "identities",
    "assignments",
    "continuity",
    "publicationReadiness",
    "futureRunbooks",
];
const ASSIGNMENT_FIELDS: [&str; 8] = [
    "assignmentId",
    "roleId",
    "primaryActorId",
    "scope",
    "effectiveFrom",
    "reviewEvidence",
    "continuityProcedure",
    "status",
];
const MAINTAINED_ROOTS: [&str; 11] = [
    "crates/vexil-lang",
    "crates/vexilc",
    "crates/vexil-runtime",
    "crates/vexil-codegen-rust",
    "crates/vexil-codegen-ts",
    "crates/vexil-codegen-go",
    "crates/vexil-codegen-py",
    "crates/vexil-store",
    "packages/runtime-ts",
    "packages/runtime-py",
    "packages/runtime-go",
];
const INVENTORY_ROOT_FIELDS: [&str; 8] = [
    "$schema",
    "$id",
    "inventorySchema",
    "version",
    "historicalConfiguration",
    "manifestComparison",
    "responsibilities",
    "normalization",
];
const RESPONSIBILITY_FIELDS: [&str; 10] = [
    "id",
    "responsibilityClass",
    "description",
    "privilegeClass",
    "historicalEvidence",
    "affectedSurfaces",
    "failureImpact",
    "decisionOwner",
    "dispositionStatus",
    "advisoryDisposition",
];
const REQUIRED_RESPONSIBILITY_CLASSES: [&str; 9] = [
    "release-preparation",
    "dependency-ordering",
    "tagging",
    "publication",
    "triage",
    "labeling",
    "welcome-messaging",
    "policy-warnings",
    "manual-fallback-knowledge",
];
const PRIVILEGE_CLASSES: [&str; 3] = ["advisory", "privileged", "policy"];
const ADVISORY_DISPOSITIONS: [&str; 3] = [
    "maintained-replacement",
    "owned-manual-procedure",
    "approved-retirement",
];
const ADVISORY_PERMISSION_INTENTS: [&str; 7] = [
    "repository-metadata:read",
    "issues:read",
    "issues:write",
    "discussions:read",
    "discussions:write",
    "pull-requests:read",
    "pull-requests:write",
];
const ADVISORY_EFFECTS: [&str; 3] = ["advisory-route", "maintainer-review-note", "advisory-label"];
const PRIVILEGED_OPERATION_ROOT_FIELDS: [&str; 6] = [
    "$schema",
    "$id",
    "version",
    "inventorySource",
    "nonAuthorityStatement",
    "operations",
];
const PRIVILEGED_OPERATION_FIELDS: [&str; 17] = [
    "id",
    "responsibilityId",
    "kind",
    "owner",
    "authorityClass",
    "target",
    "minimumPermissions",
    "auditSurface",
    "requiredInputs",
    "authentication",
    "hybridBoundary",
    "currentReadiness",
    "blockingPrerequisites",
    "preEffectStopCondition",
    "failureBehavior",
    "fallback",
    "effectPolicy",
];

/// Computes the retained governance-revision-v1 digest from the public files
/// that determine detached-approval eligibility. This intentionally frames
/// path and content bytes, so neither concatenation ambiguity nor JSON
/// reserialization can change the identity.
pub fn governance_revision_v1(root: &Path) -> Result<String, String> {
    let assignments_path = "release/stewardship/assignments.json";
    let assignments = read_json(&root.join(assignments_path))?;
    validate_assignments(&assignments)?;
    let mut paths = vec![
        "release/stewardship.json".to_owned(),
        assignments_path.to_owned(),
        "release/schemas/stewardship.schema.json".to_owned(),
        "release/schemas/stewardship-assignment.schema.json".to_owned(),
        "release/schemas/release-detached-approval-1.0.schema.json".to_owned(),
        "release/schemas/release-approval-disposition-1.0.schema.json".to_owned(),
    ];
    collect_checked_in_governance_paths(&assignments, &mut paths);
    paths.sort();
    if paths.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("governance revision has duplicate public inputs".to_owned());
    }
    let root_canonical =
        fs::canonicalize(root).map_err(|error| format!("canonicalize repository root: {error}"))?;
    let mut revision = b"vexil-governance-revision-1.0\n".to_vec();
    for path in paths {
        let candidate = Path::new(&path);
        if candidate.is_absolute()
            || candidate
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(format!(
                "governance revision has unsafe public path: {path}"
            ));
        }
        let full_path = fs::canonicalize(root.join(&path))
            .map_err(|error| format!("governance revision input is missing {path}: {error}"))?;
        if !full_path.starts_with(&root_canonical) {
            return Err(format!(
                "governance revision input escapes repository root: {path}"
            ));
        }
        let bytes = fs::read(&full_path)
            .map_err(|error| format!("read governance revision input {path}: {error}"))?;
        revision.extend_from_slice(path.len().to_string().as_bytes());
        revision.extend_from_slice(b":");
        revision.extend_from_slice(path.as_bytes());
        revision.extend_from_slice(b"\n");
        revision.extend_from_slice(bytes.len().to_string().as_bytes());
        revision.extend_from_slice(b":");
        revision.extend_from_slice(&bytes);
        revision.extend_from_slice(b"\n");
    }
    Ok(sha256_hex(&revision))
}

/// Constructs one schema-valid detached approval from exact canonical Manifest
/// bytes and current public governance. It has no filesystem side effects.
pub fn construct_detached_approval(
    root: &Path,
    request: &DetachedApprovalRequest<'_>,
) -> Result<Vec<u8>, String> {
    if request
        .provider_audit_reference
        .is_some_and(|reference| reference.is_empty())
    {
        return Err("provider audit reference must not be empty when supplied".to_owned());
    }
    let manifest = parse_canonical_json_bytes(request.manifest_bytes, "Release Manifest")?;
    validate_canonical_release_record_schema(root, &manifest)?;
    let manifest = object(&manifest, "Release Manifest")?;
    if text(manifest.get("recordKind"), "Release Manifest kind")? != "release-manifest" {
        return Err("approval constructor requires a Release Manifest".to_owned());
    }
    let manifest_id = text(manifest.get("manifestId"), "Release Manifest ID")?;
    let manifest_digest = sha256_hex(request.manifest_bytes);
    if text(manifest.get("evidenceSetId"), "Manifest evidence-set ID")? != request.evidence_set_id
        || text(
            manifest.get("evidenceSetDigest"),
            "Manifest evidence-set digest",
        )? != request.evidence_set_digest
    {
        return Err(
            "approval constructor input does not bind the Manifest's exact evidence set".to_owned(),
        );
    }
    if request.governance_digest != governance_revision_v1(root)? {
        return Err(
            "approval constructor governance digest does not match governance-revision-v1"
                .to_owned(),
        );
    }
    ensure_release_steward_eligible(
        root,
        request.actor,
        request.role,
        request.assignment_id,
        request.approved_at,
        request.manifest_approver_actor,
        true,
    )?;
    let approved = utc_second_timestamp(request.approved_at)?;
    let expires = utc_second_timestamp(request.expires_at)?;
    if approved >= expires {
        return Err("detached approval must have approvedAt before expiresAt".to_owned());
    }
    let mut approval = serde_json::json!({
        "$schema": "https://vexil.dev/release/schemas/release-detached-approval-1.0.schema.json",
        "approvalId": request.approval_id,
        "approvedAt": request.approved_at,
        "approver": {"actor": request.actor, "assignment": request.assignment_id, "role": request.role},
        "evidenceSetDigest": request.evidence_set_digest,
        "evidenceSetId": request.evidence_set_id,
        "expiresAt": request.expires_at,
        "governanceDigest": request.governance_digest,
        "manifestDigest": manifest_digest,
        "manifestId": manifest_id,
        "recordKind": "release-detached-approval",
        "schemaVersion": "1.0"
    });
    if let Some(actor) = request.manifest_approver_actor {
        approval
            .as_object_mut()
            .ok_or("constructed approval is not an object")?
            .insert(
                "manifestApproverActor".to_owned(),
                Value::String(actor.to_owned()),
            );
    }
    validate_canonical_release_record_schema(root, &approval)?;
    canonical_json_bytes(&approval)
}

/// Constructs a retained withdrawal or revocation record. The
/// original approval bytes are read-only input and are never rewritten.
pub fn construct_approval_disposition(
    root: &Path,
    approval_bytes: &[u8],
    request: &ApprovalDispositionRequest<'_>,
) -> Result<Vec<u8>, String> {
    let approval = parse_canonical_json_bytes(approval_bytes, "detached approval")?;
    validate_canonical_release_record_schema(root, &approval)?;
    let approval = object(&approval, "detached approval")?;
    if text(approval.get("recordKind"), "detached approval kind")? != "release-detached-approval" {
        return Err("disposition constructor requires a detached approval".to_owned());
    }
    if !matches!(request.disposition, "withdrawn" | "revoked") {
        return Err("approval disposition must be withdrawn or revoked".to_owned());
    }
    ensure_repository_administrator(
        root,
        request.actor,
        request.role,
        request.assignment_id,
        request.effective_at,
    )?;
    if utc_second_timestamp(request.effective_at)?
        < utc_second_timestamp(text(approval.get("approvedAt"), "approval time")?)?
    {
        return Err("approval disposition predates the approval".to_owned());
    }
    let record = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-approval-disposition-1.0.schema.json",
        "approvalDigest":sha256_hex(approval_bytes),
        "approvalId":text(approval.get("approvalId"), "approval ID")?,
        "authority":{"actor":request.actor,"assignment":request.assignment_id,"role":request.role},
        "disposition":request.disposition,
        "dispositionId":request.disposition_id,
        "effectiveAt":request.effective_at,
        "reason":request.reason,
        "recordKind":"release-approval-disposition",
        "schemaVersion":"1.0",
        "sourceEvidence":{"collector":request.collector,"observedAt":request.observed_at,"source":request.source}
    });
    validate_canonical_release_record_schema(root, &record)?;
    canonical_json_bytes(&record)
}

/// Rechecks an immutable detached approval immediately before a privileged
/// effect. The result distinguishes malformed data, missing canonical-merge
/// evidence, current ineligibility, and an offline eligible approval.
pub fn assess_detached_approval(
    root: &Path,
    approval_bytes: &[u8],
    evaluation_time: &str,
    merge: Option<&ApprovalMergeEvidence<'_>>,
) -> Result<DetachedApprovalOutcome, String> {
    let approval = match parse_canonical_json_bytes(approval_bytes, "detached approval") {
        Ok(approval) => approval,
        Err(_) => return Ok(DetachedApprovalOutcome::InvalidStructure),
    };
    if validate_canonical_release_record_schema(root, &approval).is_err() {
        return Ok(DetachedApprovalOutcome::InvalidStructure);
    }
    let approval = object(&approval, "detached approval")?;
    if text(approval.get("recordKind"), "detached approval kind")? != "release-detached-approval" {
        return Ok(DetachedApprovalOutcome::InvalidStructure);
    }
    let Some(merge) = merge else {
        return Ok(DetachedApprovalOutcome::ValidButNotCanonical);
    };
    if !valid_detached_approval_dependencies(root, approval, merge)? {
        return Ok(DetachedApprovalOutcome::InvalidStructure);
    }
    if !valid_protected_main_merge_evidence(approval, approval_bytes, evaluation_time, merge)? {
        return Ok(DetachedApprovalOutcome::ValidButNotCanonical);
    }
    if !merge.dispositions_complete {
        return Ok(DetachedApprovalOutcome::CanonicalButCurrentlyIneligible);
    }
    let approver = object(
        approval.get("approver").ok_or("approval has no approver")?,
        "approver",
    )?;
    let governance_current = text(
        approval.get("governanceDigest"),
        "approval governance digest",
    )? == governance_revision_v1(root)?;
    let time_eligible = utc_second_timestamp(text(approval.get("approvedAt"), "approval time")?)?
        <= utc_second_timestamp(evaluation_time)?
        && utc_second_timestamp(evaluation_time)?
            < utc_second_timestamp(text(approval.get("expiresAt"), "approval expiry")?)?;
    let actor_eligible = ensure_release_steward_eligible(
        root,
        text(approver.get("actor"), "approval actor")?,
        text(approver.get("role"), "approval role")?,
        text(approver.get("assignment"), "approval assignment")?,
        evaluation_time,
        merge.manifest_approver_actor,
        true,
    )
    .is_ok();
    let disposition_invalidates = merge.dispositions.iter().any(|bytes| {
        let Ok(disposition) = parse_canonical_json_bytes(bytes, "approval disposition") else {
            return true;
        };
        if validate_canonical_release_record_schema(root, &disposition).is_err() {
            return true;
        }
        let Ok(disposition) = object(&disposition, "approval disposition") else {
            return true;
        };
        disposition.get("approvalId") == approval.get("approvalId")
            && disposition.get("approvalDigest").and_then(Value::as_str)
                == Some(sha256_hex(approval_bytes).as_str())
            && disposition
                .get("effectiveAt")
                .and_then(Value::as_str)
                .and_then(|effective| utc_second_timestamp(effective).ok())
                .is_none_or(|effective| {
                    utc_second_timestamp(evaluation_time).map_or(true, |now| now >= effective)
                })
    });
    if governance_current && time_eligible && actor_eligible && !disposition_invalidates {
        Ok(DetachedApprovalOutcome::Eligible)
    } else {
        Ok(DetachedApprovalOutcome::CanonicalButCurrentlyIneligible)
    }
}

/// Performs the authorization-only preflight required before a future Release
/// Run may begin. It is deliberately side-effect free: success returns only
/// canonical record bytes and their external SHA-256 identity; failure returns
/// every independently evaluable blocker and no bytes, lease, event, or effect.
pub fn authorize_privileged_run_start(
    root: &Path,
    request: &PrivilegedRunStartRequest<'_>,
) -> Result<GeneratedPrivilegedRunStartAuthorization, PrivilegedRunStartError> {
    let mut blockers = Vec::new();
    let authorization = request.authorization;
    let record = match object(authorization, "privileged Run start authorization") {
        Ok(record) => record,
        Err(error) => {
            push_authorization_blocker(
                &mut blockers,
                "authorization-structure",
                "authorization",
                error,
            );
            return Err(authorization_error(blockers));
        }
    };

    if let Err(error) = validate_canonical_release_record_schema(root, authorization) {
        push_authorization_blocker(
            &mut blockers,
            "authorization-schema",
            "release/schemas/privileged-run-start-authorization-1.0.schema.json",
            error,
        );
    }
    if let Err(error) = ensure_no_private_leakage(
        &serde_json::to_string(authorization)
            .map_err(|error| format!("serialize authorization: {error}"))
            .unwrap_or_else(|error| error),
    ) {
        push_authorization_blocker(&mut blockers, "public-inputs-only", "authorization", error);
    }
    if let Err(error) = validate_authorization_time_window(record, request.evaluation_time) {
        push_authorization_blocker(
            &mut blockers,
            "authorization-window",
            "authorization",
            error,
        );
    }
    if let Err(error) = validate_authorization_manifest_and_evidence(
        root,
        record,
        request.manifest_bytes,
        request.evidence_set_bytes,
    ) {
        push_authorization_blocker(
            &mut blockers,
            "manifest-and-evidence-binding",
            "authorization",
            error,
        );
    }
    if let Err(error) = validate_authorization_history_tags(
        record,
        request.historical_tag_baseline,
        request.historical_tag_snapshot,
    ) {
        push_authorization_blocker(
            &mut blockers,
            "fresh-historical-tag-observation",
            "release/history",
            error,
        );
    }
    if let Err(error) =
        validate_authorization_approvals(root, record, request.approvals, request.evaluation_time)
    {
        push_authorization_blocker(
            &mut blockers,
            "current-detached-approval",
            "release/manifests",
            error,
        );
    }
    if let Err(error) =
        validate_authorization_governance_and_principals(root, record, request.evaluation_time)
    {
        push_authorization_blocker(
            &mut blockers,
            "current-governance-and-principals",
            "release/stewardship",
            error,
        );
    }
    if let Err(error) = validate_external_controls_repository(root) {
        push_authorization_blocker(
            &mut blockers,
            "external-control-contract",
            "release/controls",
            error,
        );
    }
    if let Err(error) = validate_authorization_scope(record, request.manifest_bytes) {
        push_authorization_blocker(
            &mut blockers,
            "exact-authorized-scope",
            "authorization",
            error,
        );
    }

    if !blockers.is_empty() {
        return Err(authorization_error(blockers));
    }
    match canonical_json_bytes(authorization) {
        Ok(bytes) => Ok(GeneratedPrivilegedRunStartAuthorization {
            external_digest: sha256_hex(&bytes),
            bytes,
        }),
        Err(error) => Err(authorization_error(vec![AuthorizationBlocker {
            requirement: "authorization-canonical-bytes",
            source: "authorization".to_owned(),
            message: error,
        }])),
    }
}

/// Validates the dual gate required before future privileged-job dispatch.
/// This creates no lease, event, Run state, credential access, or effect.
pub fn validate_privileged_job_preflight(
    root: &Path,
    request: &PrivilegedJobPreflight<'_>,
) -> Result<(), String> {
    let authorization = parse_canonical_json_bytes(
        request.authorization_bytes,
        "privileged job start authorization",
    )?;
    validate_canonical_release_record_schema(root, &authorization)?;
    let authorization = object(&authorization, "privileged job start authorization")?;
    if text(
        authorization.get("recordKind"),
        "privileged job authorization kind",
    )? != "privileged-run-start-authorization"
    {
        return Err("privileged job requires a start authorization record".to_owned());
    }
    if text(
        authorization.get("runId"),
        "privileged job authorization Run ID",
    )? != request.run_id
    {
        return Err("privileged job Run ID does not match its authorization".to_owned());
    }
    validate_authorization_time_window(authorization, request.evaluation_time)?;
    let principal = object(
        authorization
            .get("executionPrincipal")
            .ok_or("privileged job authorization lacks execution principal")?,
        "privileged job execution principal",
    )?;
    for (field, actual) in [
        ("actor", request.actor),
        ("role", request.role),
        ("assignment", request.assignment_id),
    ] {
        if text(
            principal.get(field),
            "privileged job execution-principal field",
        )? != actual
        {
            return Err(format!(
                "privileged job {field} does not match its authorization"
            ));
        }
    }
    if !request.coordinator_lease_active {
        return Err("privileged job requires an active Coordinator-owned lease".to_owned());
    }
    if !request.sequenced_run_context_active {
        return Err("privileged job requires an active sequenced Run context".to_owned());
    }
    Ok(())
}

fn push_authorization_blocker(
    blockers: &mut Vec<AuthorizationBlocker>,
    requirement: &'static str,
    source: impl Into<String>,
    message: String,
) {
    blockers.push(AuthorizationBlocker {
        requirement,
        source: source.into(),
        message,
    });
}

fn authorization_error(mut blockers: Vec<AuthorizationBlocker>) -> PrivilegedRunStartError {
    blockers.sort();
    blockers.dedup();
    PrivilegedRunStartError { blockers }
}

fn validate_authorization_time_window(
    record: &Map<String, Value>,
    evaluation_time: &str,
) -> Result<(), String> {
    let issued_at = utc_second_timestamp(text(record.get("issuedAt"), "authorization issuedAt")?)?;
    let not_before =
        utc_second_timestamp(text(record.get("notBefore"), "authorization notBefore")?)?;
    let expires_at =
        utc_second_timestamp(text(record.get("expiresAt"), "authorization expiresAt")?)?;
    let evaluation = utc_second_timestamp(evaluation_time)?;
    if issued_at > not_before || not_before >= expires_at {
        return Err("authorization must satisfy issuedAt <= notBefore < expiresAt".to_owned());
    }
    if evaluation < not_before || evaluation >= expires_at {
        return Err("authorization is not valid at the supplied evaluation time".to_owned());
    }
    let run_id = text(record.get("runId"), "authorization Run ID")?;
    let expected_path = format!("release/runs/{run_id}/start-authorization.json");
    if text(
        record.get("materializationPath"),
        "authorization materialization path",
    )? != expected_path
    {
        return Err("authorization materialization path does not bind its exact Run ID".to_owned());
    }
    Ok(())
}

fn validate_authorization_manifest_and_evidence(
    root: &Path,
    record: &Map<String, Value>,
    manifest_bytes: &[u8],
    evidence_set_bytes: &[u8],
) -> Result<(), String> {
    let manifest_value = parse_canonical_json_bytes(manifest_bytes, "authorization Manifest")?;
    validate_canonical_release_record_schema(root, &manifest_value)?;
    let manifest = object(&manifest_value, "authorization Manifest")?;
    if text(manifest.get("recordKind"), "authorization Manifest kind")? != "release-manifest" {
        return Err("authorization input is not a Release Manifest".to_owned());
    }
    if text(
        manifest.get("schemaVersion"),
        "authorization Manifest schema version",
    )? != "1.1"
    {
        return Err("authorization requires the retained release-manifest@1.1 contract".to_owned());
    }
    validate_manifest_security(root, &manifest_value)?;
    let evidence_set =
        parse_canonical_json_bytes(evidence_set_bytes, "authorization evidence set")?;
    validate_canonical_release_record_schema(root, &evidence_set)?;
    let evidence_set = object(&evidence_set, "authorization evidence set")?;
    if text(
        evidence_set.get("recordKind"),
        "authorization evidence-set kind",
    )? != "release-evidence-set"
    {
        return Err("authorization input is not a reviewed evidence set".to_owned());
    }
    for field in ["manifestId", "evidenceSetId", "evidenceSetDigest"] {
        if record.get(field) != manifest.get(field) {
            return Err(format!(
                "authorization does not bind its exact Manifest {field}"
            ));
        }
    }
    if text(
        record.get("manifestDigest"),
        "authorization Manifest digest",
    )? != sha256_hex(manifest_bytes)
    {
        return Err("authorization Manifest digest does not bind exact canonical bytes".to_owned());
    }
    if record.get("evidenceSetId") != evidence_set.get("evidenceSetId")
        || text(
            record.get("evidenceSetDigest"),
            "authorization evidence digest",
        )? != sha256_hex(evidence_set_bytes)
    {
        return Err("authorization evidence set does not bind exact canonical bytes".to_owned());
    }
    for field in ["stateSchema", "reducer"] {
        if record.get(field) != manifest.get(field) {
            return Err(format!(
                "authorization does not freeze the Manifest {field}"
            ));
        }
    }
    let candidate = object(
        record
            .get("candidate")
            .ok_or("authorization lacks candidate binding")?,
        "authorization candidate binding",
    )?;
    let manifest_candidate = object(
        manifest
            .get("candidate")
            .ok_or("Manifest lacks candidate custody artifact")?,
        "Manifest candidate custody artifact",
    )?;
    if text(
        candidate.get("bundleDigest"),
        "authorization candidate bundle digest",
    )? != text(
        manifest_candidate.get("digest"),
        "Manifest candidate custody digest",
    )? {
        return Err(
            "authorization candidate bundle does not bind the Manifest custody artifact".to_owned(),
        );
    }
    let security = object(
        record
            .get("security")
            .ok_or("authorization lacks security binding")?,
        "authorization security binding",
    )?;
    let manifest_security = object(
        manifest
            .get("security")
            .ok_or("Manifest lacks security artifact")?,
        "Manifest security artifact",
    )?;
    if text(
        security.get("findingsDigest"),
        "authorization security findings digest",
    )? != text(
        manifest_security.get("digest"),
        "Manifest security artifact digest",
    )? {
        return Err(
            "authorization security findings do not bind the Manifest security artifact".to_owned(),
        );
    }
    Ok(())
}

fn validate_authorization_history_tags(
    record: &Map<String, Value>,
    baseline: &Value,
    snapshot: &Value,
) -> Result<(), String> {
    validate_history_tag_snapshot(baseline, snapshot)?;
    let baseline = object(baseline, "authorization historical-tag baseline")?;
    let baseline_digest = text(
        baseline.get("baselineDigest"),
        "historical-tag baseline digest",
    )?
    .strip_prefix("sha256:")
    .ok_or("historical-tag baseline digest must use sha256:")?;
    let baseline_binding = object(
        record
            .get("historicalTagBaseline")
            .ok_or("authorization lacks baseline binding")?,
        "authorization historical-tag baseline binding",
    )?;
    if text(
        baseline_binding.get("digest"),
        "authorization baseline digest",
    )? != baseline_digest
    {
        return Err("authorization baseline digest does not bind the ratified baseline".to_owned());
    }
    let snapshot_binding = object(
        record
            .get("historicalTagSnapshot")
            .ok_or("authorization lacks snapshot binding")?,
        "authorization historical-tag snapshot binding",
    )?;
    if text(
        snapshot_binding.get("digest"),
        "authorization snapshot digest",
    )? != sha256_hex(&canonical_json_bytes(snapshot)?)
    {
        return Err(
            "authorization snapshot digest does not bind exact fresh observation bytes".to_owned(),
        );
    }
    Ok(())
}

fn validate_authorization_approvals(
    root: &Path,
    record: &Map<String, Value>,
    supplied: &[DetachedApprovalPreflight<'_>],
    evaluation_time: &str,
) -> Result<(), String> {
    let selected = array(
        record.get("selectedApprovals"),
        "authorization selected approvals",
    )?;
    if selected.len() != supplied.len() {
        return Err(
            "authorization selected approvals do not match the complete supplied approval set"
                .to_owned(),
        );
    }
    let mut supplied_by_id = BTreeMap::new();
    for preflight in supplied {
        let approval =
            parse_canonical_json_bytes(preflight.approval_bytes, "authorization approval")?;
        validate_canonical_release_record_schema(root, &approval)?;
        let approval = object(&approval, "authorization approval")?;
        let approval_id = text(approval.get("approvalId"), "authorization approval ID")?;
        if supplied_by_id
            .insert(approval_id.to_owned(), preflight)
            .is_some()
        {
            return Err(format!(
                "authorization supplies duplicate approval {approval_id}"
            ));
        }
    }
    for selected_approval in selected {
        let selected_approval = object(selected_approval, "authorization selected approval")?;
        let approval_id = text(selected_approval.get("approvalId"), "selected approval ID")?;
        let preflight = supplied_by_id.remove(approval_id).ok_or_else(|| {
            format!("authorization selected approval {approval_id} was not supplied")
        })?;
        if text(
            selected_approval.get("approvalDigest"),
            "selected approval digest",
        )? != sha256_hex(preflight.approval_bytes)
        {
            return Err(format!(
                "authorization selected approval {approval_id} digest is misbound"
            ));
        }
        if assess_detached_approval(
            root,
            preflight.approval_bytes,
            evaluation_time,
            Some(&preflight.merge),
        )? != DetachedApprovalOutcome::Eligible
        {
            return Err(format!(
                "authorization selected approval {approval_id} is not currently eligible"
            ));
        }
    }
    if !supplied_by_id.is_empty() {
        return Err("authorization did not select every supplied approval".to_owned());
    }
    Ok(())
}

fn validate_authorization_governance_and_principals(
    root: &Path,
    record: &Map<String, Value>,
    evaluation_time: &str,
) -> Result<(), String> {
    let governance = object(
        record
            .get("governanceRevision")
            .ok_or("authorization lacks governance revision")?,
        "authorization governance revision",
    )?;
    if text(governance.get("id"), "authorization governance revision ID")?
        != "governance-revision-v1"
        || text(
            governance.get("digest"),
            "authorization governance revision digest",
        )? != governance_revision_v1(root)?
    {
        return Err("authorization governance revision is stale or misbound".to_owned());
    }
    let issuer = object(
        record.get("issuer").ok_or("authorization lacks issuer")?,
        "authorization issuer",
    )?;
    ensure_active_assignment(
        root,
        text(issuer.get("actor"), "authorization issuer actor")?,
        text(issuer.get("role"), "authorization issuer role")?,
        text(issuer.get("assignment"), "authorization issuer assignment")?,
        "release-manifest-lifecycle",
        evaluation_time,
        "authorization issuer",
    )?;
    let principal = object(
        record
            .get("executionPrincipal")
            .ok_or("authorization lacks execution principal")?,
        "authorization execution principal",
    )?;
    ensure_active_assignment(
        root,
        text(principal.get("actor"), "authorization execution actor")?,
        text(principal.get("role"), "authorization execution role")?,
        text(
            principal.get("assignment"),
            "authorization execution assignment",
        )?,
        "release-run-execution",
        evaluation_time,
        "authorization execution principal",
    )
}

fn validate_authorization_scope(
    record: &Map<String, Value>,
    manifest_bytes: &[u8],
) -> Result<(), String> {
    let targets = array(record.get("allowedTargets"), "authorization targets")?
        .iter()
        .map(|value| text(Some(value), "authorization target").map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let controls = array(
        record.get("targetControlEvidence"),
        "authorization target control evidence",
    )?
    .iter()
    .map(|value| {
        let control = object(value, "authorization target control evidence")?;
        text(control.get("target"), "authorization target control target").map(str::to_owned)
    })
    .collect::<Result<BTreeSet<_>, _>>()?;
    if targets != controls {
        return Err(
            "authorization target controls must cover exactly the allowed targets".to_owned(),
        );
    }
    let manifest = parse_canonical_json_bytes(manifest_bytes, "authorization Manifest")?;
    let manifest = object(&manifest, "authorization Manifest")?;
    if let Some(units) = manifest.get("releaseUnits") {
        let manifest_units = array(Some(units), "Manifest release units")?
            .iter()
            .map(|value| {
                let value = object(value, "Manifest release unit")?;
                text(value.get("unitId"), "Manifest release unit ID").map(str::to_owned)
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        for unit in array(record.get("allowedUnits"), "authorization units")? {
            let unit = text(Some(unit), "authorization unit")?;
            if !manifest_units.contains(unit) {
                return Err(format!("authorization expands scope with unit {unit}"));
            }
        }
    }
    Ok(())
}

fn collect_checked_in_governance_paths(value: &Value, paths: &mut Vec<String>) {
    match value {
        Value::Object(entries) => entries
            .values()
            .for_each(|entry| collect_checked_in_governance_paths(entry, paths)),
        Value::Array(entries) => entries
            .iter()
            .for_each(|entry| collect_checked_in_governance_paths(entry, paths)),
        Value::String(path)
            if path.starts_with("release/")
                && !path.contains("..")
                && (path.ends_with(".json") || path.ends_with(".md")) =>
        {
            paths.push(path.to_owned());
        }
        _ => {}
    }
}

fn ensure_release_steward_eligible(
    root: &Path,
    actor: &str,
    role: &str,
    assignment_id: &str,
    evaluation_time: &str,
    manifest_approver_actor: Option<&str>,
    require_distinct_manifest_approver: bool,
) -> Result<(), String> {
    if role != "release-steward" {
        return Err("detached approval requires the asserted release-steward role".to_owned());
    }
    let assignments = read_json(&root.join("release/stewardship/assignments.json"))?;
    validate_assignments(&assignments)?;
    let root = object(&assignments, "assignment record")?;
    let qualified = strings(
        object(
            root.get("continuity")
                .ok_or("assignment record has no continuity")?,
            "continuity",
        )?
        .get("qualifiedReleaseStewardActorIds"),
        "qualified Release Steward actor IDs",
    )?;
    if !qualified.contains(&actor) {
        return Err("approval actor is not a current qualified Release Steward".to_owned());
    }
    let assignment = array(root.get("assignments"), "assignments")?
        .iter()
        .filter_map(Value::as_object)
        .filter(|assignment| {
            assignment.get("assignmentId").and_then(Value::as_str) == Some(assignment_id)
        })
        .collect::<Vec<_>>();
    let [assignment] = assignment.as_slice() else {
        return Err("approval assignment is missing or ambiguous".to_owned());
    };
    if assignment.get("primaryActorId").and_then(Value::as_str) != Some(actor)
        || assignment.get("roleId").and_then(Value::as_str) != Some(role)
        || assignment.get("status").and_then(Value::as_str) != Some("active")
        || assignment
            .get("scope")
            .and_then(Value::as_object)
            .and_then(|scope| scope.get("kind"))
            .and_then(Value::as_str)
            != Some("release-manifest-lifecycle")
    {
        return Err("approval actor/role/assignment is not currently eligible".to_owned());
    }
    let effective = text(assignment.get("effectiveFrom"), "assignment effective date")?;
    if format!("{effective}T00:00:00Z").as_str() > evaluation_time {
        return Err("approval assignment is not effective at the evaluation time".to_owned());
    }
    if require_distinct_manifest_approver
        && qualified.len() > 1
        && manifest_approver_actor.is_none_or(|other| other == actor)
    {
        return Err(
            "multi-steward governance requires a distinct Manifest approver actor".to_owned(),
        );
    }
    Ok(())
}

fn ensure_active_assignment(
    root: &Path,
    actor: &str,
    role: &str,
    assignment_id: &str,
    scope_kind: &str,
    evaluation_time: &str,
    context: &str,
) -> Result<(), String> {
    let assignments = read_json(&root.join("release/stewardship/assignments.json"))?;
    validate_assignments(&assignments)?;
    let assignments = object(&assignments, "assignment record")?;
    let matches = array(assignments.get("assignments"), "assignments")?
        .iter()
        .filter_map(Value::as_object)
        .filter(|assignment| {
            assignment.get("assignmentId").and_then(Value::as_str) == Some(assignment_id)
                && assignment.get("primaryActorId").and_then(Value::as_str) == Some(actor)
                && assignment.get("roleId").and_then(Value::as_str) == Some(role)
                && assignment.get("status").and_then(Value::as_str) == Some("active")
                && assignment
                    .get("scope")
                    .and_then(Value::as_object)
                    .and_then(|scope| scope.get("kind"))
                    .and_then(Value::as_str)
                    == Some(scope_kind)
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "{context} actor/role/assignment is not currently eligible"
        ));
    }
    let effective = text(matches[0].get("effectiveFrom"), "assignment effective date")?;
    if format!("{effective}T00:00:00Z").as_str() > evaluation_time {
        return Err(format!(
            "{context} assignment is not effective at the evaluation time"
        ));
    }
    Ok(())
}

fn ensure_repository_administrator(
    root: &Path,
    actor: &str,
    role: &str,
    assignment_id: &str,
    evaluation_time: &str,
) -> Result<(), String> {
    if role != "repository-administrator" {
        return Err(
            "approval disposition requires a repository-administrator authority".to_owned(),
        );
    }
    let assignments = read_json(&root.join("release/stewardship/assignments.json"))?;
    validate_assignments(&assignments)?;
    let assignments = object(&assignments, "assignment record")?;
    let matches = array(assignments.get("assignments"), "assignments")?
        .iter()
        .filter_map(Value::as_object)
        .filter(|assignment| {
            assignment.get("assignmentId").and_then(Value::as_str) == Some(assignment_id)
                && assignment.get("primaryActorId").and_then(Value::as_str) == Some(actor)
                && assignment.get("roleId").and_then(Value::as_str) == Some(role)
                && assignment.get("status").and_then(Value::as_str) == Some("active")
        })
        .collect::<Vec<_>>();
    let [assignment] = matches.as_slice() else {
        return Err("approval disposition authority is missing or ambiguous".to_owned());
    };
    if format!(
        "{}T00:00:00Z",
        text(assignment.get("effectiveFrom"), "assignment effective date")?
    )
    .as_str()
        > evaluation_time
    {
        return Err("approval disposition authority is not effective".to_owned());
    }
    Ok(())
}

fn valid_protected_main_merge_evidence(
    approval: &Map<String, Value>,
    approval_bytes: &[u8],
    evaluation_time: &str,
    merge: &ApprovalMergeEvidence<'_>,
) -> Result<bool, String> {
    let manifest_id = text(approval.get("manifestId"), "approval Manifest ID")?;
    let approval_id = text(approval.get("approvalId"), "approval ID")?;
    let expected_path = format!("release/manifests/{manifest_id}/approvals/{approval_id}.json");
    Ok(merge.repository == "vexil-lang/vexil"
        && merge.reference == "refs/heads/main"
        && valid_git_object_id(merge.commit_id)
        && valid_git_object_id(merge.tree_id)
        && valid_git_object_id(merge.blob_id)
        && merge.approval_path == expected_path
        && merge.observed_blob_bytes == approval_bytes
        && merge.approval_digest == sha256_hex(approval_bytes)
        && !merge.merge_or_pr_id.is_empty()
        && !merge.collector.is_empty()
        && merge.observed_at == evaluation_time
        && is_valid_utc_second(merge.observed_at))
}

fn valid_detached_approval_dependencies(
    root: &Path,
    approval: &Map<String, Value>,
    merge: &ApprovalMergeEvidence<'_>,
) -> Result<bool, String> {
    let manifest = match parse_canonical_json_bytes(merge.manifest_bytes, "merged Release Manifest")
    {
        Ok(value) if validate_canonical_release_record_schema(root, &value).is_ok() => value,
        _ => return Ok(false),
    };
    let evidence_set = match parse_canonical_json_bytes(
        merge.evidence_set_bytes,
        "merged reviewed evidence set",
    ) {
        Ok(value) if validate_canonical_release_record_schema(root, &value).is_ok() => value,
        _ => return Ok(false),
    };
    let Ok(manifest) = object(&manifest, "merged Release Manifest") else {
        return Ok(false);
    };
    let Ok(evidence_set) = object(&evidence_set, "merged reviewed evidence set") else {
        return Ok(false);
    };
    if text(manifest.get("recordKind"), "merged Release Manifest kind")? != "release-manifest"
        || text(evidence_set.get("recordKind"), "merged evidence-set kind")?
            != "release-evidence-set"
        || validate_evidence_set_entries(root, evidence_set).is_err()
    {
        return Ok(false);
    }
    Ok(approval.get("manifestId") == manifest.get("manifestId")
        && approval.get("manifestDigest").and_then(Value::as_str)
            == Some(sha256_hex(merge.manifest_bytes).as_str())
        && approval.get("evidenceSetId") == manifest.get("evidenceSetId")
        && approval.get("evidenceSetDigest") == manifest.get("evidenceSetDigest")
        && approval.get("evidenceSetId") == evidence_set.get("evidenceSetId")
        && approval.get("evidenceSetDigest").and_then(Value::as_str)
            == Some(sha256_hex(merge.evidence_set_bytes).as_str()))
}

fn valid_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_canonical_json_bytes(bytes: &[u8], label: &str) -> Result<Value, String> {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) || bytes.contains(&b'\r') || !bytes.ends_with(b"\n") {
        return Err(format!("{label} has invalid raw-byte profile"));
    }
    let value: Value =
        serde_json::from_slice(bytes).map_err(|error| format!("parse {label}: {error}"))?;
    if canonical_json_bytes(&value)? != bytes {
        return Err(format!(
            "{label} is parse-equivalent but not canonically encoded"
        ));
    }
    Ok(value)
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, String> {
    let mut bytes =
        serde_json::to_vec(value).map_err(|error| format!("encode canonical JSON: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Validates the inputs that an isolated, non-publishing candidate build may
/// consume. It neither creates a workspace nor invokes a build tool, network,
/// credential, adapter, tag, registry, or provider operation.
pub fn prepare_isolated_candidate_build(
    root: &Path,
    manifest_bytes: &[u8],
) -> Result<IsolatedCandidateBuildPlan, String> {
    validate_manifest_clean_worktree(root)?;
    let manifest = parse_canonical_json_bytes(manifest_bytes, "candidate-build Manifest")?;
    validate_canonical_release_record_schema(root, &manifest)?;
    let manifest = object(&manifest, "candidate-build Manifest")?;
    if text(manifest.get("recordKind"), "candidate-build Manifest kind")? != "release-manifest"
        || text(
            manifest.get("schemaVersion"),
            "candidate-build Manifest schema version",
        )? != "1.1"
    {
        return Err("candidate build requires a retained release-manifest@1.1".to_owned());
    }
    let base_commit = text(manifest.get("baseCommit"), "candidate-build base commit")?;
    validate_reviewed_commit(root, base_commit, "candidate-build base commit")?;
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("resolve candidate-build HEAD: {error}"))?;
    if !head.status.success() || String::from_utf8_lossy(&head.stdout).trim() != base_commit {
        return Err(
            "candidate build requires a clean checkout at the Manifest base commit".to_owned(),
        );
    }
    let mut source_commits = BTreeMap::new();
    for unit in array(
        manifest.get("releaseUnits"),
        "candidate-build Manifest units",
    )? {
        let unit = object(unit, "candidate-build Manifest unit")?;
        let unit_id = text(unit.get("unitId"), "candidate-build unit ID")?;
        let source_commit = text(
            unit.get("sourceCommit"),
            "candidate-build unit source commit",
        )?;
        validate_reviewed_commit(root, source_commit, "candidate-build unit source commit")?;
        if source_commits
            .insert(unit_id.to_owned(), source_commit.to_owned())
            .is_some()
        {
            return Err(format!("candidate build has duplicate unit {unit_id}"));
        }
    }
    Ok(IsolatedCandidateBuildPlan {
        manifest_id: text(manifest.get("manifestId"), "candidate-build Manifest ID")?.to_owned(),
        manifest_digest: sha256_hex(manifest_bytes),
        base_commit: base_commit.to_owned(),
        source_commits,
    })
}

/// Materializes one detached, clean Git worktree for every exact Release Unit
/// in a canonical Manifest. The caller supplies a new, absolute workspace
/// root outside the reviewed checkout and owns removal after inspection. This
/// function does not build, package, attest, contact a network, load
/// credentials, or create any release/provider effect.
pub fn materialize_isolated_candidate_workspaces(
    root: &Path,
    manifest_bytes: &[u8],
    workspace_root: &Path,
) -> Result<Vec<IsolatedCandidateWorkspace>, String> {
    let plan = prepare_isolated_candidate_build(root, manifest_bytes)?;
    if !workspace_root.is_absolute() {
        return Err("candidate workspace root must be an absolute path".to_owned());
    }
    if workspace_root.exists() {
        return Err("candidate workspace root must not already exist".to_owned());
    }
    let source_root = root
        .canonicalize()
        .map_err(|error| format!("resolve reviewed source root: {error}"))?;
    let workspace_parent = workspace_root
        .parent()
        .ok_or_else(|| "candidate workspace root must have a parent directory".to_owned())?
        .canonicalize()
        .map_err(|error| format!("resolve candidate workspace parent: {error}"))?;
    let resolved_workspace_root = workspace_parent.join(
        workspace_root
            .file_name()
            .ok_or_else(|| "candidate workspace root must name one directory".to_owned())?,
    );
    if resolved_workspace_root.starts_with(&source_root) {
        return Err("candidate workspace root must be outside the reviewed checkout".to_owned());
    }

    fs::create_dir(workspace_root)
        .map_err(|error| format!("create candidate workspace root: {error}"))?;
    let mut workspaces = Vec::new();
    for (unit_id, source_commit) in &plan.source_commits {
        let path = workspace_root.join(unit_id);
        let result = Command::new("git")
            .current_dir(root)
            .args(["worktree", "add", "--detach"])
            .arg(&path)
            .arg(source_commit)
            .output()
            .map_err(|error| format!("create candidate workspace for {unit_id}: {error}"));
        match result {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                cleanup_isolated_candidate_workspaces(root, &workspaces);
                let _ = fs::remove_dir_all(workspace_root);
                return Err(format!(
                    "create candidate workspace for {unit_id}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            Err(error) => {
                cleanup_isolated_candidate_workspaces(root, &workspaces);
                let _ = fs::remove_dir_all(workspace_root);
                return Err(error);
            }
        }
        if let Err(error) = validate_isolated_candidate_workspace(&path, source_commit) {
            let failed = IsolatedCandidateWorkspace {
                unit_id: unit_id.clone(),
                source_commit: source_commit.clone(),
                path,
            };
            cleanup_isolated_candidate_workspaces(root, &workspaces);
            cleanup_isolated_candidate_workspaces(root, std::slice::from_ref(&failed));
            let _ = fs::remove_dir_all(workspace_root);
            return Err(error);
        }
        workspaces.push(IsolatedCandidateWorkspace {
            unit_id: unit_id.clone(),
            source_commit: source_commit.clone(),
            path,
        });
    }
    Ok(workspaces)
}

fn cleanup_isolated_candidate_workspaces(root: &Path, workspaces: &[IsolatedCandidateWorkspace]) {
    for workspace in workspaces {
        let _ = Command::new("git")
            .current_dir(root)
            .args(["worktree", "remove", "--force"])
            .arg(&workspace.path)
            .output();
    }
}

fn validate_isolated_candidate_workspace(path: &Path, source_commit: &str) -> Result<(), String> {
    validate_manifest_clean_worktree(path)?;
    let output = Command::new("git")
        .current_dir(path)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("resolve isolated candidate checkout HEAD: {error}"))?;
    if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim() != source_commit {
        return Err(
            "isolated candidate checkout does not match its exact source commit".to_owned(),
        );
    }
    for private_path in ["_bmad", ".agents", "_bmad-output"] {
        if path.join(private_path).exists() {
            return Err(format!(
                "isolated candidate checkout contains prohibited private path {private_path}"
            ));
        }
    }
    Ok(())
}

/// Builds one exact Release Manifest from reviewed inputs. A successful result
/// contains only canonical UTF-8/LF JSON bytes and their external SHA-256
/// identity; it never writes a public record or creates release authority.
pub fn generate_release_manifest(
    root: &Path,
    request: &ReleaseManifestRequest<'_>,
) -> Result<GeneratedReleaseManifest, ManifestGenerationError> {
    let mut diagnostics = Vec::new();
    let manifest = request.manifest;

    if let Err(error) = validate_canonical_release_record_schema(root, manifest) {
        push_manifest_diagnostic(
            &mut diagnostics,
            "schema-valid-manifest",
            "release/schemas/release-manifest-1.1.schema.json",
            error,
        );
    }
    if manifest.get("schemaVersion").and_then(Value::as_str) != Some("1.1") {
        push_manifest_diagnostic(
            &mut diagnostics,
            "manifest-generation-schema",
            "manifest.schemaVersion",
            "Manifest generation requires the retained release-manifest@1.1 contract".to_owned(),
        );
    }
    if let Err(error) = ensure_no_private_leakage(&manifest.to_string()) {
        push_manifest_diagnostic(&mut diagnostics, "public-inputs-only", "manifest", error);
    }
    if let Err(error) = validate_canonical_reference_paths(manifest) {
        push_manifest_diagnostic(&mut diagnostics, "public-inputs-only", "manifest", error);
    }
    if let Ok(manifest) = object(manifest, "Release Manifest") {
        if let Err(error) = validate_retained_state_artifacts(root, manifest) {
            push_manifest_diagnostic(
                &mut diagnostics,
                "retained-state-artifacts",
                "manifest.stateSchema/reducer",
                error,
            );
        }
    }

    let evidence_set =
        match parse_canonical_json_bytes(request.evidence_set_bytes, "reviewed evidence set") {
            Ok(evidence_set) => Some(evidence_set),
            Err(error) => {
                push_manifest_diagnostic(
                    &mut diagnostics,
                    "canonical-evidence-set",
                    "evidence-set.json",
                    error,
                );
                None
            }
        };
    if let Some(evidence_set) = evidence_set.as_ref() {
        if let Err(error) = ensure_no_private_leakage(&evidence_set.to_string()) {
            push_manifest_diagnostic(
                &mut diagnostics,
                "public-inputs-only",
                "evidence-set.json",
                error,
            );
        }
        if let Err(error) = validate_canonical_reference_paths(evidence_set) {
            push_manifest_diagnostic(
                &mut diagnostics,
                "public-inputs-only",
                "evidence-set.json",
                error,
            );
        }
        if let Err(error) = validate_canonical_release_record_schema(root, evidence_set) {
            push_manifest_diagnostic(
                &mut diagnostics,
                "schema-valid-evidence-set",
                "evidence-set.json",
                error,
            );
        }
        if let Ok(evidence_set) = object(evidence_set, "reviewed evidence set") {
            match text(
                evidence_set.get("evidenceSetId"),
                "reviewed evidence-set ID",
            ) {
                Ok(evidence_set_id) => {
                    let materialized = root
                        .join("release/evidence-sets")
                        .join(evidence_set_id)
                        .join("evidence-set.json");
                    match fs::read(&materialized) {
                        Ok(bytes) if bytes == request.evidence_set_bytes => {}
                        Ok(_) => push_manifest_diagnostic(
                            &mut diagnostics,
                            "reviewed-evidence-binding",
                            materialized.display().to_string(),
                            "supplied reviewed evidence-set bytes do not match the canonical public materialization".to_owned(),
                        ),
                        Err(error) => push_manifest_diagnostic(
                            &mut diagnostics,
                            "reviewed-evidence-binding",
                            materialized.display().to_string(),
                            format!("canonical reviewed evidence-set materialization is missing or unreadable: {error}"),
                        ),
                    }
                }
                Err(error) => push_manifest_diagnostic(
                    &mut diagnostics,
                    "reviewed-evidence-binding",
                    "evidence-set.json",
                    error,
                ),
            }
            if let Err(error) = validate_evidence_set_entries(root, evidence_set) {
                push_manifest_diagnostic(
                    &mut diagnostics,
                    "reviewed-evidence-binding",
                    "evidence-set.json",
                    error,
                );
            }
            if let Ok(manifest) = object(manifest, "Release Manifest") {
                let expected_id = evidence_set.get("evidenceSetId").and_then(Value::as_str);
                let expected_digest = sha256_hex(request.evidence_set_bytes);
                if manifest.get("evidenceSetId").and_then(Value::as_str) != expected_id {
                    push_manifest_diagnostic(
                        &mut diagnostics,
                        "reviewed-evidence-binding",
                        "manifest.evidenceSetId",
                        "Manifest does not bind the supplied reviewed evidence-set ID".to_owned(),
                    );
                }
                if manifest.get("evidenceSetDigest").and_then(Value::as_str)
                    != Some(expected_digest.as_str())
                {
                    push_manifest_diagnostic(
                        &mut diagnostics,
                        "reviewed-evidence-binding",
                        "manifest.evidenceSetDigest",
                        "Manifest does not bind the supplied reviewed evidence-set digest"
                            .to_owned(),
                    );
                }
                if let Err(error) = validate_manifest_evidence_coverage(manifest, evidence_set) {
                    push_manifest_diagnostic(
                        &mut diagnostics,
                        "complete-reviewed-evidence",
                        "evidence-set.json",
                        error,
                    );
                }
                if let Err(error) =
                    validate_manifest_live_tag_observation(root, manifest, evidence_set)
                {
                    push_manifest_diagnostic(
                        &mut diagnostics,
                        "fresh-live-tag-observation",
                        "manifest.historicalTagSnapshot",
                        error,
                    );
                }
            }
        }
    }

    if let Err(error) = validate_manifest_release_units_all(root, manifest) {
        push_manifest_diagnostic(
            &mut diagnostics,
            "source-led-release-set",
            "release/catalog.json",
            error,
        );
    }
    if let Err(error) = validate_manifest_change_units(root, manifest) {
        push_manifest_diagnostic(
            &mut diagnostics,
            "checkpoint-change-unit-binding",
            "manifest.releaseUnits.changeUnits",
            error,
        );
    }
    if let Err(error) = validate_manifest_security(root, manifest) {
        push_manifest_diagnostic(
            &mut diagnostics,
            "manifest-bound-security-gate",
            "manifest.security",
            error,
        );
    }
    if let Err(error) = validate_manifest_clean_worktree(root) {
        push_manifest_diagnostic(
            &mut diagnostics,
            "clean-reviewed-source",
            "git-status",
            error,
        );
    }
    if let Err(error) = validate_manifest_supersession(root, manifest) {
        push_manifest_diagnostic(
            &mut diagnostics,
            "immutable-supersession",
            "release/manifests",
            error,
        );
    }
    if let Err(error) = validate_manifest_id_is_unmaterialized(root, manifest) {
        push_manifest_diagnostic(
            &mut diagnostics,
            "immutable-manifest-identity",
            "release/manifests",
            error,
        );
    }

    diagnostics.sort();
    diagnostics.dedup();
    if !diagnostics.is_empty() {
        return Err(ManifestGenerationError { diagnostics });
    }

    let bytes = canonical_json_bytes(manifest).map_err(|error| ManifestGenerationError {
        diagnostics: vec![ManifestGenerationDiagnostic {
            requirement: "canonical-emission",
            source: "manifest".to_owned(),
            message: error,
        }],
    })?;
    Ok(GeneratedReleaseManifest {
        external_digest: format!("sha256:{}", sha256_hex(&bytes)),
        bytes,
    })
}

fn push_manifest_diagnostic(
    diagnostics: &mut Vec<ManifestGenerationDiagnostic>,
    requirement: &'static str,
    source: impl Into<String>,
    message: String,
) {
    diagnostics.push(ManifestGenerationDiagnostic {
        requirement,
        source: source.into(),
        message,
    });
}

fn validate_manifest_clean_worktree(root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("inspect reviewed public worktree: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "inspect reviewed public worktree: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let dirty = String::from_utf8(output.stdout)
        .map_err(|error| format!("decode reviewed public worktree status: {error}"))?;
    if dirty.trim().is_empty() {
        Ok(())
    } else {
        Err(format!(
            "reviewed public worktree is modified or contains untracked source: {}",
            dirty.trim().replace('\n', ", ")
        ))
    }
}

fn validate_reviewed_commit(root: &Path, commit: &str, label: &str) -> Result<(), String> {
    if !valid_git_object_id(commit) || commit.len() != 40 {
        return Err(format!(
            "{label} must be a lowercase full 40-hex source commit"
        ));
    }
    let output = Command::new("git")
        .args(["cat-file", "-e", &format!("{commit}^{{commit}}")])
        .current_dir(root)
        .output()
        .map_err(|error| format!("resolve {label}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{label} does not resolve to a commit in the reviewed repository"
        ));
    }
    Ok(())
}

fn validate_manifest_evidence_coverage(
    manifest: &Map<String, Value>,
    evidence_set: &Map<String, Value>,
) -> Result<(), String> {
    let entries = array(evidence_set.get("entries"), "reviewed evidence-set entries")?;
    let mut evidence = BTreeSet::new();
    for entry in entries {
        let entry = object(entry, "reviewed evidence-set entry")?;
        evidence.insert((
            text(entry.get("path"), "reviewed evidence-set entry path")?.to_owned(),
            text(
                entry.get("contentDigest"),
                "reviewed evidence-set entry digest",
            )?
            .to_owned(),
        ));
    }
    let mut bound_artifacts = BTreeSet::new();
    let mut errors = Vec::new();
    for (field, required_prefix) in [
        ("approvalPolicy", "release/policies/"),
        ("failurePolicy", "release/policies/"),
        ("recoveryPolicy", "release/policies/"),
        ("closeoutRequirements", "release/policies/"),
        ("security", "release/security/"),
        ("candidate", "release/candidates/"),
        ("rehearsal", "release/rehearsals/"),
        ("registryCustody", "release/identities/"),
        ("historicalTagSnapshot", "release/history/"),
        ("compatibilityEvidence", "release/evidence/"),
        ("stateSchema", "release/schemas/"),
        ("reducer", "release/reducers/"),
    ] {
        let artifact = match required_value(manifest, field)
            .and_then(|value| object(value, "Manifest reviewed immutable evidence identity"))
        {
            Ok(artifact) => artifact,
            Err(error) => {
                errors.push(format!("Manifest {field} evidence: {error}"));
                continue;
            }
        };
        let identity = (
            text(
                artifact.get("id"),
                "Manifest reviewed immutable evidence path",
            )?
            .to_owned(),
            text(
                artifact.get("digest"),
                "Manifest reviewed immutable evidence digest",
            )?
            .to_owned(),
        );
        if !identity.0.starts_with(required_prefix) {
            errors.push(format!(
                "Manifest {field} evidence must be materialized under {required_prefix}: {}",
                identity.0
            ));
        }
        if !bound_artifacts.insert(identity.clone()) {
            errors.push(format!(
                "Manifest reuses one reviewed evidence identity for multiple required gates: {}",
                identity.0
            ));
        }
        if !evidence.contains(&identity) {
            errors.push(format!(
                "Manifest required evidence is absent from its reviewed evidence set: {}",
                identity.0
            ));
        }
    }
    for release_unit in array(manifest.get("releaseUnits"), "Manifest release units")? {
        let release_unit = object(release_unit, "Manifest release unit")?;
        let rationale = match release_unit
            .get("versionRationale")
            .ok_or_else(|| "Manifest release unit has no version rationale".to_owned())
            .and_then(|value| object(value, "Manifest version rationale"))
        {
            Ok(rationale) => rationale,
            Err(error) => {
                errors.push(error.to_owned());
                continue;
            }
        };
        let rationale_identity = (
            format!(
                "release/rationales/{}.json",
                text(rationale.get("id"), "Manifest version rationale ID")?
            ),
            text(rationale.get("digest"), "Manifest version rationale digest")?.to_owned(),
        );
        if !evidence.contains(&rationale_identity) {
            errors.push(format!(
                "Manifest version rationale is absent from its reviewed evidence set: {}",
                rationale_identity.0
            ));
        }
        for change_unit in array(release_unit.get("changeUnits"), "Manifest Change Units")? {
            let change_unit = object(change_unit, "Manifest Change Unit")?;
            let identity = (
                format!(
                    "release/checkpoint-change-units/{}.json",
                    text(change_unit.get("id"), "Manifest Change Unit ID")?
                ),
                text(change_unit.get("digest"), "Manifest Change Unit digest")?.to_owned(),
            );
            if !evidence.contains(&identity) {
                errors.push(format!(
                    "Manifest Change Unit is absent from its reviewed evidence set: {}",
                    identity.0
                ));
            }
        }
    }
    errors.sort();
    errors.dedup();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// A Manifest may rely on a tag observation only when its exact, public
/// observation record remains valid for the time that reviewed evidence was
/// accepted. Historic observations remain useful evidence, but cannot silently
/// become fresh release authority merely because they are still present.
fn validate_manifest_live_tag_observation(
    root: &Path,
    manifest: &Map<String, Value>,
    evidence_set: &Map<String, Value>,
) -> Result<(), String> {
    let snapshot = object(
        required_value(manifest, "historicalTagSnapshot")?,
        "Manifest historical tag snapshot identity",
    )?;
    let path = text(snapshot.get("id"), "Manifest historical tag snapshot path")?;
    if !path.starts_with("release/history/observations/") || !path.ends_with(".json") {
        return Err(
            "Manifest historicalTagSnapshot must name a public history observation record"
                .to_owned(),
        );
    }
    let bytes = fs::read(root.join(path))
        .map_err(|error| format!("read Manifest historical tag observation {path}: {error}"))?;
    let digest = text(
        snapshot.get("digest"),
        "Manifest historical tag snapshot digest",
    )?;
    if sha256_hex(&bytes) != digest {
        return Err(
            "Manifest historicalTagSnapshot digest does not match its exact public observation bytes"
                .to_owned(),
        );
    }
    let observation = parse_canonical_json_bytes(&bytes, "Manifest historical tag observation")?;
    validate_schema_instance(
        root,
        "release/schemas/history-observation.schema.json",
        &observation,
        "Manifest historical tag observation",
    )?;
    let observation = object(&observation, "Manifest historical tag observation")?;
    if text(
        observation.get("state"),
        "Manifest historical tag observation state",
    )? != "observed"
    {
        return Err(
            "Manifest historicalTagSnapshot must be an observed live-tag observation".to_owned(),
        );
    }
    let observed_at = text(
        observation.get("observedAt"),
        "Manifest historical tag observation time",
    )?;
    let valid_until = text(
        observation.get("validUntil"),
        "Manifest historical tag observation validity",
    )?;
    let reviewed_at = text(
        evidence_set.get("reviewedAt"),
        "Manifest reviewed evidence-set time",
    )?;
    let observed_at = utc_second_timestamp(observed_at)?;
    let valid_until = utc_second_timestamp(valid_until)?;
    let reviewed_at = utc_second_timestamp(reviewed_at)?;
    if observed_at > reviewed_at {
        return Err(
            "Manifest historicalTagSnapshot was observed after its evidence set was reviewed"
                .to_owned(),
        );
    }
    if valid_until < reviewed_at {
        return Err(
            "Manifest historicalTagSnapshot validUntil does not cover the evidence-set review time"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_manifest_change_units(root: &Path, manifest: &Value) -> Result<(), String> {
    let manifest = object(manifest, "Release Manifest")?;
    let mut identifiers = BTreeSet::new();
    let mut errors = Vec::new();
    for unit in array(manifest.get("releaseUnits"), "Manifest release units")? {
        let unit = object(unit, "Manifest release unit")?;
        let unit_id = text(unit.get("unitId"), "Manifest release unit ID")?;
        for reference in array(unit.get("changeUnits"), "Manifest Change Units")? {
            let reference = match object(reference, "Manifest Change Unit reference") {
                Ok(reference) => reference,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            let id = match text(reference.get("id"), "Manifest Change Unit ID") {
                Ok(id) => id,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            if !identifiers.insert(id.to_owned()) {
                errors.push(format!(
                    "Manifest Change Unit is reused by more than one release unit: {id}"
                ));
                continue;
            }
            let path = root
                .join("release/checkpoint-change-units")
                .join(format!("{id}.json"));
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    errors.push(format!(
                        "Manifest Change Unit is missing from public records for {unit_id}: {id}: {error}"
                    ));
                    continue;
                }
            };
            let digest = match text(reference.get("digest"), "Manifest Change Unit digest") {
                Ok(digest) => digest,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            if sha256_hex(&bytes) != digest {
                errors.push(format!(
                    "Manifest Change Unit digest does not match its exact public record bytes: {id}"
                ));
                continue;
            }
            let record = match parse_canonical_json_bytes(&bytes, "Manifest Change Unit") {
                Ok(record) => record,
                Err(error) => {
                    errors.push(format!("Manifest Change Unit {id}: {error}"));
                    continue;
                }
            };
            if let Err(error) = validate_schema_instance(
                root,
                "release/schemas/checkpoint-change-unit-1.0.schema.json",
                &record,
                "Manifest Change Unit",
            ) {
                errors.push(format!("Manifest Change Unit {id}: {error}"));
                continue;
            }
            match object(&record, "Manifest Change Unit")
                .and_then(|record| text(record.get("changeUnitId"), "checkpoint Change Unit ID"))
            {
                Ok(record_id) if record_id == id => {}
                Ok(_) => errors.push(format!(
                    "Manifest Change Unit record identity does not match its filename reference: {id}"
                )),
                Err(error) => errors.push(format!("Manifest Change Unit {id}: {error}")),
            }
        }
    }
    errors.sort();
    errors.dedup();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn validate_manifest_security(root: &Path, manifest: &Value) -> Result<(), String> {
    let manifest = object(manifest, "Release Manifest")?;
    let security = object(
        required_value(manifest, "security")?,
        "Manifest security identity",
    )?;
    let path = text(security.get("id"), "Manifest security path")?;
    if !path.starts_with("release/security/scans/") || !path.ends_with(".json") {
        return Err("Manifest security must name a retained public security scan".to_owned());
    }
    let bytes = fs::read(root.join(path))
        .map_err(|error| format!("read Manifest security scan {path}: {error}"))?;
    if sha256_hex(&bytes) != text(security.get("digest"), "Manifest security digest")? {
        return Err("Manifest security digest does not match exact public scan bytes".to_owned());
    }
    let scan = parse_canonical_json_bytes(&bytes, "Manifest security scan")?;
    validate_schema_instance(
        root,
        "release/schemas/security-scan-1.0.schema.json",
        &scan,
        "Manifest security scan",
    )?;
    let scan = object(&scan, "Manifest security scan")?;
    let scope = object(
        scan.get("scope").ok_or("security scan has no scope")?,
        "security scan scope",
    )?;
    let lockfile = text(scope.get("lockfile"), "security scan lockfile")?;
    let expected = format!(
        "sha256:{}",
        sha256_hex(
            &fs::read(root.join(lockfile))
                .map_err(|error| format!("read security scan lockfile: {error}"))?
        )
    );
    if scan.get("lockfileDigest").and_then(Value::as_str) != Some(expected.as_str()) {
        return Err("security scan lockfile digest is stale".to_owned());
    }
    for finding in array(scan.get("findings"), "security scan findings")? {
        let finding = object(finding, "security finding")?;
        let severity = text(finding.get("severity"), "security finding severity")?;
        let status = text(finding.get("status"), "security finding status")?;
        if matches!(severity, "high" | "critical") && status != "remediated" {
            return Err(format!(
                "unresolved {severity} security finding blocks Manifest: {}",
                text(finding.get("id"), "security finding ID")?
            ));
        }
    }
    Ok(())
}

fn validate_manifest_id_is_unmaterialized(root: &Path, manifest: &Value) -> Result<(), String> {
    let manifest = object(manifest, "Release Manifest")?;
    let manifest_id = text(manifest.get("manifestId"), "Manifest ID")?;
    let materialized = root
        .join("release/manifests")
        .join(manifest_id)
        .join("manifest.json");
    if materialized.exists() {
        return Err(format!(
            "Manifest ID already has immutable materialized bytes: {manifest_id}"
        ));
    }
    Ok(())
}

fn validate_manifest_release_units_all(root: &Path, manifest: &Value) -> Result<(), String> {
    let manifest_object = object(manifest, "Release Manifest")?;
    let units = array(
        manifest_object.get("releaseUnits"),
        "Manifest release units",
    )?;
    let mut errors = Vec::new();
    for unit in units {
        let Some(unit_id) = unit.get("unitId").cloned() else {
            continue;
        };
        let mut isolated = manifest.clone();
        let isolated = isolated
            .as_object_mut()
            .ok_or("Release Manifest must be an object")?;
        isolated.insert("releaseUnits".to_owned(), Value::Array(vec![unit.clone()]));
        isolated.insert("publicationOrder".to_owned(), Value::Array(vec![unit_id]));
        if let Err(error) = validate_manifest_release_units(root, &Value::Object(isolated.clone()))
        {
            errors.push(error);
        }
    }
    if let Err(error) = validate_manifest_release_units(root, manifest) {
        errors.push(error);
    }
    errors.sort();
    errors.dedup();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn validate_manifest_release_units(root: &Path, manifest: &Value) -> Result<(), String> {
    let manifest = object(manifest, "Release Manifest")?;
    for field in [
        "approvalPolicy",
        "failurePolicy",
        "recoveryPolicy",
        "closeoutRequirements",
        "security",
        "candidate",
        "rehearsal",
        "registryCustody",
        "historicalTagSnapshot",
        "compatibilityEvidence",
    ] {
        let artifact = object(
            required_value(manifest, field)?,
            "Manifest reviewed immutable evidence identity",
        )?;
        for required in ["id", "version", "digest"] {
            text(
                artifact.get(required),
                "Manifest reviewed immutable evidence identity field",
            )?;
        }
    }
    let base_commit = text(manifest.get("baseCommit"), "Manifest base commit")?;
    validate_reviewed_commit(root, base_commit, "Manifest baseCommit")?;
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("resolve reviewed public HEAD: {error}"))?;
    if !head.status.success() || String::from_utf8_lossy(&head.stdout).trim() != base_commit {
        return Err("Manifest baseCommit must match the clean reviewed public HEAD".to_owned());
    }
    let catalog = read_json(&root.join("release/catalog.json"))?;
    let lifecycle = read_json(&root.join("release/catalog-lifecycle.json"))?;
    validate_catalog_lifecycle(root, &catalog, &lifecycle)?;
    let release_order = derive_release_order(root, &catalog)?;
    let catalog = object(&catalog, "release catalog")?;
    let catalog_units = array(catalog.get("units"), "release catalog units")?;
    let mut by_id = BTreeMap::new();
    for unit in catalog_units {
        let unit = object(unit, "release catalog unit")?;
        by_id.insert(
            text(unit.get("id"), "release catalog unit ID")?.to_owned(),
            unit,
        );
    }
    let units = array(manifest.get("releaseUnits"), "Manifest release units")?;
    let mut selected = BTreeSet::new();
    let mut release_unit_order = Vec::new();
    for release_unit in units {
        let release_unit = object(release_unit, "Manifest release unit")?;
        let unit_id = text(release_unit.get("unitId"), "Manifest release unit ID")?;
        if !selected.insert(unit_id.to_owned()) {
            return Err(format!(
                "Manifest selects release unit more than once: {unit_id}"
            ));
        }
        release_unit_order.push(unit_id.to_owned());
        let source_commit = text(
            release_unit.get("sourceCommit"),
            "Manifest unit source commit",
        )?;
        validate_reviewed_commit(
            root,
            source_commit,
            &format!("Manifest release unit {unit_id} sourceCommit"),
        )?;
        let catalog_unit = by_id
            .get(unit_id)
            .ok_or_else(|| format!("Manifest selects missing catalog unit {unit_id}"))?;
        let publication = object(
            catalog_unit
                .get("publication")
                .ok_or("catalog unit has no publication")?,
            "catalog publication",
        )?;
        if publication.get("classification").and_then(Value::as_str)
            != Some("publishable-source-unit")
        {
            return Err(format!(
                "Manifest selects non-publishable catalog unit {unit_id}"
            ));
        }
        let version_source = object(
            release_unit
                .get("versionSource")
                .ok_or("Manifest release unit has no version source")?,
            "Manifest version source",
        )?;
        let catalog_version_source = object(
            catalog_unit
                .get("versionSource")
                .ok_or("catalog unit has no version source")?,
            "catalog version source",
        )?;
        for field in ["path", "observedDeclaration"] {
            if version_source.get(field) != catalog_version_source.get(field) {
                return Err(format!(
                    "Manifest release unit {unit_id} {field} does not match source-led catalog authority"
                ));
            }
        }
        let version_path = text(version_source.get("path"), "Manifest version source path")?;
        let expected_source_bytes = fs::read(root.join(version_path)).map_err(|error| {
            format!("read current source-led version path {version_path}: {error}")
        })?;
        let source_version = Command::new("git")
            .args(["show", &format!("{source_commit}:{version_path}")])
            .current_dir(root)
            .output()
            .map_err(|error| {
                format!("read Manifest release unit {unit_id} version source at commit: {error}")
            })?;
        if !source_version.status.success() || source_version.stdout != expected_source_bytes {
            return Err(format!(
                "Manifest release unit {unit_id} sourceCommit does not contain the reviewed authoritative version source"
            ));
        }
        if release_unit.get("proposedVersion") != catalog_version_source.get("observedDeclaration")
        {
            return Err(format!(
                "Manifest release unit {unit_id} proposedVersion does not match its authoritative checked-in version"
            ));
        }
        let catalog_targets = array(catalog_unit.get("targets"), "catalog unit targets")?;
        let mut catalog_target_identities = BTreeSet::new();
        for target in catalog_targets {
            let target = object(target, "catalog target")?;
            catalog_target_identities.insert((
                text(target.get("kind"), "catalog target kind")?.to_owned(),
                text(target.get("name"), "catalog target name")?.to_owned(),
            ));
        }
        let mut manifest_target_identities = BTreeSet::new();
        let mut manifest_target_order = Vec::new();
        for target in array(release_unit.get("targets"), "Manifest release unit targets")? {
            let target = object(target, "Manifest release unit target")?;
            if target.get("mandatory").and_then(Value::as_bool) != Some(true) {
                return Err(format!(
                    "Manifest release unit {unit_id} cannot mark a source-led catalog target optional without explicit catalog authority"
                ));
            }
            let identity = (
                text(target.get("kind"), "Manifest target kind")?.to_owned(),
                text(target.get("name"), "Manifest target name")?.to_owned(),
            );
            if !catalog_target_identities.contains(&identity) {
                return Err(format!(
                    "Manifest release unit {unit_id} binds an unknown catalog target {}/{}",
                    identity.0, identity.1
                ));
            }
            if !manifest_target_identities.insert(identity) {
                return Err(format!(
                    "Manifest release unit {unit_id} binds a target more than once"
                ));
            }
            manifest_target_order.push((
                text(target.get("kind"), "Manifest target kind")?.to_owned(),
                text(target.get("name"), "Manifest target name")?.to_owned(),
            ));
        }
        if manifest_target_identities != catalog_target_identities {
            return Err(format!(
                "Manifest release unit {unit_id} targets do not exactly match source-led catalog targets"
            ));
        }
        if manifest_target_order
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(format!(
                "Manifest release unit {unit_id} targets must use ascending kind/name order"
            ));
        }
        let namespace = text(
            catalog_unit.get("canonicalTagNamespace"),
            "catalog canonical tag namespace",
        )?;
        let proposed_version = text(
            release_unit.get("proposedVersion"),
            "Manifest release unit proposed version",
        )?;
        let expected_tag = namespace.replace("<semver>", proposed_version);
        if release_unit.get("canonicalTag").and_then(Value::as_str) != Some(expected_tag.as_str()) {
            return Err(format!(
                "Manifest release unit {unit_id} canonicalTag does not match its catalog namespace"
            ));
        }
        validate_candidate_tag(root, &Value::Object(catalog.clone()), &expected_tag)?;
        let rationale = object(
            release_unit
                .get("versionRationale")
                .ok_or("Manifest release unit has no version rationale")?,
            "Manifest version rationale",
        )?;
        let rationale_id = text(rationale.get("id"), "Manifest version rationale ID")?;
        let rationale_path = root
            .join("release/rationales")
            .join(format!("{rationale_id}.json"));
        let rationale_bytes = fs::read(&rationale_path).map_err(|error| {
            format!("Manifest version rationale is missing or unreadable {rationale_id}: {error}")
        })?;
        if rationale.get("digest").and_then(Value::as_str)
            != Some(sha256_hex(&rationale_bytes).as_str())
        {
            return Err(format!(
                "Manifest version rationale digest does not match exact public bytes: {rationale_id}"
            ));
        }
        let rationale_record: Value = serde_json::from_slice(&rationale_bytes)
            .map_err(|error| format!("parse Manifest version rationale {rationale_id}: {error}"))?;
        validate_version_rationale(root, &Value::Object(catalog.clone()), &rationale_record)?;
        let rationale_record = object(&rationale_record, "Manifest version rationale record")?;
        if rationale_record.get("unitId").and_then(Value::as_str) != Some(unit_id)
            || rationale_record.get("proposedPackageVersion") != release_unit.get("proposedVersion")
            || rationale_record
                .get("previousPackageVersion")
                .and_then(Value::as_object)
                .and_then(|previous| previous.get("version"))
                != release_unit.get("previousVersion")
        {
            return Err(format!(
                "Manifest release unit {unit_id} does not match its authoritative version rationale"
            ));
        }
        let mut change_units = BTreeSet::new();
        let mut change_unit_order = Vec::new();
        for change_unit in array(release_unit.get("changeUnits"), "Manifest Change Units")? {
            let change_unit = object(change_unit, "Manifest Change Unit")?;
            let id = text(change_unit.get("id"), "Manifest Change Unit ID")?;
            let digest = text(change_unit.get("digest"), "Manifest Change Unit digest")?;
            if !change_units.insert((id.to_owned(), digest.to_owned())) {
                return Err(format!(
                    "Manifest release unit {unit_id} binds a Change Unit more than once"
                ));
            }
            change_unit_order.push((id.to_owned(), digest.to_owned()));
        }
        if change_unit_order.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(format!(
                "Manifest release unit {unit_id} Change Units must use ascending ID/digest order"
            ));
        }
    }
    let expected_order = release_order
        .into_iter()
        .filter(|unit_id| selected.contains(unit_id))
        .collect::<Vec<_>>();
    let actual_order = array(
        manifest.get("publicationOrder"),
        "Manifest publication order",
    )?
    .iter()
    .map(|value| text(Some(value), "Manifest publication-order unit").map(str::to_owned))
    .collect::<Result<Vec<_>, String>>()?;
    if actual_order != expected_order {
        return Err(
            "Manifest publicationOrder does not freeze the catalog's typed dependency order"
                .to_owned(),
        );
    }
    if release_unit_order != expected_order {
        return Err("Manifest releaseUnits must use the frozen typed publication order".to_owned());
    }
    Ok(())
}

fn validate_manifest_supersession(root: &Path, manifest: &Value) -> Result<(), String> {
    let manifest = object(manifest, "Release Manifest")?;
    let Some(supersedes) = manifest.get("supersedes") else {
        return Ok(());
    };
    if supersedes.is_null() {
        return Ok(());
    }
    let supersedes = object(supersedes, "Manifest supersedes reference")?;
    let manifest_id = text(manifest.get("manifestId"), "Manifest ID")?;
    let prior_id = text(supersedes.get("manifestId"), "superseded Manifest ID")?;
    if manifest_id == prior_id {
        return Err("a Manifest cannot supersede itself".to_owned());
    }
    let prior_path = root
        .join("release/manifests")
        .join(prior_id)
        .join("manifest.json");
    let root_canonical =
        fs::canonicalize(root).map_err(|error| format!("canonicalize repository root: {error}"))?;
    let prior_canonical = fs::canonicalize(&prior_path).map_err(|error| {
        format!("superseded Manifest is missing or unreadable {prior_id}: {error}")
    })?;
    if !prior_canonical.starts_with(&root_canonical) {
        return Err(format!(
            "superseded Manifest escapes the reviewed public repository: {prior_id}"
        ));
    }
    let prior_bytes = fs::read(&prior_path).map_err(|error| {
        format!("superseded Manifest is missing or unreadable {prior_id}: {error}")
    })?;
    let prior = parse_canonical_json_bytes(&prior_bytes, "superseded Manifest")?;
    validate_canonical_release_record(root, &prior, &prior_canonical)?;
    let prior = object(&prior, "superseded Manifest")?;
    if text(prior.get("recordKind"), "superseded Manifest kind")? != "release-manifest"
        || text(prior.get("manifestId"), "superseded Manifest ID")? != prior_id
    {
        return Err(format!(
            "superseded Manifest is not a canonical Manifest with its claimed identity: {prior_id}"
        ));
    }
    validate_canonical_record_location(root, &prior_canonical, "release-manifest", prior_id)?;
    if supersedes.get("manifestDigest").and_then(Value::as_str)
        != Some(sha256_hex(&prior_bytes).as_str())
    {
        return Err(format!(
            "superseded Manifest digest does not match exact immutable bytes: {prior_id}"
        ));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn validate_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let record = read_json(&root.join("release/stewardship.json"))?;
    validate_contract_schema(root, &record)?;
    validate_contract(&record)?;
    validate_documentation_parity(
        &record,
        &fs::read_to_string(root.join("docs/book/src/release/stewardship.md"))
            .map_err(|error| format!("read stewardship documentation: {error}"))?,
    )?;
    validate_assignments_repository(root)?;
    validate_responsibilities_repository(root)?;
    validate_privileged_operations_repository(root)?;
    validate_stewardship_exercises_repository(root)?;
    validate_external_controls_repository(root)?;
    validate_history_repository(root)?;
    validate_catalog_lifecycle_repository(root)?;
    validate_catalog_repository(root)?;
    validate_version_rationale_repository(root)?;
    if root.join(".git").exists() {
        validate_checkpoint_change_units_repository(root)?;
    }
    validate_canonical_release_records_repository(root)?;
    validate_public_boundary(root)?;
    Ok(())
}

/// Validates the non-promoting checkpoint inventory against its retained Git slice.
pub fn validate_checkpoint_change_units_repository(root: &Path) -> Result<(), String> {
    let directory = root.join("release/checkpoint-change-units");
    let mut seen_paths = BTreeSet::new();
    let mut records = 0usize;
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("read checkpoint Change Units: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("read checkpoint Change Unit: {error}"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            return Err(format!(
                "checkpoint Change Unit is not JSON: {}",
                path.display()
            ));
        }
        let record = read_json(&path)?;
        validate_schema_instance(
            root,
            "release/schemas/checkpoint-change-unit-1.0.schema.json",
            &record,
            "checkpoint Change Unit",
        )?;
        let record = object(&record, "checkpoint Change Unit")?;
        let base = text(record.get("baseCommit"), "checkpoint base commit")?;
        let checkpoint = text(record.get("checkpointCommit"), "checkpoint commit")?;
        let paths = array(record.get("paths"), "checkpoint Change Unit paths")?;
        let mut diff_paths = Vec::new();
        for change in paths {
            let change = object(change, "checkpoint Change Unit path")?;
            let before = text(change.get("beforePath"), "checkpoint before path")?;
            if !seen_paths.insert(before.to_owned()) {
                return Err(format!(
                    "checkpoint path belongs to more than one Change Unit: {before}"
                ));
            }
            diff_paths.push(before.to_owned());
            let before_blob = Command::new("git")
                .args(["rev-parse", &format!("{base}:{before}")])
                .current_dir(root)
                .output()
                .map_err(|error| format!("resolve checkpoint before blob: {error}"))?;
            if !before_blob.status.success()
                || change.get("beforeBlob").and_then(Value::as_str)
                    != Some(String::from_utf8_lossy(&before_blob.stdout).trim())
            {
                return Err(format!("checkpoint before blob does not match: {before}"));
            }
            let after_blob = match change.get("afterPath").and_then(Value::as_str) {
                Some(after) => Command::new("git")
                    .args(["rev-parse", &format!("{checkpoint}:{after}")])
                    .current_dir(root)
                    .output()
                    .map_err(|error| format!("resolve checkpoint after blob: {error}"))?,
                None => {
                    if change.get("afterBlob").and_then(Value::as_str)
                        != Some("0000000000000000000000000000000000000000")
                    {
                        return Err(format!(
                            "deleted checkpoint path has a nonzero after blob: {before}"
                        ));
                    }
                    continue;
                }
            };
            if !after_blob.status.success()
                || change.get("afterBlob").and_then(Value::as_str)
                    != Some(String::from_utf8_lossy(&after_blob.stdout).trim())
            {
                return Err(format!("checkpoint after blob does not match: {before}"));
            }
        }
        let output = Command::new("git")
            .args([
                "diff",
                "--no-ext-diff",
                "--binary",
                "--full-index",
                base,
                checkpoint,
                "--",
            ])
            .args(&diff_paths)
            .current_dir(root)
            .output()
            .map_err(|error| format!("recompute checkpoint Change Unit patch: {error}"))?;
        if !output.status.success()
            || record.get("patchDigest").and_then(Value::as_str)
                != Some(format!("sha256:{}", sha256_hex(&output.stdout)).as_str())
        {
            return Err(format!(
                "checkpoint Change Unit patch digest does not match: {}",
                path.display()
            ));
        }
        records += 1;
    }
    let expected = Command::new("git")
        .args([
            "diff",
            "--name-status",
            "-M100%",
            "11f315880415038ac6013d7ee4d378296cd51c5d",
            "d4099e8188f40603ebf52473d6543ce4a6054201",
        ])
        .current_dir(root)
        .output()
        .map_err(|error| format!("enumerate checkpoint paths: {error}"))?;
    let expected_paths = String::from_utf8_lossy(&expected.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            fields.next()?;
            let first = fields.next()?;
            Some(first.to_owned())
        })
        .collect::<BTreeSet<_>>();
    if !expected.status.success() || expected_paths != seen_paths {
        return Err(
            "checkpoint Change Units do not exactly cover the retained checkpoint diff".to_owned(),
        );
    }
    if records != 5 || seen_paths.len() != 28 {
        return Err(
            "checkpoint Change Units must retain exactly five records covering 28 paths".to_owned(),
        );
    }
    Ok(())
}

/// Validates retained, public canonical release-record families. This is a
/// structural/offline boundary only; it neither creates nor authorizes records.
pub fn validate_canonical_release_records_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let mut index = CanonicalRecordIndex::default();
    for relative in ["release/evidence-sets", "release/manifests", "release/runs"] {
        let directory = root.join(relative);
        if !directory.exists() {
            continue;
        }
        validate_canonical_release_record_tree(root, &directory, &mut index)?;
    }
    validate_canonical_record_references(root, &index)
}

fn validate_canonical_release_record_tree(
    root: &Path,
    directory: &Path,
    index: &mut CanonicalRecordIndex,
) -> Result<(), String> {
    let directory = fs::canonicalize(directory).map_err(|error| {
        format!(
            "canonicalize release-record directory {}: {error}",
            directory.display()
        )
    })?;
    if !index.visited_directories.insert(directory.clone()) {
        return Err(format!(
            "canonical release-record directory is revisited through a symlink or junction: {}",
            directory.display()
        ));
    }
    let root_canonical =
        fs::canonicalize(root).map_err(|error| format!("canonicalize repository root: {error}"))?;
    let mut entries = fs::read_dir(&directory)
        .map_err(|error| {
            format!(
                "read canonical release-record directory {}: {error}",
                directory.display()
            )
        })?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            format!(
                "enumerate canonical release-record directory {}: {error}",
                directory.display()
            )
        })?;
    entries.sort();
    for path in entries {
        let canonical = fs::canonicalize(&path).map_err(|error| {
            format!(
                "canonicalize release-record path {}: {error}",
                path.display()
            )
        })?;
        if !canonical.starts_with(&root_canonical) {
            return Err(format!(
                "canonical release-record path escapes repository root: {}",
                path.display()
            ));
        }
        if canonical.is_dir() {
            validate_canonical_release_record_tree(root, &canonical, index)?;
        } else if canonical
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let record = read_canonical_json(&canonical)?;
            validate_canonical_release_record(root, &record, &canonical)?;
            let (kind, id) = canonical_record_identity(&record)?;
            let kind = kind.to_owned();
            let id = id.to_owned();
            validate_canonical_record_location(root, &canonical, &kind, &id)?;
            let digest = format!(
                "{:x}",
                Sha256::digest(fs::read(&canonical).map_err(|error| {
                    format!(
                        "read canonical record {} for digest: {error}",
                        canonical.display()
                    )
                })?)
            );
            if index
                .records
                .insert(
                    (kind.clone(), id.clone()),
                    CanonicalRecord {
                        digest,
                        path: canonical,
                        value: record,
                    },
                )
                .is_some()
            {
                return Err(format!("duplicate canonical record identity: {kind}/{id}"));
            }
        } else if canonical
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            index
                .events
                .extend(validate_canonical_jsonl(root, &canonical)?);
        } else {
            return Err(format!(
                "canonical release-record directory contains unsupported file: {}",
                canonical.display()
            ));
        }
    }
    Ok(())
}

fn canonical_record_identity(record: &Value) -> Result<(&str, &str), String> {
    let object = object(record, "canonical release record")?;
    let kind = text(object.get("recordKind"), "canonical release record kind")?;
    let field = match kind {
        "release-manifest" => "manifestId",
        "release-evidence-set" => "evidenceSetId",
        "release-detached-approval" => "approvalId",
        "release-approval-disposition" => "dispositionId",
        "privileged-run-start-authorization" => "authorizationId",
        "release-adapter-result-envelope" => "adapterResultId",
        "release-run-evidence" => "evidenceId",
        "release-closeout" => "closeoutId",
        other => return Err(format!("{other} has no canonical file identity")),
    };
    Ok((
        kind,
        text(object.get(field), "canonical release record ID")?,
    ))
}

fn validate_canonical_record_location(
    root: &Path,
    path: &Path,
    kind: &str,
    id: &str,
) -> Result<(), String> {
    let root =
        fs::canonicalize(root).map_err(|error| format!("canonicalize repository root: {error}"))?;
    let relative = path.strip_prefix(&root).map_err(|_| {
        format!(
            "canonical release-record path escapes repository root: {}",
            path.display()
        )
    })?;
    let parts = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let matches = match kind {
        "release-evidence-set" => parts == ["release", "evidence-sets", id, "evidence-set.json"],
        "release-manifest" => parts == ["release", "manifests", id, "manifest.json"],
        "release-detached-approval" => {
            parts.len() == 5
                && parts[0] == "release"
                && parts[1] == "manifests"
                && parts[3] == "approvals"
                && parts[4] == format!("{id}.json")
        }
        "release-approval-disposition" => {
            parts.len() == 5
                && parts[0] == "release"
                && parts[1] == "manifests"
                && parts[3] == "approval-dispositions"
                && parts[4] == format!("{id}.json")
        }
        "privileged-run-start-authorization" => {
            parts.len() == 4
                && parts[0] == "release"
                && parts[1] == "runs"
                && parts[3] == "start-authorization.json"
        }
        "release-adapter-result-envelope" => {
            parts.len() == 6
                && parts[0] == "release"
                && parts[1] == "runs"
                && parts[3] == "evidence"
                && parts[4] == "adapter-results"
                && parts[5] == format!("{id}.json")
        }
        "release-run-evidence" => {
            parts.len() == 5
                && parts[0] == "release"
                && parts[1] == "runs"
                && parts[3] == "evidence"
                && parts[4] == format!("{id}.json")
        }
        "release-closeout" => {
            parts.len() == 4
                && parts[0] == "release"
                && parts[1] == "runs"
                && parts[3] == "closeout.json"
        }
        _ => unreachable!("canonical file identities were checked before path validation"),
    };
    if !matches {
        return Err(format!(
            "canonical {kind} record is not materialized at its required public location: {}",
            relative.display()
        ));
    }
    Ok(())
}

fn canonical_record_schema(kind: &str, version: &str) -> Option<(&'static str, &'static str)> {
    match (kind, version) {
        ("release-manifest", "1.0") => Some((
            "release/schemas/release-manifest-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-manifest-1.0.schema.json",
        )),
        ("release-manifest", "1.1") => Some((
            "release/schemas/release-manifest-1.1.schema.json",
            "https://vexil.dev/release/schemas/release-manifest-1.1.schema.json",
        )),
        ("release-evidence-set", "1.0") => Some((
            "release/schemas/release-evidence-set-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-evidence-set-1.0.schema.json",
        )),
        ("release-detached-approval", "1.0") => Some((
            "release/schemas/release-detached-approval-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-detached-approval-1.0.schema.json",
        )),
        ("release-approval-disposition", "1.0") => Some((
            "release/schemas/release-approval-disposition-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-approval-disposition-1.0.schema.json",
        )),
        ("privileged-run-start-authorization", "1.0") => Some((
            "release/schemas/privileged-run-start-authorization-1.0.schema.json",
            "https://vexil.dev/release/schemas/privileged-run-start-authorization-1.0.schema.json",
        )),
        ("release-adapter-result-envelope", "1.0") => Some((
            "release/schemas/release-adapter-result-envelope-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-adapter-result-envelope-1.0.schema.json",
        )),
        ("release-run-event", "1.0") => Some((
            "release/schemas/release-run-event-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-run-event-1.0.schema.json",
        )),
        ("release-run-evidence", "1.0") => Some((
            "release/schemas/release-run-evidence-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-run-evidence-1.0.schema.json",
        )),
        ("release-closeout", "1.0") => Some((
            "release/schemas/release-closeout-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-closeout-1.0.schema.json",
        )),
        _ => None,
    }
}

fn read_canonical_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("read canonical record {}: {error}", path.display()))?;
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) || bytes.contains(&b'\r') || !bytes.ends_with(b"\n") {
        return Err(format!(
            "canonical record has invalid raw-byte profile: {}",
            path.display()
        ));
    }
    let record: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse canonical record {}: {error}", path.display()))?;
    let expected = [
        serde_json::to_vec(&record)
            .map_err(|error| format!("encode canonical record {}: {error}", path.display()))?,
        vec![b'\n'],
    ]
    .concat();
    if bytes != expected {
        return Err(format!(
            "canonical record is parse-equivalent but not canonically encoded: {}",
            path.display()
        ));
    }
    Ok(record)
}

fn validate_canonical_release_record(
    root: &Path,
    record: &Value,
    path: &Path,
) -> Result<(), String> {
    validate_canonical_release_record_schema(root, record)?;
    let object = object(record, "canonical release record")?;
    let kind = text(object.get("recordKind"), "canonical release record kind")?;
    ensure_no_private_leakage(&record.to_string())?;
    validate_canonical_reference_paths(record)?;
    validate_canonical_record_times(kind, object)?;
    validate_canonical_digest_fields(record, path)?;
    if matches!(
        kind,
        "release-manifest" | "privileged-run-start-authorization"
    ) {
        validate_retained_state_artifacts(root, object)?;
    }
    Ok(())
}

fn validate_retained_state_artifacts(
    root: &Path,
    record: &Map<String, Value>,
) -> Result<(), String> {
    for field in ["stateSchema", "reducer"] {
        let artifact = object(required_value(record, field)?, "retained state artifact")?;
        let id = text(artifact.get("id"), "retained artifact ID")?;
        let version = text(artifact.get("version"), "retained artifact version")?;
        let digest = text(artifact.get("digest"), "retained artifact digest")?;
        let relative = Path::new(id);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(format!("retained {field} artifact path is unsafe: {id}"));
        }
        let file_name = relative
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .ok_or_else(|| format!("retained {field} artifact has no public filename"))?;
        if !file_name.contains(&format!("-{version}.")) {
            return Err(format!(
                "retained {field} artifact filename does not carry its exact version {version}"
            ));
        }
        let root_canonical = fs::canonicalize(root)
            .map_err(|error| format!("canonicalize repository root: {error}"))?;
        let artifact_path = fs::canonicalize(root.join(relative)).map_err(|error| {
            format!("retained {field} artifact is missing or unreadable {id}: {error}")
        })?;
        if !artifact_path.starts_with(&root_canonical) {
            return Err(format!(
                "retained {field} artifact escapes the repository root: {id}"
            ));
        }
        let actual = format!(
            "{:x}",
            Sha256::digest(
                fs::read(&artifact_path)
                    .map_err(|error| format!("read retained {field} artifact {id}: {error}"))?,
            )
        );
        if actual != digest {
            return Err(format!(
                "retained {field} artifact digest does not match its exact public bytes: {id}"
            ));
        }
    }
    Ok(())
}

/// Validates one retained canonical record against its exact kind/version schema.
/// Repository validation additionally enforces canonical bytes, locations, and
/// cross-family ownership references.
pub fn validate_canonical_release_record_schema(root: &Path, record: &Value) -> Result<(), String> {
    let object = object(record, "canonical release record")?;
    let kind = text(object.get("recordKind"), "canonical release record kind")?;
    let version = text(
        object.get("schemaVersion"),
        "canonical release record schema version",
    )?;
    let Some((schema, id)) = canonical_record_schema(kind, version) else {
        return Err(format!(
            "unknown or unsupported canonical record kind/version: {kind}@{version}"
        ));
    };
    if text(object.get("$schema"), "canonical release record schema ID")? != id {
        return Err(format!(
            "canonical record $schema does not match retained kind/version dispatch: {kind}@{version}"
        ));
    }
    validate_schema_instance(root, schema, record, kind)
}

fn validate_canonical_digest_fields(value: &Value, path: &Path) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            for (field, value) in fields {
                if (field.ends_with("Digest") || field == "digest")
                    && !(field == "priorEventDigest" && value.is_null())
                    && !value.as_str().is_some_and(|digest| {
                        digest.len() == 64
                            && digest
                                .bytes()
                                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                    })
                {
                    return Err(format!(
                        "canonical record has malformed SHA-256 digest field {field}: {}",
                        path.display()
                    ));
                }
                validate_canonical_digest_fields(value, path)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                validate_canonical_digest_fields(value, path)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_canonical_reference_paths(record: &Value) -> Result<(), String> {
    match record {
        Value::Object(fields) => {
            for (name, value) in fields {
                if name.ends_with("Path") || name == "path" {
                    let path = text(Some(value), "canonical record path")?;
                    if path.starts_with('/') || path.contains("..") || path.contains('\\') {
                        return Err(format!(
                            "canonical record contains unsafe public path: {path}"
                        ));
                    }
                }
                validate_canonical_reference_paths(value)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                validate_canonical_reference_paths(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_canonical_record_times(kind: &str, record: &Map<String, Value>) -> Result<(), String> {
    for field in [
        "approvedAt",
        "expiresAt",
        "effectiveAt",
        "issuedAt",
        "notBefore",
        "observedAt",
        "reviewedAt",
    ] {
        if let Some(value) = record.get(field) {
            let value = text(Some(value), "security-relevant timestamp")?;
            if !is_valid_utc_second(value) {
                return Err(format!(
                    "canonical {kind} timestamp {field} must be whole-second UTC"
                ));
            }
        }
    }
    if let (Some(approved), Some(expires)) = (record.get("approvedAt"), record.get("expiresAt")) {
        if text(Some(approved), "approval time")? >= text(Some(expires), "approval expiry")? {
            return Err("detached approval must have approvedAt before expiresAt".to_owned());
        }
    }
    if let (Some(issued), Some(not_before), Some(expires)) = (
        record.get("issuedAt"),
        record.get("notBefore"),
        record.get("expiresAt"),
    ) {
        if !(text(Some(issued), "authorization issue time")?
            <= text(Some(not_before), "authorization not-before time")?
            && text(Some(not_before), "authorization not-before time")?
                < text(Some(expires), "authorization expiry")?)
        {
            return Err(
                "start authorization must have issuedAt <= notBefore < expiresAt".to_owned(),
            );
        }
    }
    Ok(())
}

fn is_valid_utc_second(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    let number = |start: usize, end: usize| -> Option<u32> {
        std::str::from_utf8(&bytes[start..end]).ok()?.parse().ok()
    };
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0, 4),
        number(5, 7),
        number(8, 10),
        number(11, 13),
        number(14, 16),
        number(17, 19),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    day >= 1 && day <= max_day && hour < 24 && minute < 60 && second < 60
}

fn utc_second_timestamp(value: &str) -> Result<i64, String> {
    if !is_valid_utc_second(value) {
        return Err(format!("invalid whole-second UTC timestamp: {value}"));
    }
    let parse = |start: usize, end: usize| -> Result<i64, String> {
        value[start..end]
            .parse()
            .map_err(|_| format!("invalid whole-second UTC timestamp: {value}"))
    };
    let year = parse(0, 4)?;
    let month = parse(5, 7)?;
    let day = parse(8, 10)?;
    let hour = parse(11, 13)?;
    let minute = parse(14, 16)?;
    let second = parse(17, 19)?;
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let month_from_march = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Ok((era * 146_097 + day_of_era - 719_468) * 86_400 + hour * 3600 + minute * 60 + second)
}

fn validate_canonical_jsonl(
    root: &Path,
    path: &Path,
) -> Result<Vec<(std::path::PathBuf, Value)>, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("read canonical event stream {}: {error}", path.display()))?;
    if bytes.starts_with(&[0xef, 0xbb, 0xbf])
        || bytes.contains(&b'\r')
        || !bytes.ends_with(b"\n")
        || bytes.windows(2).any(|window| window == b"\n\n")
    {
        return Err(format!(
            "canonical JSONL stream has invalid raw-byte profile: {}",
            path.display()
        ));
    }
    let mut previous = None;
    let mut sequence = 0_u64;
    let mut dedupe = BTreeSet::new();
    let mut events = Vec::new();
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        let record: Value = serde_json::from_slice(line)
            .map_err(|error| format!("parse canonical JSONL event {}: {error}", path.display()))?;
        let expected = [
            serde_json::to_vec(&record).map_err(|error| {
                format!("encode canonical JSONL event {}: {error}", path.display())
            })?,
            vec![b'\n'],
        ]
        .concat();
        if line != expected {
            return Err(format!(
                "canonical JSONL event is not canonically encoded: {}",
                path.display()
            ));
        }
        validate_canonical_release_record(root, &record, path)?;
        let event = object(&record, "release Run event")?;
        sequence += 1;
        if event.get("sequenceNumber").and_then(Value::as_u64) != Some(sequence) {
            return Err(
                "release Run events must have contiguous sequence numbers beginning at one"
                    .to_owned(),
            );
        }
        let prior = event.get("priorEventDigest");
        let expected_prior = previous
            .as_ref()
            .map(|previous: &Vec<u8>| format!("{:x}", Sha256::digest(previous)));
        if prior.and_then(Value::as_str) != expected_prior.as_deref() {
            return Err("release Run event prior-event digest chain is invalid".to_owned());
        }
        let key = (
            text(event.get("runId"), "release Run event run ID")?.to_owned(),
            text(event.get("operationId"), "release Run event operation ID")?.to_owned(),
            event
                .get("attempt")
                .and_then(Value::as_u64)
                .ok_or_else(|| "release Run event attempt must be an integer".to_owned())?,
        );
        if !dedupe.insert(key) {
            return Err(
                "release Run event operation/attempt deduplication key is not unique".to_owned(),
            );
        }
        validate_canonical_event_location(root, path, event)?;
        events.push((path.to_owned(), record));
        previous = Some(line.to_vec());
    }
    Ok(events)
}

fn validate_canonical_event_location(
    root: &Path,
    path: &Path,
    event: &Map<String, Value>,
) -> Result<(), String> {
    let root =
        fs::canonicalize(root).map_err(|error| format!("canonicalize repository root: {error}"))?;
    let relative = path.strip_prefix(&root).map_err(|_| {
        format!(
            "canonical JSONL event stream escapes repository root: {}",
            path.display()
        )
    })?;
    let parts = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let run_id = text(event.get("runId"), "release Run event run ID")?;
    if parts.len() != 4
        || parts[0] != "release"
        || parts[1] != "runs"
        || parts[2] != run_id
        || parts[3] != "events.jsonl"
    {
        return Err(format!(
            "release Run event is not materialized in its Run events.jsonl stream: {}",
            relative.display()
        ));
    }
    Ok(())
}

fn validate_canonical_record_references(
    root: &Path,
    index: &CanonicalRecordIndex,
) -> Result<(), String> {
    for ((kind, id), record) in &index.records {
        let object = object(&record.value, "canonical release record")?;
        match kind.as_str() {
            "release-evidence-set" => validate_evidence_set_entries(root, object)?,
            "release-manifest" => validate_record_reference(
                index,
                object,
                "evidenceSetId",
                "evidenceSetDigest",
                "release-evidence-set",
                id,
            )?,
            "release-detached-approval" => {
                validate_manifest_and_evidence_references(index, object, id)?;
                validate_detached_approval_bindings(root, object, id)?;
                validate_manifest_directory_reference(&record.path, object, kind)?;
            }
            "release-approval-disposition" => {
                validate_record_reference(
                    index,
                    object,
                    "approvalId",
                    "approvalDigest",
                    "release-detached-approval",
                    id,
                )?;
                validate_approval_disposition_directory(index, &record.path, object)?;
            }
            "privileged-run-start-authorization" => {
                validate_manifest_and_evidence_references(index, object, id)?;
                validate_start_authorization_bindings(root, index, object, &record.path, id)?;
                validate_run_directory_reference(&record.path, object, kind)?;
            }
            "release-adapter-result-envelope" | "release-run-evidence" | "release-closeout" => {
                validate_record_reference(
                    index,
                    object,
                    "manifestId",
                    "manifestDigest",
                    "release-manifest",
                    id,
                )?;
                validate_run_directory_reference(&record.path, object, kind)?;
            }
            _ => {}
        }
    }
    validate_approval_dispositions(root, index)?;
    validate_run_event_bindings(index)?;
    validate_run_scoped_record_bindings(root, index)?;
    validate_closeout_evidence_bindings(index)?;
    Ok(())
}

fn start_authorization_for_run<'a>(
    index: &'a CanonicalRecordIndex,
    run_id: &str,
) -> Result<&'a Map<String, Value>, String> {
    let mut matches = index.records.iter().filter_map(|((kind, _), record)| {
        (kind == "privileged-run-start-authorization"
            && record.value.get("runId").and_then(Value::as_str) == Some(run_id))
        .then_some(&record.value)
    });
    let authorization = matches
        .next()
        .ok_or_else(|| format!("Run {run_id} has no start authorization"))?;
    if matches.next().is_some() {
        return Err(format!("Run {run_id} has ambiguous start authorizations"));
    }
    object(authorization, "Run start authorization")
}

fn validate_run_authorization_binding(
    index: &CanonicalRecordIndex,
    record: &Map<String, Value>,
    context: &str,
) -> Result<(), String> {
    let run_id = text(record.get("runId"), "canonical Run reference")?;
    let authorization = start_authorization_for_run(index, run_id)?;
    let authorization_id = text(
        authorization.get("authorizationId"),
        "Run start authorization ID",
    )?;
    let authorization_record = index
        .records
        .get(&(
            String::from("privileged-run-start-authorization"),
            authorization_id.to_owned(),
        ))
        .ok_or_else(|| {
            format!("{context} references missing Run start authorization {authorization_id}")
        })?;
    if record.get("authorizationId").and_then(Value::as_str) != Some(authorization_id)
        || record.get("authorizationDigest").and_then(Value::as_str)
            != Some(authorization_record.digest.as_str())
    {
        return Err(format!(
            "{context} does not bind its Run start authorization's exact immutable identity and digest"
        ));
    }
    for field in [
        "manifestId",
        "manifestDigest",
        "evidenceSetId",
        "evidenceSetDigest",
    ] {
        if record.get(field) != authorization.get(field) {
            return Err(format!(
                "{context} does not bind the Run start authorization's frozen {field}"
            ));
        }
    }
    if let Some(operation_id) = record.get("operationId").and_then(Value::as_str) {
        let allowed_operations = array(
            authorization.get("allowedOperations"),
            "start authorization allowed operations",
        )?;
        if !allowed_operations
            .iter()
            .any(|allowed| allowed.as_str() == Some(operation_id))
        {
            return Err(format!(
                "{context} operation {operation_id} is not allowed by its Run start authorization"
            ));
        }
    }
    if let Some(actor) = record.get("actor").and_then(Value::as_str) {
        let execution_principal = object(
            authorization
                .get("executionPrincipal")
                .ok_or("Run start authorization has no execution principal")?,
            "Run start authorization execution principal",
        )?;
        if execution_principal.get("actor").and_then(Value::as_str) != Some(actor) {
            return Err(format!(
                "{context} actor does not match its Run start authorization execution principal"
            ));
        }
        if let Some(event_principal) = record.get("executionPrincipal") {
            if Some(event_principal) != authorization.get("executionPrincipal") {
                return Err(format!(
                    "{context} execution principal does not match its Run start authorization"
                ));
            }
        }
    }
    for field in ["stateSchema", "reducer"] {
        if let Some(value) = record.get(field) {
            if Some(value) != authorization.get(field) {
                return Err(format!(
                    "{context} does not bind the Run start authorization's frozen {field}"
                ));
            }
        }
    }
    Ok(())
}

fn validate_run_event_bindings(index: &CanonicalRecordIndex) -> Result<(), String> {
    for (_, event) in &index.events {
        validate_run_authorization_binding(
            index,
            object(event, "release Run event")?,
            "release Run event",
        )?;
    }
    Ok(())
}

fn validate_run_scoped_record_bindings(
    root: &Path,
    index: &CanonicalRecordIndex,
) -> Result<(), String> {
    for ((kind, id), record) in &index.records {
        if matches!(
            kind.as_str(),
            "release-adapter-result-envelope" | "release-run-evidence" | "release-closeout"
        ) {
            validate_run_authorization_binding(
                index,
                object(&record.value, "canonical Run-scoped record")?,
                &format!("{kind} {id}"),
            )?;
            if kind == "release-closeout" {
                let closeout = object(&record.value, "release closeout")?;
                let steward = object(
                    required_value(closeout, "steward")?,
                    "release closeout steward",
                )?;
                ensure_active_assignment(
                    root,
                    text(steward.get("actor"), "release closeout steward actor")?,
                    text(steward.get("role"), "release closeout steward role")?,
                    text(
                        steward.get("assignment"),
                        "release closeout steward assignment",
                    )?,
                    "release-manifest-lifecycle",
                    text(closeout.get("closedAt"), "release closeout time")?,
                    "release closeout steward",
                )?;
            }
        }
    }
    Ok(())
}

fn validate_closeout_evidence_bindings(index: &CanonicalRecordIndex) -> Result<(), String> {
    for ((kind, id), record) in &index.records {
        if kind != "release-closeout" {
            continue;
        }
        let closeout = object(&record.value, "release closeout")?;
        let run_id = text(closeout.get("runId"), "release closeout Run ID")?;
        let mut expected = Vec::new();
        for ((evidence_kind, evidence_id), evidence) in &index.records {
            if !matches!(
                evidence_kind.as_str(),
                "release-adapter-result-envelope" | "release-run-evidence"
            ) {
                continue;
            }
            let evidence_record = object(&evidence.value, "Run evidence record")?;
            if evidence_record.get("runId").and_then(Value::as_str) == Some(run_id) {
                expected.push((
                    evidence_kind.clone(),
                    evidence_id.clone(),
                    evidence.digest.clone(),
                ));
            }
        }
        let actual = array(closeout.get("evidence"), "release closeout evidence")?
            .iter()
            .map(|entry| {
                let entry = object(entry, "release closeout evidence entry")?;
                Ok((
                    text(entry.get("kind"), "release closeout evidence kind")?.to_owned(),
                    text(entry.get("id"), "release closeout evidence ID")?.to_owned(),
                    text(entry.get("digest"), "release closeout evidence digest")?.to_owned(),
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        if actual != expected {
            return Err(format!(
                "release closeout {id} evidence must be the exact ordered digest-checked inventory for Run {run_id}"
            ));
        }
    }
    Ok(())
}

fn validate_approval_disposition_directory(
    index: &CanonicalRecordIndex,
    path: &Path,
    record: &Map<String, Value>,
) -> Result<(), String> {
    let approval_id = text(record.get("approvalId"), "approval disposition approval ID")?;
    let approval = index
        .records
        .get(&(
            String::from("release-detached-approval"),
            approval_id.to_owned(),
        ))
        .ok_or_else(|| format!("approval disposition references missing approval {approval_id}"))?;
    let manifest_directory = |path: &Path| {
        path.ancestors()
            .nth(2)
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
    };
    if manifest_directory(path) != manifest_directory(&approval.path) {
        return Err(
            "approval disposition is materialized under a different Manifest than its approval"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_approval_dispositions(root: &Path, index: &CanonicalRecordIndex) -> Result<(), String> {
    let mut dispositions = BTreeSet::new();
    for ((kind, _), record) in &index.records {
        if kind != "release-approval-disposition" {
            continue;
        }
        let disposition = object(&record.value, "approval disposition")?;
        let approval_id = text(
            disposition.get("approvalId"),
            "approval disposition approval ID",
        )?;
        if !dispositions.insert(approval_id.to_owned()) {
            return Err(format!(
                "approval {approval_id} has more than one immutable disposition"
            ));
        }
        let approval = index
            .records
            .get(&(
                String::from("release-detached-approval"),
                approval_id.to_owned(),
            ))
            .ok_or_else(|| {
                format!("approval disposition references missing approval {approval_id}")
            })?;
        let approval = object(&approval.value, "detached approval")?;
        if utc_second_timestamp(text(
            disposition.get("effectiveAt"),
            "disposition effective time",
        )?)? < utc_second_timestamp(text(approval.get("approvedAt"), "approval time")?)?
        {
            return Err(format!(
                "approval disposition predates approval {approval_id}"
            ));
        }
        let authority = object(
            disposition
                .get("authority")
                .ok_or("approval disposition has no authority")?,
            "approval disposition authority",
        )?;
        ensure_repository_administrator(
            root,
            text(authority.get("actor"), "approval disposition actor")?,
            text(authority.get("role"), "approval disposition role")?,
            text(
                authority.get("assignment"),
                "approval disposition assignment",
            )?,
            text(
                disposition.get("effectiveAt"),
                "approval disposition effective time",
            )?,
        )?;
    }
    Ok(())
}

fn validate_evidence_set_entries(root: &Path, record: &Map<String, Value>) -> Result<(), String> {
    let steward = object(
        record
            .get("steward")
            .ok_or("reviewed evidence set has no Release Steward")?,
        "reviewed evidence-set steward",
    )?;
    ensure_release_steward_eligible(
        root,
        text(steward.get("actor"), "reviewed evidence-set steward actor")?,
        text(steward.get("role"), "reviewed evidence-set steward role")?,
        text(
            steward.get("assignment"),
            "reviewed evidence-set steward assignment",
        )?,
        text(record.get("reviewedAt"), "reviewed evidence-set time")?,
        None,
        false,
    )?;
    let entries = array(record.get("entries"), "reviewed evidence-set entries")?;
    let mut previous = None;
    for entry in entries {
        let entry = object(entry, "reviewed evidence-set entry")?;
        let key = (
            text(entry.get("kind"), "reviewed evidence-set entry kind")?.to_owned(),
            text(entry.get("id"), "reviewed evidence-set entry ID")?.to_owned(),
            text(entry.get("path"), "reviewed evidence-set entry path")?.to_owned(),
            text(
                entry.get("contentDigest"),
                "reviewed evidence-set entry digest",
            )?
            .to_owned(),
        );
        if key.0 == "release-manifest" || key.2.starts_with("release/manifests/") {
            return Err("reviewed evidence-set must not reference a Release Manifest".to_owned());
        }
        let evidence_path = root.join(&key.2);
        let root_canonical = fs::canonicalize(root)
            .map_err(|error| format!("canonicalize repository root: {error}"))?;
        let evidence_canonical = fs::canonicalize(&evidence_path).map_err(|error| {
            format!(
                "reviewed evidence-set entry path is missing or unreadable {}: {error}",
                key.2
            )
        })?;
        if !evidence_canonical.starts_with(&root_canonical) {
            return Err(format!(
                "reviewed evidence-set entry path escapes repository root: {}",
                key.2
            ));
        }
        let actual_digest = format!(
            "{:x}",
            Sha256::digest(fs::read(&evidence_canonical).map_err(|error| {
                format!("read reviewed evidence-set entry {}: {error}", key.2)
            })?)
        );
        if actual_digest != key.3 {
            return Err(format!(
                "reviewed evidence-set entry digest does not match public path: {}",
                key.2
            ));
        }
        if previous.as_ref().is_some_and(|previous| previous >= &key) {
            return Err(
                "reviewed evidence-set entries must be strictly deterministically ordered"
                    .to_owned(),
            );
        }
        previous = Some(key);
    }
    Ok(())
}

fn validate_manifest_and_evidence_references(
    index: &CanonicalRecordIndex,
    record: &Map<String, Value>,
    context: &str,
) -> Result<(), String> {
    validate_record_reference(
        index,
        record,
        "manifestId",
        "manifestDigest",
        "release-manifest",
        context,
    )?;
    validate_record_reference(
        index,
        record,
        "evidenceSetId",
        "evidenceSetDigest",
        "release-evidence-set",
        context,
    )?;
    let manifest_id = text(record.get("manifestId"), "canonical Manifest reference")?;
    let manifest = index
        .records
        .get(&(String::from("release-manifest"), manifest_id.to_owned()))
        .ok_or_else(|| format!("canonical record references missing Manifest {manifest_id}"))?;
    let manifest = object(&manifest.value, "Release Manifest")?;
    for field in ["evidenceSetId", "evidenceSetDigest"] {
        if record.get(field) != manifest.get(field) {
            return Err(format!(
                "{context} does not bind the exact evidence set frozen in Manifest {manifest_id}"
            ));
        }
    }
    Ok(())
}

fn validate_detached_approval_bindings(
    _root: &Path,
    record: &Map<String, Value>,
    _context: &str,
) -> Result<(), String> {
    text(
        record.get("governanceDigest"),
        "detached approval governance digest",
    )?;
    Ok(())
}

fn validate_start_authorization_bindings(
    root: &Path,
    index: &CanonicalRecordIndex,
    record: &Map<String, Value>,
    path: &Path,
    context: &str,
) -> Result<(), String> {
    let manifest_id = text(record.get("manifestId"), "start authorization Manifest ID")?;
    let manifest = index
        .records
        .get(&(String::from("release-manifest"), manifest_id.to_owned()))
        .ok_or_else(|| format!("{context} references missing Manifest {manifest_id}"))?;
    let manifest = object(&manifest.value, "Release Manifest")?;
    for field in ["stateSchema", "reducer"] {
        if record.get(field) != manifest.get(field) {
            return Err(format!(
                "{context} does not freeze the exact {field} retained by Manifest {manifest_id}"
            ));
        }
    }
    let governance = object(
        required_value(record, "governanceRevision")?,
        "start authorization governance revision",
    )?;
    if text(
        governance.get("id"),
        "start authorization governance revision ID",
    )? != "governance-revision-v1"
        || text(
            governance.get("digest"),
            "start authorization governance revision digest",
        )? != governance_revision_v1(root)?
    {
        return Err(format!(
            "{context} does not bind the exact governance-revision-v1 identity and digest"
        ));
    }
    let run_id = text(record.get("runId"), "start authorization Run ID")?;
    let expected_path = format!("release/runs/{run_id}/start-authorization.json");
    if text(
        record.get("materializationPath"),
        "start authorization materialization path",
    )? != expected_path
    {
        return Err(format!(
            "{context} materialization path does not bind its exact Run location"
        ));
    }
    let mut approvals = BTreeSet::new();
    for selected in array(
        record.get("selectedApprovals"),
        "start authorization selected approvals",
    )? {
        let selected = object(selected, "start authorization selected approval")?;
        let approval_id = text(selected.get("approvalId"), "selected approval ID")?;
        if !approvals.insert(approval_id.to_owned()) {
            return Err(format!(
                "{context} selects approval {approval_id} more than once"
            ));
        }
        validate_record_reference(
            index,
            selected,
            "approvalId",
            "approvalDigest",
            "release-detached-approval",
            context,
        )?;
        let approval = index
            .records
            .get(&(
                String::from("release-detached-approval"),
                approval_id.to_owned(),
            ))
            .ok_or_else(|| format!("{context} references missing approval {approval_id}"))?;
        let approval = object(&approval.value, "selected detached approval")?;
        for field in [
            "manifestId",
            "manifestDigest",
            "evidenceSetId",
            "evidenceSetDigest",
        ] {
            if approval.get(field) != record.get(field) {
                return Err(format!(
                    "{context} selected approval {approval_id} does not bind its frozen {field}"
                ));
            }
        }
        if let Some(disposition) = selected.get("disposition").filter(|value| !value.is_null()) {
            let disposition = object(disposition, "selected approval disposition")?;
            let disposition_id = text(disposition.get("id"), "selected disposition ID")?;
            let disposition_digest =
                text(disposition.get("digest"), "selected disposition digest")?;
            let target = index
                .records
                .get(&(
                    String::from("release-approval-disposition"),
                    disposition_id.to_owned(),
                ))
                .ok_or_else(|| {
                    format!("{context} references missing approval disposition {disposition_id}")
                })?;
            if target.digest != disposition_digest {
                return Err(format!(
                    "{context} references approval disposition {disposition_id} with a mismatched immutable digest"
                ));
            }
            let target = object(&target.value, "approval disposition")?;
            if target.get("approvalId") != selected.get("approvalId")
                || target.get("approvalDigest") != selected.get("approvalDigest")
            {
                return Err(format!(
                    "{context} disposition {disposition_id} does not bind its selected approval"
                ));
            }
        }
    }
    let actual_path = path
        .strip_prefix(fs::canonicalize(root).map_err(|error| {
            format!("canonicalize repository root for start authorization: {error}")
        })?)
        .map_err(|_| format!("{context} start authorization path escapes repository root"))?
        .to_string_lossy()
        .replace('\\', "/");
    if actual_path != expected_path {
        return Err(format!(
            "{context} start authorization is not materialized at its declared path"
        ));
    }
    let issuer = object(
        required_value(record, "issuer")?,
        "start authorization Release Steward issuer",
    )?;
    ensure_active_assignment(
        root,
        text(issuer.get("actor"), "start authorization issuer actor")?,
        text(issuer.get("role"), "start authorization issuer role")?,
        text(
            issuer.get("assignment"),
            "start authorization issuer assignment",
        )?,
        "release-manifest-lifecycle",
        text(record.get("issuedAt"), "start authorization issuance time")?,
        "start authorization issuer",
    )?;
    let execution_principal = object(
        required_value(record, "executionPrincipal")?,
        "start authorization Release Run Coordinator execution principal",
    )?;
    ensure_active_assignment(
        root,
        text(
            execution_principal.get("actor"),
            "start authorization execution actor",
        )?,
        text(
            execution_principal.get("role"),
            "start authorization execution role",
        )?,
        text(
            execution_principal.get("assignment"),
            "start authorization execution assignment",
        )?,
        "release-run-execution",
        text(
            record.get("notBefore"),
            "start authorization not-before time",
        )?,
        "start authorization execution principal",
    )?;
    let allowed_targets = array(record.get("allowedTargets"), "start authorization targets")?
        .iter()
        .map(|target| text(Some(target), "start authorization target").map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let target_control_evidence = array(
        record.get("targetControlEvidence"),
        "start authorization target-control evidence",
    )?;
    let evidence_targets = target_control_evidence
        .iter()
        .map(|evidence| {
            let evidence = object(evidence, "start authorization target-control evidence")?;
            text(evidence.get("target"), "target-control evidence target").map(str::to_owned)
        })
        .collect::<Result<BTreeSet<_>, String>>()?;
    if allowed_targets != evidence_targets
        || evidence_targets.len() != target_control_evidence.len()
    {
        return Err(format!(
            "{context} allowed targets must exactly match target-control and permission evidence"
        ));
    }
    Ok(())
}

fn validate_record_reference(
    index: &CanonicalRecordIndex,
    record: &Map<String, Value>,
    id_field: &str,
    digest_field: &str,
    expected_kind: &str,
    context: &str,
) -> Result<(), String> {
    let id = text(record.get(id_field), "canonical record reference ID")?;
    let digest = text(
        record.get(digest_field),
        "canonical record reference digest",
    )?;
    let Some(target) = index
        .records
        .get(&(expected_kind.to_owned(), id.to_owned()))
    else {
        return Err(format!(
            "{context} references missing {expected_kind} identity {id}"
        ));
    };
    if target.digest != digest {
        return Err(format!(
            "{context} references {expected_kind} {id} with a mismatched immutable digest"
        ));
    }
    Ok(())
}

fn validate_manifest_directory_reference(
    path: &Path,
    record: &Map<String, Value>,
    kind: &str,
) -> Result<(), String> {
    let manifest_id = text(record.get("manifestId"), "canonical Manifest reference")?;
    let actual = path
        .ancestors()
        .nth(2)
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    if actual.as_deref() != Some(manifest_id) {
        return Err(format!(
            "{kind} is materialized under a different Manifest identity"
        ));
    }
    Ok(())
}

fn validate_run_directory_reference(
    path: &Path,
    record: &Map<String, Value>,
    kind: &str,
) -> Result<(), String> {
    let run_id = text(record.get("runId"), "canonical Run reference")?;
    let parts = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let actual = parts
        .iter()
        .position(|part| part == "runs")
        .and_then(|index| parts.get(index + 1));
    if actual.map(String::as_str) != Some(run_id) {
        return Err(format!(
            "{kind} is materialized under a different Run identity"
        ));
    }
    Ok(())
}

/// Validates the source-led release-unit catalog without consulting providers or
/// inferring publication from source presence.
pub fn validate_catalog_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let catalog = read_json(&root.join("release/catalog.json"))?;
    validate_catalog_schema(root, &catalog)?;
    validate_catalog(root, &catalog)?;
    let documentation = fs::read_to_string(root.join("docs/book/src/release/catalog.md"))
        .map_err(|error| format!("read release catalog documentation: {error}"))?;
    validate_catalog_documentation_parity(root, &catalog, &documentation)?;
    validate_npm_publish_workflow(root)
}

/// Validates the public lifecycle ledger against the active source-led catalog.
/// It is structural governance only: it neither selects a Release Set nor creates
/// a Manifest, tag, provider query, or publication effect.
pub fn validate_catalog_lifecycle_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let catalog = read_json(&root.join("release/catalog.json"))?;
    let lifecycle = read_json(&root.join("release/catalog-lifecycle.json"))?;
    validate_catalog_schema(root, &catalog)?;
    validate_catalog_lifecycle_schema(root, &lifecycle)?;
    validate_catalog_lifecycle(root, &catalog, &lifecycle)
}

pub fn validate_catalog_lifecycle(
    root: &Path,
    catalog: &Value,
    lifecycle: &Value,
) -> Result<(), String> {
    validate_catalog_lifecycle_schema(root, lifecycle)?;
    ensure_no_private_leakage(&serde_json::to_string(lifecycle).map_err(|error| {
        format!("serialize catalog lifecycle for boundary validation: {error}")
    })?)?;

    let catalog_units = array(catalog.get("units"), "release catalog units")?;
    let ledger = object(lifecycle, "catalog lifecycle ledger")?;
    let catalog_revision = text(ledger.get("catalogRevision"), "catalog lifecycle revision")?;
    let records = array(ledger.get("records"), "catalog lifecycle records")?;
    let expected_roots = expected_catalog_source_roots(root)?;
    let catalog_roots: BTreeSet<_> = catalog_units
        .iter()
        .filter_map(|unit| unit.get("sourceRoot").and_then(Value::as_str))
        .map(str::to_owned)
        .collect();
    if !expected_roots.is_subset(&catalog_roots) {
        return Err(
            "catalog lifecycle requires add/propose review for a new source unit".to_owned(),
        );
    }
    if !catalog_roots.is_subset(&expected_roots) {
        return Err(
            "catalog lifecycle requires retire/exclude review for a removed source unit".to_owned(),
        );
    }
    let mut record_ids = BTreeSet::new();
    let mut stable_ids = BTreeSet::new();
    let mut source_roots = BTreeSet::new();
    let mut namespaces = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let mut active = BTreeMap::new();
    let mut all_records = BTreeMap::new();
    let mut previous_unit_id = "";

    for value in records {
        let record = object(value, "catalog lifecycle record")?;
        let record_id = text(
            record.get("lifecycleRecordId"),
            "catalog lifecycle record ID",
        )?;
        let unit_id = text(record.get("unitId"), "catalog lifecycle unit ID")?;
        if !record_ids.insert(record_id.to_owned()) || !stable_ids.insert(unit_id.to_owned()) {
            return Err("catalog lifecycle record and stable unit IDs must be unique".to_owned());
        }
        if unit_id <= previous_unit_id {
            return Err("catalog lifecycle records must be sorted by stable unit ID".to_owned());
        }
        previous_unit_id = unit_id;
        let source_root = text(record.get("sourceRoot"), "catalog lifecycle source root")?;
        if !source_roots.insert(source_root.to_owned()) {
            return Err("catalog lifecycle source roots must never be reused".to_owned());
        }
        let namespace = text(
            record.get("canonicalTagNamespace"),
            "catalog lifecycle tag namespace",
        )?;
        if namespace != "not-applicable" && !namespaces.insert(namespace.to_owned()) {
            return Err(
                "catalog lifecycle canonical tag namespaces must never be reused".to_owned(),
            );
        }
        for target in array(
            record.get("targetIdentities"),
            "catalog lifecycle target identities",
        )? {
            let target = object(target, "catalog lifecycle target identity")?;
            let identity = format!(
                "{}:{}",
                text(target.get("kind"), "catalog lifecycle target kind")?,
                text(target.get("name"), "catalog lifecycle target name")?
            );
            if !targets.insert(identity) {
                return Err("catalog lifecycle target identities must never be reused".to_owned());
            }
        }
        validate_lifecycle_review(root, record)?;
        let state = text(record.get("state"), "catalog lifecycle state")?;
        let decision = object(
            required_value(record, "lifecycleDecision")?,
            "catalog lifecycle decision",
        )?;
        let effective_revision = text(
            record.get("effectiveRevision"),
            "catalog lifecycle effective revision",
        )?;
        if text(
            decision.get("effectiveRevision"),
            "catalog lifecycle decision effective revision",
        )? != effective_revision
        {
            return Err(
                "catalog lifecycle decision revision must equal its record effective revision"
                    .to_owned(),
            );
        }
        let expected_decision = match state {
            "active"
                if record
                    .get("predecessorUnitId")
                    .and_then(Value::as_str)
                    .is_some() =>
            {
                "approved-rename"
            }
            "active" => "accepted-active-baseline",
            "renamed" => "approved-rename",
            "retired" => "approved-retirement",
            "excluded" => "approved-exclusion",
            _ => return Err("catalog lifecycle state is not recognized".to_owned()),
        };
        if text(decision.get("state"), "catalog lifecycle decision state")? != expected_decision {
            return Err(
                "catalog lifecycle decision state does not match its lifecycle state".to_owned(),
            );
        }
        if state == "active" && effective_revision != catalog_revision {
            return Err(
                "active catalog lifecycle revision must equal the catalog revision".to_owned(),
            );
        }
        let successor = record.get("successorUnitId").and_then(Value::as_str);
        match state {
            "active" if successor.is_some() => {
                return Err("active catalog lifecycle entries cannot name a rename successor".to_owned())
            }
            "renamed" if successor.is_none() => {
                return Err("renamed catalog lifecycle entries require an explicit successor".to_owned())
            }
            "retired" | "excluded" if successor.is_some() => {
                return Err("retired or excluded catalog lifecycle entries cannot reuse an identity through a successor".to_owned())
            }
            "active" | "renamed" | "retired" | "excluded" => {}
            _ => return Err("catalog lifecycle state is not recognized".to_owned()),
        }
        if state == "active" {
            active.insert(unit_id, record);
        }
        all_records.insert(unit_id, record);
    }

    for (unit_id, record) in &all_records {
        let state = text(record.get("state"), "catalog lifecycle state")?;
        let publication = object(
            required_value(record, "publication")?,
            "catalog lifecycle publication",
        )?;
        let needs_rationale = state != "active"
            || record
                .get("predecessorUnitId")
                .and_then(Value::as_str)
                .is_some();
        if needs_rationale
            && text(
                publication.get("classification"),
                "catalog lifecycle publication classification",
            )? != "non-publishable"
        {
            let impact = object(
                required_value(record, "compatibilityImpact")?,
                "catalog lifecycle compatibility impact",
            )?;
            if text(
                impact.get("state"),
                "catalog lifecycle compatibility impact state",
            )? != "requires-rationale"
                || impact
                    .get("rationaleReference")
                    .and_then(Value::as_str)
                    .is_none()
                || text(
                    impact.get("decisionState"),
                    "catalog lifecycle rationale decision state",
                )? != "accepted"
            {
                return Err(format!("catalog lifecycle transition for {unit_id} requires an accepted compatibility rationale"));
            }
            let rationale_id = text(
                impact.get("rationaleReference"),
                "catalog lifecycle rationale reference",
            )?;
            let rationale = read_json(
                &root
                    .join("release/rationales")
                    .join(format!("{rationale_id}.json")),
            )
            .map_err(|error| {
                format!(
                    "catalog lifecycle rationale reference {rationale_id} is unavailable: {error}"
                )
            })?;
            validate_version_rationale(root, catalog, &rationale)?;
            let expected_rationale_unit = record
                .get("successorUnitId")
                .and_then(Value::as_str)
                .unwrap_or(unit_id);
            if text(
                rationale.get("unitId"),
                "catalog lifecycle rationale unit ID",
            )? != expected_rationale_unit
            {
                return Err(format!(
                    "catalog lifecycle rationale for {unit_id} must bind its transitioning unit"
                ));
            }
        }
        if state == "renamed" {
            let successor = text(
                record.get("successorUnitId"),
                "catalog lifecycle rename successor",
            )?;
            let successor_record = all_records.get(successor).ok_or_else(|| {
                format!("catalog lifecycle rename successor is unknown: {successor}")
            })?;
            if successor_record.get("state").and_then(Value::as_str) != Some("active")
                || successor_record
                    .get("predecessorUnitId")
                    .and_then(Value::as_str)
                    != Some(*unit_id)
            {
                return Err(
                    "catalog lifecycle rename must form an explicit predecessor/successor chain"
                        .to_owned(),
                );
            }
        }
        if let Some(predecessor) = record.get("predecessorUnitId").and_then(Value::as_str) {
            let predecessor_record = all_records.get(predecessor).ok_or_else(|| {
                format!("catalog lifecycle rename predecessor is unknown: {predecessor}")
            })?;
            if predecessor_record.get("state").and_then(Value::as_str) != Some("renamed")
                || predecessor_record
                    .get("successorUnitId")
                    .and_then(Value::as_str)
                    != Some(*unit_id)
            {
                return Err(
                    "catalog lifecycle active rename successor must link back to its predecessor"
                        .to_owned(),
                );
            }
        }
    }

    for unit in catalog_units {
        let unit = object(unit, "release catalog unit")?;
        let id = text(unit.get("id"), "release catalog unit ID")?;
        let Some(record) = active.get(id) else {
            return Err(format!(
                "catalog lifecycle requires add/propose review for current unit: {id}"
            ));
        };
        for (record_key, catalog_key) in [
            ("sourceRoot", "sourceRoot"),
            ("canonicalTagNamespace", "canonicalTagNamespace"),
            ("owner", "owner"),
            ("publication", "publication"),
        ] {
            if record.get(record_key) != unit.get(catalog_key) {
                let action = if record_key == "publication" {
                    "publishability transition with review"
                } else {
                    "rename with predecessor"
                };
                return Err(format!(
                    "catalog lifecycle mismatch for {id} requires {action}"
                ));
            }
        }
        if record.get("targetIdentities") != unit.get("targets") {
            return Err(format!(
                "catalog lifecycle mismatch for {id} requires rename with predecessor"
            ));
        }
    }
    for (id, _) in active {
        if !catalog_units
            .iter()
            .any(|unit| unit.get("id").and_then(Value::as_str) == Some(id))
        {
            return Err(format!(
                "catalog lifecycle requires retire/exclude review for removed unit: {id}"
            ));
        }
    }
    Ok(())
}

fn validate_lifecycle_review(root: &Path, record: &Map<String, Value>) -> Result<(), String> {
    let owner = object(required_value(record, "owner")?, "catalog lifecycle owner")?;
    let proposal = object(
        required_value(record, "stewardProposal")?,
        "catalog lifecycle proposal",
    )?;
    require_utc_timestamp(
        proposal.get("proposedAt"),
        "catalog lifecycle proposal timestamp",
    )?;
    if proposal.get("roleId") != owner.get("roleId")
        || proposal.get("assignmentId") != owner.get("assignmentId")
    {
        return Err(
            "catalog lifecycle proposal must be attributed to the accountable owner".to_owned(),
        );
    }
    let acceptance = object(
        required_value(record, "releaseStewardAcceptance")?,
        "catalog lifecycle acceptance",
    )?;
    require_utc_timestamp(
        acceptance.get("acceptedAt"),
        "catalog lifecycle acceptance timestamp",
    )?;
    if text(
        acceptance.get("roleId"),
        "catalog lifecycle acceptance role",
    )? != "release-steward"
    {
        return Err("catalog lifecycle acceptance must assert the Release Steward role".to_owned());
    }
    let assertions = array(
        acceptance.get("roleAssertions"),
        "catalog lifecycle role assertions",
    )?;
    if assertions.len() != 2
        || assertions[0].as_str() != owner.get("roleId").and_then(Value::as_str)
        || assertions[1].as_str() != Some("release-steward")
    {
        return Err("catalog lifecycle acceptance must retain distinct owner and Release Steward role assertions".to_owned());
    }
    let assignments = read_json(&root.join("release/stewardship/assignments.json"))?;
    let assignments = array(assignments.get("assignments"), "stewardship assignments")?;
    for (review, label, expected_role) in [
        (
            proposal,
            "proposal",
            text(owner.get("roleId"), "catalog lifecycle owner role")?,
        ),
        (acceptance, "acceptance", "release-steward"),
    ] {
        let assignment_id = text(
            review.get("assignmentId"),
            "catalog lifecycle assignment ID",
        )?;
        let assignment = assignments
            .iter()
            .find(|value| value.get("assignmentId").and_then(Value::as_str) == Some(assignment_id))
            .ok_or_else(|| format!("catalog lifecycle {label} references an unknown assignment"))?;
        if assignment.get("roleId").and_then(Value::as_str) != Some(expected_role)
            || assignment.get("status").and_then(Value::as_str) != Some("active")
            || assignment.get("primaryActorId") != review.get("actorId")
        {
            return Err(format!(
                "catalog lifecycle {label} actor is not the active asserted steward"
            ));
        }
    }
    Ok(())
}

/// Validates public, per-unit version rationale records without selecting a
/// Release Set, resolving evidence, or authorizing any release operation.
pub fn validate_version_rationale_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let catalog = read_json(&root.join("release/catalog.json"))?;
    validate_catalog_schema(root, &catalog)?;
    validate_catalog(root, &catalog)?;

    let directory = root.join("release/rationales");
    let mut entries = fs::read_dir(&directory)
        .map_err(|error| format!("read {}: {error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read {} entry: {error}", directory.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    let mut rationale_ids = BTreeSet::new();
    let mut unit_ids = BTreeSet::new();
    for entry in entries {
        let path = entry.path();
        if !entry
            .file_type()
            .map_err(|error| format!("inspect {}: {error}", path.display()))?
            .is_file()
            || path.extension().and_then(|extension| extension.to_str()) != Some("json")
        {
            return Err(format!(
                "version rationale directory may contain only JSON files: {}",
                path.display()
            ));
        }
        let record = read_json(&path)?;
        validate_version_rationale(root, &catalog, &record)?;
        let record = object(&record, "version rationale")?;
        let rationale_id = text(record.get("rationaleId"), "version rationale ID")?;
        let unit_id = text(record.get("unitId"), "version rationale unit ID")?;
        let expected_name = format!("{rationale_id}.json");
        if path.file_name().and_then(|name| name.to_str()) != Some(expected_name.as_str()) {
            return Err(format!(
                "version rationale file name must match rationale ID: {}",
                path.display()
            ));
        }
        if !rationale_ids.insert(rationale_id.to_owned()) || !unit_ids.insert(unit_id.to_owned()) {
            return Err("version rationale IDs and catalog unit IDs must be unique".to_owned());
        }
    }
    Ok(())
}

pub fn validate_version_rationale(
    root: &Path,
    catalog: &Value,
    record: &Value,
) -> Result<(), String> {
    validate_version_rationale_schema(root, record)?;
    validate_catalog(root, catalog)?;
    let lifecycle = read_json(&root.join("release/catalog-lifecycle.json"))?;
    validate_catalog_lifecycle_schema(root, &lifecycle)?;
    let catalog_revision = text(
        lifecycle.get("catalogRevision"),
        "catalog lifecycle revision for version rationale",
    )?;
    ensure_no_private_leakage(&serde_json::to_string(record).map_err(|error| {
        format!("serialize version rationale for boundary validation: {error}")
    })?)?;

    let rationale = object(record, "version rationale")?;
    let rationale_id = text(rationale.get("rationaleId"), "version rationale ID")?;
    let expected_record_id = format!("https://vexil.dev/release/rationales/{rationale_id}.json");
    if text(rationale.get("$id"), "version rationale public ID")? != expected_record_id {
        return Err("version rationale public ID must match its rationale ID".to_owned());
    }
    let unit_id = text(rationale.get("unitId"), "version rationale unit ID")?;
    let catalog_units = array(catalog.get("units"), "release catalog units")?;
    let unit = catalog_units
        .iter()
        .find(|unit| unit.get("id").and_then(Value::as_str) == Some(unit_id))
        .ok_or_else(|| format!("version rationale references unknown catalog unit: {unit_id}"))?;
    let unit = object(unit, "version rationale catalog unit")?;
    let publication = object(required_value(unit, "publication")?, "catalog publication")?;
    if text(
        publication.get("classification"),
        "catalog publication classification",
    )? == "non-publishable"
    {
        return Err("version rationale must bind a publishable catalog unit".to_owned());
    }
    let proposed = text(
        rationale.get("proposedPackageVersion"),
        "version rationale proposed package version",
    )?;
    validate_strict_semver(proposed)?;
    let version_source = object(
        required_value(unit, "versionSource")?,
        "catalog version source",
    )?;
    if text(
        version_source.get("observedDeclaration"),
        "catalog version declaration",
    )? != proposed
    {
        return Err(
            "version rationale proposed package version must equal the checked-in catalog declaration"
                .to_owned(),
        );
    }

    let change_class = text(
        rationale.get("changeClass"),
        "version rationale change class",
    )?;
    let previous = object(
        required_value(rationale, "previousPackageVersion")?,
        "version rationale previous package version",
    )?;
    let previous_kind = text(previous.get("kind"), "previous package version kind")?;
    if previous_kind != "initial-non-release-baseline"
        || !previous.get("version").unwrap_or(&Value::Null).is_null()
        || change_class != "initial-source-version"
    {
        return Err(
            "until a dedicated public provenance contract exists, rationales must use an explicit initial non-release baseline and initial-source-version change class"
                .to_owned(),
        );
    }

    let surfaces = array(rationale.get("affectedSurfaces"), "affected surfaces")?;
    let mut previous_surface: Option<(&str, &str)> = None;
    let mut namespaces = BTreeSet::new();
    let mut behavior_changed = false;
    for surface in surfaces {
        let surface = object(surface, "affected surface")?;
        let namespace = text(surface.get("namespace"), "affected surface namespace")?;
        let name = text(surface.get("surface"), "affected surface name")?;
        if previous_surface.is_some_and(|previous| (namespace, name) <= previous)
            || !namespaces.insert(namespace)
        {
            return Err(
                "affected-surface assessments must be unique and sorted by namespace and surface"
                    .to_owned(),
            );
        }
        previous_surface = Some((namespace, name));
        let authority_path = text(
            surface.get("authorityPath"),
            "affected surface authority path",
        )?;
        if text(
            surface.get("authorityRevision"),
            "affected surface authority revision",
        )? != catalog_revision
        {
            return Err(
                "affected surface authority revision must equal the canonical catalog lifecycle revision"
                    .to_owned(),
            );
        }
        let authority_path_value = Path::new(authority_path);
        if authority_path_value.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        }) {
            return Err(
                "affected surface authority path must stay within its public authority root"
                    .to_owned(),
            );
        }
        let authority_root = authority_path
            .split('/')
            .next()
            .ok_or("affected surface authority path is empty")?;
        let authority_root = fs::canonicalize(root.join(authority_root))
            .map_err(|error| format!("canonicalize authority root {authority_root}: {error}"))?;
        let resolved_authority_path = fs::canonicalize(root.join(authority_path));
        if !resolved_authority_path
            .as_ref()
            .is_ok_and(|path| path.is_file() && path.starts_with(&authority_root))
        {
            return Err(format!(
                "affected surface authority path is missing: {authority_path}"
            ));
        }
        if namespace == "language-spec"
            && text(surface.get("languageStatus"), "language status")? == "draft"
            && text(surface.get("assertion"), "language assessment assertion")?
                == "formal-conformance"
        {
            return Err("draft language status cannot be claimed as formal conformance".to_owned());
        }
        behavior_changed |= matches!(
            text(
                surface.get("compatibility"),
                "affected surface compatibility"
            )?,
            "behavior-changed" | "public-api-changed"
        );
    }
    if namespaces.len() != 3
        || !namespaces.contains("language-spec")
        || !namespaces.contains("wire-format")
        || !namespaces.contains("package-api")
    {
        return Err("version rationales must assess language-spec, wire-format, and package-api independently".to_owned());
    }
    behavior_changed |= matches!(change_class, "behavior-change" | "public-api-change");

    let evidence_identity = text(
        rationale.get("compatibilityEvidenceIdentity"),
        "compatibility evidence identity",
    )?;
    let support_matrix = object(
        required_value(rationale, "supportMatrix")?,
        "version rationale support matrix",
    )?;
    let mut support_claims = BTreeSet::new();
    for claim in array(support_matrix.get("claims"), "support matrix claims")? {
        let claim = object(claim, "support matrix claim")?;
        let platform = text(claim.get("platform"), "support claim platform")?;
        let language_version = text(
            claim.get("languageVersion"),
            "support claim language version",
        )?;
        if !support_claims.insert((platform, language_version)) {
            return Err(
                "support matrix claims must be unique per platform and language version".to_owned(),
            );
        }
        if text(
            claim.get("evidenceIdentity"),
            "support claim evidence identity",
        )? != evidence_identity
        {
            return Err(
                "support claims must link exactly to the rationale compatibility evidence identity"
                    .to_owned(),
            );
        }
    }

    let impact = object(
        required_value(rationale, "dependencyImpact")?,
        "version rationale dependency impact",
    )?;
    let mut prior_affected_unit = "";
    for affected_unit in array(impact.get("affectedUnitIds"), "dependency impact unit IDs")? {
        let affected_unit = text(Some(affected_unit), "dependency impact unit ID")?;
        if affected_unit <= prior_affected_unit
            || !catalog_units
                .iter()
                .any(|unit| unit.get("id").and_then(Value::as_str) == Some(affected_unit))
        {
            return Err(
                "dependency-impact unit IDs must be known catalog units in stable order".to_owned(),
            );
        }
        prior_affected_unit = affected_unit;
    }

    let review = object(
        required_value(rationale, "packageStewardReview")?,
        "version rationale Package Steward review",
    )?;
    require_utc_timestamp(review.get("reviewedAt"), "Package Steward review timestamp")?;
    let owner = object(required_value(unit, "owner")?, "catalog unit owner")?;
    if text(review.get("roleId"), "Package Steward review role")? != "package-steward"
        || review.get("assignmentId") != owner.get("assignmentId")
    {
        return Err(
            "version rationale review must be attributed to the unit Package Steward".to_owned(),
        );
    }
    let assignments = read_json(&root.join("release/stewardship/assignments.json"))?;
    let assignments = array(assignments.get("assignments"), "stewardship assignments")?;
    let assignment_id = text(review.get("assignmentId"), "Package Steward assignment ID")?;
    let assignment = assignments
        .iter()
        .find(|assignment| {
            assignment.get("assignmentId").and_then(Value::as_str) == Some(assignment_id)
        })
        .ok_or("version rationale review references an unknown Package Steward assignment")?;
    let assignment = object(assignment, "Package Steward assignment")?;
    if text(assignment.get("roleId"), "Package Steward assignment role")? != "package-steward"
        || text(
            assignment.get("status"),
            "Package Steward assignment status",
        )? != "active"
        || review.get("actorId") != assignment.get("primaryActorId")
    {
        return Err(
            "version rationale review actor is not the active unit Package Steward".to_owned(),
        );
    }

    let decision = rationale
        .get("publicCompatibilityDecision")
        .ok_or("missing public compatibility decision field")?;
    if behavior_changed && decision.is_null() {
        return Err(
            "behavior or public API compatibility changes require an approved public decision"
                .to_owned(),
        );
    }
    if !decision.is_null() {
        let decision = object(decision, "public compatibility decision")?;
        ensure_public_decision_source(text(
            decision.get("source"),
            "public compatibility decision source",
        )?)?;
        require_utc_timestamp(
            decision.get("approvedAt"),
            "public compatibility decision timestamp",
        )?;
    }
    Ok(())
}

pub fn validate_catalog(root: &Path, catalog: &Value) -> Result<(), String> {
    let catalog = object(catalog, "release catalog")?;
    let expected_roots = expected_catalog_source_roots(root)?;
    let units = array(catalog.get("units"), "release catalog units")?;
    let mut ids = BTreeSet::new();
    let mut roots = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let mut tag_namespaces = BTreeSet::new();
    let mut previous_id = "";
    for unit in units {
        let unit = object(unit, "release catalog unit")?;
        let id = text(unit.get("id"), "release catalog unit id")?;
        if id <= previous_id || !ids.insert(id) {
            return Err("release catalog unit IDs must be unique".to_owned());
        }
        previous_id = id;
        let source_root = text(unit.get("sourceRoot"), "release catalog source root")?;
        if !roots.insert(source_root.to_owned()) {
            return Err("release catalog source roots must be unique".to_owned());
        }
        if !root.join(source_root).is_dir() {
            return Err(format!(
                "release catalog source root is missing: {source_root}"
            ));
        }
        validate_catalog_kind(root, unit)?;
        validate_catalog_owner(root, source_root, unit)?;
        for target in array(unit.get("targets"), "release catalog targets")? {
            let target = object(target, "release catalog target")?;
            let identity = format!(
                "{}:{}",
                text(target.get("kind"), "target kind")?,
                text(target.get("name"), "target name")?
            );
            if !targets.insert(identity) {
                return Err("release catalog targets must be unique by kind and name".to_owned());
            }
        }
        validate_catalog_publication(unit)?;
        validate_catalog_version_source(root, id, unit)?;
        validate_catalog_targets(root, unit)?;
        validate_catalog_changelog(root, unit)?;
        let namespace = text(unit.get("canonicalTagNamespace"), "catalog tag namespace")?;
        let status = text(
            object(required_value(unit, "publication")?, "catalog publication")?.get("status"),
            "catalog publication status",
        )?;
        if status == "non-publishable" {
            if namespace != "not-applicable" {
                return Err(
                    "non-publishable catalog units must declare no canonical tag namespace"
                        .to_owned(),
                );
            }
        } else {
            if !tag_namespaces.insert(namespace) {
                return Err("release catalog canonical tag namespaces must be unique".to_owned());
            }
            let expected_namespace = if id.starts_with("vexil-codegen-")
                || matches!(id, "vexil-lang" | "vexil-runtime" | "vexil-store")
            {
                format!("{id}-v<semver>")
            } else {
                match id {
                    "vexil-runtime-go" => "packages/runtime-go/v<semver>".to_owned(),
                    "vexil-runtime-ts" => "vexil-runtime-ts-v<semver>".to_owned(),
                    "vexil-runtime-py" => "vexil-runtime-py-v<semver>".to_owned(),
                    "vexilc" => "vexilc-v<semver>".to_owned(),
                    _ => {
                        return Err(format!(
                            "catalog unit {id} has an unexpected publishable tag namespace"
                        ))
                    }
                }
            };
            if namespace != expected_namespace {
                return Err(format!("catalog unit {id} must use {expected_namespace}"));
            }
        }
    }
    if roots != expected_roots {
        return Err("release catalog must enumerate each maintained and non-publishable source unit exactly once".to_owned());
    }
    let go = units
        .iter()
        .find(|unit| unit["id"] == "vexil-runtime-go")
        .ok_or("release catalog is missing the Go runtime")?;
    let go = object(go, "Go runtime catalog unit")?;
    let go_version = object(required_value(go, "versionSource")?, "Go version source")?;
    let observed_go_version = text(
        go_version.get("observedDeclaration"),
        "Go version source observed declaration",
    )?;
    if text(
        object(required_value(go, "publication")?, "Go publication")?.get("status"),
        "Go status",
    )? != "source-inventory-only"
        || text(go_version.get("path"), "Go version source path")? != "packages/runtime-go/VERSION"
        || text(go_version.get("format"), "Go version source format")? != "go-version-file"
    {
        return Err(
            "Go runtime must use its checked-in VERSION source without asserting release readiness"
                .to_owned(),
        );
    }
    validate_go_version_decision(root, observed_go_version)?;
    let python = units
        .iter()
        .find(|unit| unit["id"] == "vexil-runtime-py")
        .ok_or("release catalog is missing the Python runtime")?;
    if text(
        object(
            required_value(
                object(python, "Python runtime catalog unit")?,
                "publication",
            )?,
            "Python publication",
        )?
        .get("status"),
        "Python status",
    )? != "candidate-unreleased"
    {
        return Err("Python runtime must be cataloged as candidate-unreleased".to_owned());
    }
    validate_and_derive_release_order(root, catalog)?;
    Ok(())
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PublishBeforeEdge {
    dependency_id: String,
    dependent_id: String,
    source_kind: String,
    manifest_path: String,
    location: String,
}

#[derive(Clone, Debug)]
struct CatalogTarget {
    id: String,
    status: String,
    source_root: String,
}

impl PublishBeforeEdge {
    fn description(&self) -> String {
        format!(
            "{} -> {} ({}#{})",
            self.dependency_id, self.dependent_id, self.manifest_path, self.location
        )
    }
}

/// Returns the deterministic structural order implied by the canonical typed
/// graph. This does not select a Release Set or authorize publication.
pub fn derive_release_order(root: &Path, catalog: &Value) -> Result<Vec<String>, String> {
    validate_catalog_schema(root, catalog)?;
    validate_catalog(root, catalog)?;
    validate_and_derive_release_order(root, object(catalog, "release catalog")?)
}

fn validate_and_derive_release_order(
    root: &Path,
    catalog: &Map<String, Value>,
) -> Result<Vec<String>, String> {
    let units = array(catalog.get("units"), "release catalog units")?;
    let targets = catalog_target_index(units)?;
    let manifest_edges = manifest_publish_before_edges(root, units, &targets)?;
    let catalog_edges = catalog_publish_before_edges(root, units)?;

    if let Some(edge) = manifest_edges.difference(&catalog_edges).next() {
        return Err(format!(
            "catalog is missing manifest-derived publish_before edge {}",
            edge.description()
        ));
    }
    if let Some(edge) = catalog_edges.difference(&manifest_edges).next() {
        return Err(format!(
            "catalog publish_before edge conflicts with current manifests: {}",
            edge.description()
        ));
    }
    release_order_from_edges(units, &manifest_edges)
}

fn catalog_target_index(
    units: &[Value],
) -> Result<BTreeMap<(String, String), CatalogTarget>, String> {
    let mut targets = BTreeMap::new();
    for unit in units {
        let unit = object(unit, "release catalog unit")?;
        let id = text(unit.get("id"), "release catalog unit id")?.to_owned();
        let source_root = text(unit.get("sourceRoot"), "release catalog source root")?.to_owned();
        let publication = object(required_value(unit, "publication")?, "catalog publication")?;
        let status = text(publication.get("status"), "catalog publication status")?.to_owned();
        for target in array(unit.get("targets"), "release catalog targets")? {
            let target = object(target, "release catalog target")?;
            let key = (
                text(target.get("kind"), "catalog target kind")?.to_owned(),
                text(target.get("name"), "catalog target name")?.to_owned(),
            );
            if targets
                .insert(
                    key,
                    CatalogTarget {
                        id: id.clone(),
                        status: status.clone(),
                        source_root: source_root.clone(),
                    },
                )
                .is_some()
            {
                return Err("release catalog targets must be unique by kind and name".to_owned());
            }
        }
    }
    Ok(targets)
}

fn manifest_publish_before_edges(
    root: &Path,
    units: &[Value],
    targets: &BTreeMap<(String, String), CatalogTarget>,
) -> Result<BTreeSet<PublishBeforeEdge>, String> {
    let mut edges = BTreeSet::new();
    for unit in units {
        let unit = object(unit, "release catalog unit")?;
        let publication = object(required_value(unit, "publication")?, "catalog publication")?;
        if text(publication.get("status"), "catalog publication status")? != "source-inventory-only"
        {
            continue;
        }
        let dependent_id = text(unit.get("id"), "release catalog unit id")?;
        let source_root = text(unit.get("sourceRoot"), "release catalog source root")?;
        match text(unit.get("kind"), "release catalog unit kind")? {
            "rust-package" => {
                let manifest_path = format!("{source_root}/Cargo.toml");
                let manifest = parse_toml(
                    &fs::read_to_string(root.join(&manifest_path))
                        .map_err(|error| format!("read {manifest_path}: {error}"))?,
                )?;
                if let Some(dependencies) = manifest.get("dependencies") {
                    let dependencies = dependencies
                        .as_table()
                        .ok_or_else(|| format!("{manifest_path} [dependencies] must be a table"))?;
                    edges.extend(cargo_runtime_path_edges(
                        root,
                        source_root,
                        dependent_id,
                        &manifest_path,
                        "dependencies",
                        dependencies,
                        targets,
                    )?);
                }
                if let Some(dependencies) = manifest.get("build-dependencies") {
                    let dependencies = dependencies.as_table().ok_or_else(|| {
                        format!("{manifest_path} [build-dependencies] must be a table")
                    })?;
                    edges.extend(cargo_runtime_path_edges(
                        root,
                        source_root,
                        dependent_id,
                        &manifest_path,
                        "build-dependencies",
                        dependencies,
                        targets,
                    )?);
                }
                if let Some(target_tables) = manifest.get("target") {
                    let target_tables = target_tables
                        .as_table()
                        .ok_or_else(|| format!("{manifest_path} target must be a table"))?;
                    for (target_name, target_table) in target_tables {
                        let target_table = target_table.as_table().ok_or_else(|| {
                            format!("{manifest_path} target.{target_name} must be a table")
                        })?;
                        if let Some(dependencies) = target_table.get("dependencies") {
                            let dependencies = dependencies.as_table().ok_or_else(|| {
                                format!(
                                    "{manifest_path} target.{target_name}.dependencies must be a table"
                                )
                            })?;
                            edges.extend(cargo_runtime_path_edges(
                                root,
                                source_root,
                                dependent_id,
                                &manifest_path,
                                &format!("target.{target_name}.dependencies"),
                                dependencies,
                                targets,
                            )?);
                        }
                        if let Some(dependencies) = target_table.get("build-dependencies") {
                            let dependencies = dependencies.as_table().ok_or_else(|| {
                                format!(
                                    "{manifest_path} target.{target_name}.build-dependencies must be a table"
                                )
                            })?;
                            edges.extend(cargo_runtime_path_edges(
                                root,
                                source_root,
                                dependent_id,
                                &manifest_path,
                                &format!("target.{target_name}.build-dependencies"),
                                dependencies,
                                targets,
                            )?);
                        }
                    }
                }
            }
            "typescript-runtime" => {
                let manifest_path = format!("{source_root}/package.json");
                let manifest: Value = serde_json::from_str(
                    &fs::read_to_string(root.join(&manifest_path))
                        .map_err(|error| format!("read {manifest_path}: {error}"))?,
                )
                .map_err(|error| format!("parse {manifest_path}: {error}"))?;
                if let Some(dependencies) = manifest.get("dependencies") {
                    let dependencies = dependencies
                        .as_object()
                        .ok_or_else(|| format!("{manifest_path} dependencies must be an object"))?;
                    for name in dependencies.keys() {
                        let location = format!("dependencies.{name}");
                        if let Some(dependency_id) = resolve_known_manifest_dependency(
                            targets,
                            "npm-package",
                            name,
                            dependent_id,
                            &manifest_path,
                            &location,
                        )? {
                            edges.insert(PublishBeforeEdge {
                                dependency_id,
                                dependent_id: dependent_id.to_owned(),
                                source_kind: "npm-runtime-dependency".to_owned(),
                                manifest_path: manifest_path.clone(),
                                location,
                            });
                        }
                    }
                }
            }
            "python-runtime" => {
                let manifest_path = format!("{source_root}/pyproject.toml");
                let manifest = parse_toml(
                    &fs::read_to_string(root.join(&manifest_path))
                        .map_err(|error| format!("read {manifest_path}: {error}"))?,
                )?;
                let dependencies = manifest
                    .get("project")
                    .and_then(TomlValue::as_table)
                    .and_then(|project| project.get("dependencies"));
                if let Some(dependencies) = dependencies {
                    let dependencies = dependencies.as_array().ok_or_else(|| {
                        format!("{manifest_path} project.dependencies must be an array")
                    })?;
                    for requirement in dependencies {
                        let requirement = requirement.as_str().ok_or_else(|| {
                            format!("{manifest_path} project.dependencies must contain strings")
                        })?;
                        let name = python_requirement_name(requirement)?;
                        let location = format!("project.dependencies.{name}");
                        if let Some(dependency_id) = resolve_known_manifest_dependency(
                            targets,
                            "python-project",
                            &name,
                            dependent_id,
                            &manifest_path,
                            &location,
                        )? {
                            edges.insert(PublishBeforeEdge {
                                dependency_id,
                                dependent_id: dependent_id.to_owned(),
                                source_kind: "python-runtime-dependency".to_owned(),
                                manifest_path: manifest_path.clone(),
                                location,
                            });
                        }
                    }
                }
            }
            "go-module" => {
                let manifest_path = format!("{source_root}/go.mod");
                let requirements = go_runtime_requirements(
                    &fs::read_to_string(root.join(&manifest_path))
                        .map_err(|error| format!("read {manifest_path}: {error}"))?,
                )?;
                for module in requirements {
                    let location = format!("require {module}");
                    if let Some(dependency_id) = resolve_known_manifest_dependency(
                        targets,
                        "go-module",
                        &module,
                        dependent_id,
                        &manifest_path,
                        &location,
                    )? {
                        edges.insert(PublishBeforeEdge {
                            dependency_id,
                            dependent_id: dependent_id.to_owned(),
                            source_kind: "go-runtime-require".to_owned(),
                            manifest_path: manifest_path.clone(),
                            location,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Ok(edges)
}

fn cargo_runtime_path_edges(
    root: &Path,
    source_root: &str,
    dependent_id: &str,
    manifest_path: &str,
    location_prefix: &str,
    dependencies: &TomlTable,
    targets: &BTreeMap<(String, String), CatalogTarget>,
) -> Result<Vec<PublishBeforeEdge>, String> {
    let mut edges = Vec::new();
    for (name, declaration) in dependencies {
        let Some(declaration) = declaration.as_table() else {
            continue;
        };
        let (declaration, dependency_path_root) =
            if declaration.get("workspace").and_then(TomlValue::as_bool) == Some(true) {
                (
                    workspace_cargo_dependency(root, name, manifest_path, location_prefix)?,
                    "",
                )
            } else {
                (TomlValue::Table(declaration.clone()), source_root)
            };
        let declaration = declaration
            .as_table()
            .expect("workspace Cargo dependency declarations are normalized to tables");
        if !declaration.contains_key("path") {
            continue;
        }
        let location = format!("{location_prefix}.{name}");
        let path = declaration.get("path").and_then(TomlValue::as_str).ok_or_else(|| {
            format!(
                "catalog unit {dependent_id} has a malformed Cargo path dependency at {manifest_path}#{location}"
            )
        })?;
        if declaration
            .get("version")
            .and_then(TomlValue::as_str)
            .filter(|version| !version.is_empty())
            .is_none()
        {
            return Err(format!(
                "catalog unit {dependent_id} has a publishable Cargo path dependency without a registry version at {manifest_path}#{location}"
            ));
        }
        let package = declaration
            .get("package")
            .and_then(TomlValue::as_str)
            .unwrap_or(name);
        let dependency = resolve_manifest_dependency(
            targets,
            "cargo-package",
            package,
            dependent_id,
            manifest_path,
            &location,
        )?;
        validate_cargo_path_matches_catalog_unit(
            root,
            dependency_path_root,
            path,
            &dependency.source_root,
            dependent_id,
            manifest_path,
            &location,
        )?;
        edges.push(PublishBeforeEdge {
            dependency_id: dependency.id,
            dependent_id: dependent_id.to_owned(),
            source_kind: if location_prefix.ends_with("build-dependencies") {
                "cargo-build-dependency"
            } else {
                "cargo-runtime-dependency"
            }
            .to_owned(),
            manifest_path: manifest_path.to_owned(),
            location,
        });
    }
    Ok(edges)
}

fn workspace_cargo_dependency(
    root: &Path,
    name: &str,
    manifest_path: &str,
    location_prefix: &str,
) -> Result<TomlValue, String> {
    let workspace = parse_toml(
        &fs::read_to_string(root.join("Cargo.toml"))
            .map_err(|error| format!("read workspace manifest for {manifest_path}: {error}"))?,
    )?;
    workspace
        .get("workspace")
        .and_then(TomlValue::as_table)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(TomlValue::as_table)
        .and_then(|dependencies| dependencies.get(name))
        .cloned()
        .ok_or_else(|| format!(
            "catalog unit workspace Cargo dependency is not declared at Cargo.toml#workspace.dependencies.{name} for {manifest_path}#{location_prefix}.{name}"
        ))
}

fn validate_cargo_path_matches_catalog_unit(
    root: &Path,
    dependent_source_root: &str,
    declared_path: &str,
    expected_source_root: &str,
    dependent_id: &str,
    manifest_path: &str,
    location: &str,
) -> Result<(), String> {
    let resolved_path = fs::canonicalize(root.join(dependent_source_root).join(declared_path))
        .map_err(|error| {
            format!(
                "catalog unit {dependent_id} has an unreadable Cargo path dependency at {manifest_path}#{location}: {error}"
            )
        })?;
    let expected_path = fs::canonicalize(root.join(expected_source_root)).map_err(|error| {
        format!("read catalog dependency source root {expected_source_root}: {error}")
    })?;
    if resolved_path != expected_path {
        return Err(format!(
            "catalog unit {dependent_id} Cargo path dependency does not resolve to catalog source root {expected_source_root} at {manifest_path}#{location}"
        ));
    }
    Ok(())
}

fn resolve_manifest_dependency(
    targets: &BTreeMap<(String, String), CatalogTarget>,
    target_kind: &str,
    target_name: &str,
    dependent_id: &str,
    manifest_path: &str,
    location: &str,
) -> Result<CatalogTarget, String> {
    let Some(target) = targets.get(&(target_kind.to_owned(), target_name.to_owned())) else {
        return Err(format!(
            "catalog unit {dependent_id} references missing catalog unit {target_name} at {manifest_path}#{location}"
        ));
    };
    if target.status != "source-inventory-only" {
        return Err(format!(
            "catalog unit {dependent_id} references non-publishable or out-of-scope catalog unit {} at {manifest_path}#{location}",
            target.id
        ));
    }
    Ok(target.clone())
}

fn resolve_known_manifest_dependency(
    targets: &BTreeMap<(String, String), CatalogTarget>,
    target_kind: &str,
    target_name: &str,
    dependent_id: &str,
    manifest_path: &str,
    location: &str,
) -> Result<Option<String>, String> {
    let target_name = if target_kind == "python-project" {
        normalize_python_project_name(target_name)
    } else {
        target_name.to_owned()
    };
    let matches = targets
        .iter()
        .filter(|((kind, name), _)| {
            kind == target_kind
                && if target_kind == "python-project" {
                    normalize_python_project_name(name) == target_name
                } else {
                    name == &target_name
                }
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [((kind, name), _)] => resolve_manifest_dependency(
            targets,
            kind,
            name,
            dependent_id,
            manifest_path,
            location,
        )
        .map(|target| Some(target.id)),
        _ if target_kind == "python-project" => Err(format!(
            "catalog contains ambiguous normalized Python target {target_name} referenced by catalog unit {dependent_id} at {manifest_path}#{location}"
        )),
        _ => Err(format!(
            "catalog contains ambiguous target {target_name} referenced by catalog unit {dependent_id} at {manifest_path}#{location}"
        )),
    }
}

fn python_requirement_name(requirement: &str) -> Result<String, String> {
    let name = requirement
        .trim()
        .split(|character: char| {
            character.is_whitespace()
                || matches!(character, '[' | '<' | '>' | '=' | '!' | '~' | ';')
        })
        .next()
        .unwrap_or_default();
    if name.is_empty() {
        return Err("Python runtime dependency must have a package name".to_owned());
    }
    Ok(name.to_owned())
}

fn normalize_python_project_name(name: &str) -> String {
    name.to_ascii_lowercase().replace(['_', '.'], "-")
}

fn go_runtime_requirements(content: &str) -> Result<Vec<String>, String> {
    let mut requirements = Vec::new();
    let mut in_block = false;
    for raw_line in content.lines() {
        let line = raw_line
            .split_once("//")
            .map_or(raw_line, |(before, _)| before)
            .trim();
        if line.is_empty() {
            continue;
        }
        if line == "require (" {
            in_block = true;
            continue;
        }
        if in_block && line == ")" {
            in_block = false;
            continue;
        }
        let requirement = if in_block {
            Some(line)
        } else {
            line.strip_prefix("require ")
        };
        let Some(requirement) = requirement else {
            continue;
        };
        let mut fields = requirement.split_whitespace();
        let module = fields
            .next()
            .ok_or("Go require declaration has no module path")?;
        if fields.next().is_none() {
            return Err(format!(
                "Go require declaration has no version for {module}"
            ));
        }
        requirements.push(module.to_owned());
    }
    if in_block {
        return Err("Go require block is not closed".to_owned());
    }
    Ok(requirements)
}

fn catalog_publish_before_edges(
    root: &Path,
    units: &[Value],
) -> Result<BTreeSet<PublishBeforeEdge>, String> {
    let mut publish_before = BTreeSet::new();
    let known_ids: BTreeSet<&str> = units
        .iter()
        .filter_map(|unit| unit.get("id").and_then(Value::as_str))
        .collect();
    for unit in units {
        let unit = object(unit, "release catalog unit")?;
        let dependent_id = text(unit.get("id"), "release catalog unit id")?;
        let source_root = text(unit.get("sourceRoot"), "release catalog source root")?;
        let edges = array(unit.get("dependencyEdges"), "catalog dependency edges")?;
        let mut keys = Vec::new();
        let mut directed = BTreeSet::new();
        for edge in edges {
            let edge = object(edge, "catalog dependency edge")?;
            let edge_type = text(edge.get("edgeType"), "catalog dependency edge type")?;
            if !matches!(edge_type, "publish_before" | "compatibility" | "bundle") {
                return Err(format!(
                    "catalog unit {dependent_id} has an unknown dependency edge type {edge_type}"
                ));
            }
            let related_id = text(edge.get("relatedUnitId"), "catalog related unit id")?;
            if !known_ids.contains(related_id) {
                return Err(format!(
                    "catalog unit {dependent_id} references missing related unit {related_id}"
                ));
            }
            let direction = text(edge.get("direction"), "catalog dependency edge direction")?;
            if direction != "related-before-unit" {
                return Err(format!(
                    "catalog unit {dependent_id} must use related-before-unit dependency direction"
                ));
            }
            let evidence = object(
                required_value(edge, "sourceEvidence")?,
                "catalog dependency evidence",
            )?;
            let source_kind = text(
                evidence.get("sourceKind"),
                "catalog dependency evidence kind",
            )?;
            let manifest_path = text(evidence.get("path"), "catalog dependency evidence path")?;
            let location = text(
                evidence.get("location"),
                "catalog dependency evidence location",
            )?;
            if location.is_empty() {
                return Err("catalog dependency evidence location must not be empty".to_owned());
            }
            if edge_type == "publish_before" {
                if !matches!(
                    source_kind,
                    "cargo-runtime-dependency"
                        | "npm-runtime-dependency"
                        | "python-runtime-dependency"
                        | "go-runtime-require"
                ) {
                    return Err(format!(
                        "catalog unit {dependent_id} publish_before edges must use runtime manifest evidence"
                    ));
                }
                require_catalog_path_within_source_root(
                    source_root,
                    manifest_path,
                    "dependency evidence",
                )?;
            } else {
                validate_non_ordering_edge_decision(
                    root,
                    dependent_id,
                    related_id,
                    edge_type,
                    source_kind,
                    manifest_path,
                    location,
                )?;
            }
            let key = format!(
                "{edge_type}\u{0}{related_id}\u{0}{source_kind}\u{0}{manifest_path}\u{0}{location}"
            );
            keys.push(key);
            if !directed.insert((edge_type, related_id)) {
                return Err(format!(
                    "catalog unit {dependent_id} has duplicate dependency edge declarations"
                ));
            }
            if edge_type == "publish_before" {
                publish_before.insert(PublishBeforeEdge {
                    dependency_id: related_id.to_owned(),
                    dependent_id: dependent_id.to_owned(),
                    source_kind: source_kind.to_owned(),
                    manifest_path: manifest_path.to_owned(),
                    location: location.to_owned(),
                });
            }
        }
        if keys.windows(2).any(|window| window[0] > window[1]) {
            return Err(format!(
                "catalog unit {dependent_id} dependency edges must use stable sort order"
            ));
        }
    }
    Ok(publish_before)
}

fn validate_non_ordering_edge_decision(
    root: &Path,
    dependent_id: &str,
    related_id: &str,
    edge_type: &str,
    source_kind: &str,
    path: &str,
    decision_id: &str,
) -> Result<(), String> {
    if source_kind != "release-dependency-edge-decision" {
        return Err(format!(
            "catalog unit {dependent_id} {edge_type} edge must cite a release-dependency-edge-decision"
        ));
    }
    let decision_path = Path::new(path);
    if decision_path.is_absolute()
        || decision_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || !decision_path.starts_with(Path::new("release/decisions"))
        || decision_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
    {
        return Err(format!(
            "catalog unit {dependent_id} {edge_type} edge must cite a public JSON record under release/decisions"
        ));
    }
    let decision = read_json(&root.join(decision_path))?;
    let decision = object(&decision, "release dependency edge decision")?;
    require_string(decision, "recordKind", "release-dependency-edge-decision")?;
    require_string(decision, "status", "approved")?;
    require_string(decision, "decisionId", decision_id)?;
    require_string(decision, "edgeType", edge_type)?;
    require_string(decision, "dependentUnitId", dependent_id)?;
    require_string(decision, "relatedUnitId", related_id)?;
    Ok(())
}

fn release_order_from_edges(
    units: &[Value],
    edges: &BTreeSet<PublishBeforeEdge>,
) -> Result<Vec<String>, String> {
    let mut indegree = BTreeMap::new();
    let mut successors: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for unit in units {
        let unit = object(unit, "release catalog unit")?;
        let publication = object(required_value(unit, "publication")?, "catalog publication")?;
        if text(publication.get("status"), "catalog publication status")? == "source-inventory-only"
        {
            let id = text(unit.get("id"), "release catalog unit id")?.to_owned();
            indegree.insert(id.clone(), 0usize);
            successors.insert(id, BTreeSet::new());
        }
    }
    for edge in edges {
        let successor = successors.get_mut(&edge.dependency_id).ok_or_else(|| {
            format!(
                "catalog publish_before edge has missing dependency unit {}",
                edge.dependency_id
            )
        })?;
        if !indegree.contains_key(&edge.dependent_id) {
            return Err(format!(
                "catalog publish_before edge has non-publishable dependent unit {}",
                edge.dependent_id
            ));
        }
        if successor.insert(edge.dependent_id.clone()) {
            *indegree
                .get_mut(&edge.dependent_id)
                .expect("validated dependent exists") += 1;
        }
    }
    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut order = Vec::new();
    while let Some(id) = ready.pop_first() {
        order.push(id.clone());
        for successor in successors[&id].clone() {
            let degree = indegree
                .get_mut(&successor)
                .expect("validated successor exists");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(successor);
            }
        }
    }
    if order.len() == indegree.len() {
        return Ok(order);
    }
    let remaining: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree > 0)
        .map(|(id, _)| id.clone())
        .collect();
    let conflicts = edges
        .iter()
        .filter(|edge| {
            remaining.contains(&edge.dependency_id)
                && remaining.contains(&edge.dependent_id)
                && graph_reaches(
                    &successors,
                    &edge.dependent_id,
                    &edge.dependency_id,
                    &remaining,
                )
        })
        .map(PublishBeforeEdge::description)
        .collect::<Vec<_>>();
    Err(format!(
        "publish_before cycle blocks release order: {}",
        conflicts.join(", ")
    ))
}

fn graph_reaches(
    successors: &BTreeMap<String, BTreeSet<String>>,
    start: &str,
    target: &str,
    allowed: &BTreeSet<String>,
) -> bool {
    let mut pending = vec![start.to_owned()];
    let mut visited = BTreeSet::new();
    while let Some(current) = pending.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if current == target {
            return true;
        }
        if let Some(next) = successors.get(&current) {
            pending.extend(next.iter().filter(|id| allowed.contains(*id)).cloned());
        }
    }
    false
}

/// Validates an explicitly proposed, future component tag without treating a
/// catalog observation as an implicit release proposal or contacting a provider.
pub fn validate_candidate_tag(root: &Path, catalog: &Value, candidate: &str) -> Result<(), String> {
    validate_catalog_schema(root, catalog)?;
    validate_catalog(root, catalog)?;
    if candidate.trim() != candidate || candidate.is_empty() {
        return Err("candidate tag must be a non-empty, whitespace-free string".to_owned());
    }
    if let Some(version) = candidate.strip_prefix('v') {
        if validate_strict_semver(version).is_ok() {
            return Err(
                "project-wide root v<semver> tags are prohibited during recovery".to_owned(),
            );
        }
    }
    let mut matched_unit = None;
    for unit in array(catalog.get("units"), "release catalog units")? {
        let unit = object(unit, "release catalog unit")?;
        let namespace = text(unit.get("canonicalTagNamespace"), "catalog tag namespace")?;
        if namespace == "not-applicable" {
            continue;
        }
        let prefix = namespace
            .strip_suffix("<semver>")
            .ok_or("catalog canonical tag namespace must end in <semver>")?;
        if let Some(version) = candidate.strip_prefix(prefix) {
            validate_strict_semver(version)?;
            let id = text(unit.get("id"), "catalog unit id")?;
            let observed = text(
                object(
                    required_value(unit, "versionSource")?,
                    "catalog version source",
                )?
                .get("observedDeclaration"),
                "catalog version source observed declaration",
            )?;
            if version != observed {
                return Err(format!(
                    "candidate tag version must match the checked-in {id} version source"
                ));
            }
            if matched_unit.replace(id).is_some() {
                return Err(
                    "candidate tag matches more than one canonical future tag namespace".to_owned(),
                );
            }
        }
    }
    if matched_unit.is_none() {
        return Err("candidate tag does not match a canonical future tag namespace".to_owned());
    }
    validate_history_repository(root)?;
    let history = read_json(&root.join("release/history/baseline-tags.json"))?;
    let historical_tags = array(history.get("tags"), "historical tag baseline tags")?;
    if historical_tags
        .iter()
        .any(|tag| tag.get("name").and_then(Value::as_str) == Some(candidate))
    {
        return Err("candidate tag collides with the ratified Historical Tag baseline".to_owned());
    }
    Ok(())
}

fn expected_catalog_source_roots(root: &Path) -> Result<BTreeSet<String>, String> {
    let workspace = parse_toml(
        &fs::read_to_string(root.join("Cargo.toml")).map_err(|error| {
            format!(
                "read workspace manifest {}: {error}",
                root.join("Cargo.toml").display()
            )
        })?,
    )?;
    let workspace = workspace
        .get("workspace")
        .and_then(TomlValue::as_table)
        .ok_or("workspace manifest is missing [workspace]")?;
    let mut roots = BTreeSet::new();
    for field in ["members", "exclude"] {
        let entries = workspace
            .get(field)
            .and_then(TomlValue::as_array)
            .ok_or_else(|| format!("workspace manifest is missing workspace.{field}"))?;
        for entry in entries {
            let entry = entry
                .as_str()
                .ok_or_else(|| format!("workspace.{field} must contain strings"))?;
            if !root.join(entry).join("Cargo.toml").is_file() {
                return Err(format!(
                    "workspace.{field} source root is missing Cargo.toml: {entry}"
                ));
            }
            roots.insert(entry.to_owned());
        }
    }
    for entry in fs::read_dir(root.join("packages"))
        .map_err(|error| format!("read packages directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("read packages directory entry: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("read package entry type: {error}"))?
            .is_dir()
        {
            continue;
        }
        let path = entry.path();
        if ["package.json", "pyproject.toml", "go.mod"]
            .iter()
            .any(|manifest| path.join(manifest).is_file())
        {
            roots.insert(format!("packages/{}", entry.file_name().to_string_lossy()));
        }
    }
    let validator = root.join("release/validator/Cargo.toml");
    if !validator.is_file() {
        return Err("release validator Cargo manifest is missing".to_owned());
    }
    roots.insert("release/validator".to_owned());
    Ok(roots)
}

fn validate_catalog_kind(root: &Path, unit: &Map<String, Value>) -> Result<(), String> {
    let source_root = text(unit.get("sourceRoot"), "release catalog source root")?;
    let expected = expected_catalog_kind(root, source_root)?;
    let actual = text(unit.get("kind"), "release catalog unit kind")?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "catalog unit kind {actual} does not match source root {source_root} ({expected})"
        ))
    }
}

fn expected_catalog_kind(root: &Path, source_root: &str) -> Result<&'static str, String> {
    if source_root == "release/validator" {
        return Ok("non-publishable-package");
    }
    let workspace = parse_toml(
        &fs::read_to_string(root.join("Cargo.toml")).map_err(|error| {
            format!(
                "read workspace manifest {}: {error}",
                root.join("Cargo.toml").display()
            )
        })?,
    )?;
    let workspace = workspace
        .get("workspace")
        .and_then(TomlValue::as_table)
        .ok_or("workspace manifest is missing [workspace]")?;
    if toml_array_contains(workspace, "members", source_root)? {
        return Ok("rust-package");
    }
    if toml_array_contains(workspace, "exclude", source_root)? {
        return Ok("non-publishable-package");
    }
    let package_root = root.join(source_root);
    if package_root.join("package.json").is_file() {
        Ok("typescript-runtime")
    } else if package_root.join("pyproject.toml").is_file() {
        Ok("python-runtime")
    } else if package_root.join("go.mod").is_file() {
        Ok("go-module")
    } else {
        Err(format!(
            "catalog source root has no recognized source declaration: {source_root}"
        ))
    }
}

fn toml_array_contains(table: &TomlTable, field: &str, expected: &str) -> Result<bool, String> {
    let values = table
        .get(field)
        .and_then(TomlValue::as_array)
        .ok_or_else(|| format!("workspace manifest is missing workspace.{field}"))?;
    Ok(values.iter().any(|value| value.as_str() == Some(expected)))
}

fn validate_catalog_version_source(
    root: &Path,
    id: &str,
    unit: &Map<String, Value>,
) -> Result<(), String> {
    let version = object(
        required_value(unit, "versionSource")?,
        "release catalog version source",
    )?;
    let path = text(version.get("path"), "release catalog version source path")?;
    let observed = version.get("observedDeclaration").unwrap_or(&Value::Null);
    let format = text(
        version.get("format"),
        "release catalog version source format",
    )?;
    let source_root = text(unit.get("sourceRoot"), "release catalog source root")?;
    require_catalog_path_within_source_root(source_root, path, "version source")?;
    if id == "vexil-runtime-go" {
        let observed = observed
            .as_str()
            .ok_or("Go catalog version declaration must be a string")?;
        if format != "go-version-file" {
            return Err(
                "Go runtime version source must use the checked-in VERSION file".to_owned(),
            );
        }
        let content = fs::read_to_string(root.join(path))
            .map_err(|error| format!("read catalog version source {path}: {error}"))?;
        let declaration = strict_go_version_declaration(&content)?;
        if declaration != observed {
            return Err(format!(
                "catalog observed declaration is stale or absent: {path}"
            ));
        }
        validate_go_version_decision(root, observed)?;
        return Ok(());
    }
    let observed = observed
        .as_str()
        .ok_or("catalog version declaration must be a string outside the Go blocker")?;
    let content = fs::read_to_string(root.join(path))
        .map_err(|error| format!("read catalog version source {path}: {error}"))?;
    let declaration = version_declaration_from_source(&content, format)?;
    if declaration != observed {
        return Err(format!(
            "catalog observed declaration is stale or absent: {path}"
        ));
    }
    if id == "vexil-runtime-ts" {
        let lockfile = fs::read_to_string(root.join("packages/runtime-ts/package-lock.json"))
            .map_err(|error| format!("read TypeScript package lockfile: {error}"))?;
        validate_typescript_lockfile_agreement(&lockfile, observed)?;
    }
    if id == "vexilc" {
        let main = fs::read_to_string(root.join("crates/vexilc/src/main.rs"))
            .map_err(|error| format!("read vexilc version display source: {error}"))?;
        validate_vexilc_version_display(&main, observed)?;
    }
    Ok(())
}

fn validate_catalog_changelog(root: &Path, unit: &Map<String, Value>) -> Result<(), String> {
    let changelog = object(
        required_value(unit, "changelog")?,
        "release catalog changelog",
    )?;
    let source_root = text(unit.get("sourceRoot"), "release catalog source root")?;
    let unit_kind = text(unit.get("kind"), "release catalog unit kind")?;
    let conventional_changelog = root.join(source_root).join("CHANGELOG.md");
    match text(changelog.get("status"), "release catalog changelog status")? {
        "present" => {
            if unit_kind == "non-publishable-package" {
                return Err(
                    "non-publishable catalog units must mark changelog not-applicable".to_owned(),
                );
            }
            let path = changelog
                .get("path")
                .and_then(Value::as_str)
                .ok_or("present catalog changelog requires a path")?;
            require_catalog_path_within_source_root(source_root, path, "changelog")?;
            if !root.join(path).is_file() {
                return Err(format!("catalog changelog is missing: {path}"));
            }
            if Path::new(path).file_name().and_then(|name| name.to_str()) != Some("CHANGELOG.md") {
                return Err("catalog changelog must name the unit CHANGELOG.md file".to_owned());
            }
        }
        "absent" if changelog.get("path") == Some(&Value::Null) => {
            if unit_kind == "non-publishable-package" {
                return Err(
                    "non-publishable catalog units must mark changelog not-applicable".to_owned(),
                );
            }
            if conventional_changelog.is_file() {
                return Err(format!(
                    "catalog changelog is stale: {} exists",
                    conventional_changelog.display()
                ));
            }
        }
        "not-applicable" if changelog.get("path") == Some(&Value::Null) => {
            if unit_kind != "non-publishable-package" {
                return Err(
                    "publishable catalog units must state changelog present or absent".to_owned(),
                );
            }
        }
        _ => return Err("catalog changelog status and path must agree".to_owned()),
    }
    Ok(())
}

fn validate_catalog_owner(
    root: &Path,
    source_root: &str,
    unit: &Map<String, Value>,
) -> Result<(), String> {
    let owner = object(required_value(unit, "owner")?, "catalog owner")?;
    let role_id = text(owner.get("roleId"), "catalog owner role")?;
    let assignment_id = text(owner.get("assignmentId"), "catalog owner assignment")?;
    let assignments = read_json(&root.join("release/stewardship/assignments.json"))?;
    let assignments = array(assignments.get("assignments"), "stewardship assignments")?;
    let assignment = assignments
        .iter()
        .find(|assignment| {
            assignment.get("assignmentId").and_then(Value::as_str) == Some(assignment_id)
        })
        .ok_or_else(|| format!("catalog owner assignment is unknown: {assignment_id}"))?;
    let assignment = object(assignment, "catalog owner assignment")?;
    if text(assignment.get("roleId"), "assignment role")? != role_id
        || text(assignment.get("status"), "assignment status")? != "active"
    {
        return Err(format!(
            "catalog owner assignment does not bind active role {role_id}"
        ));
    }
    let scope = object(required_value(assignment, "scope")?, "catalog owner scope")?;
    let scope_root = text(scope.get("root"), "catalog owner scope root")?;
    match role_id {
        "package-steward"
            if scope.get("kind").and_then(Value::as_str) == Some("maintained-root")
                && scope_root == source_root =>
        {
            Ok(())
        }
        "repository-administrator"
            if scope.get("kind").and_then(Value::as_str) == Some("repository")
                && scope_root == "." =>
        {
            Ok(())
        }
        _ => Err(format!(
            "catalog owner assignment {assignment_id} does not cover source root {source_root}"
        )),
    }
}

fn validate_catalog_publication(unit: &Map<String, Value>) -> Result<(), String> {
    let publication = object(required_value(unit, "publication")?, "catalog publication")?;
    let classification = text(publication.get("classification"), "catalog classification")?;
    let category = text(publication.get("targetCategory"), "catalog target category")?;
    let status = text(publication.get("status"), "catalog publication status")?;
    let expected = match classification {
        "publishable-source-unit" => ("future-registry-target", "source-inventory-only"),
        "candidate-unreleased" => ("future-registry-target", "candidate-unreleased"),
        "blocked-version-source" => ("source-only-module", "blocked-missing-version-source"),
        "non-publishable" => ("non-release", "non-publishable"),
        _ => return Err("catalog publication classification is invalid".to_owned()),
    };
    if (category, status) != expected {
        return Err(format!(
            "catalog publication classification {classification} requires {} / {}",
            expected.0, expected.1
        ));
    }
    Ok(())
}

fn validate_catalog_targets(root: &Path, unit: &Map<String, Value>) -> Result<(), String> {
    let source_root = text(unit.get("sourceRoot"), "release catalog source root")?;
    let version = object(
        required_value(unit, "versionSource")?,
        "catalog version source",
    )?;
    let version_path = text(version.get("path"), "catalog version source path")?;
    let format = text(version.get("format"), "catalog version source format")?;
    let publication = object(required_value(unit, "publication")?, "catalog publication")?;
    let classification = text(publication.get("classification"), "catalog classification")?;
    let content = if format == "required-file-absent" {
        None
    } else {
        Some(
            fs::read_to_string(root.join(version_path))
                .map_err(|error| format!("read catalog target manifest {version_path}: {error}"))?,
        )
    };
    for target in array(unit.get("targets"), "release catalog targets")? {
        let target = object(target, "release catalog target")?;
        let kind = text(target.get("kind"), "catalog target kind")?;
        let name = text(target.get("name"), "catalog target name")?;
        let matches_source = match kind {
            "cargo-package" | "internal-tool" | "example" => {
                if format != "cargo-package-version" {
                    return Err(format!(
                        "catalog target {kind} must use a Cargo package manifest"
                    ));
                }
                name == cargo_package_name(content.as_deref().unwrap())?
            }
            "cargo-binary" => {
                if format != "cargo-package-version" {
                    return Err(
                        "catalog cargo-binary target must use a Cargo package manifest".to_owned(),
                    );
                }
                cargo_binary_names(content.as_deref().unwrap(), &root.join(source_root))?
                    .contains(name)
            }
            "npm-package" => {
                if format != "package-json-version" {
                    return Err("catalog npm target must use package.json".to_owned());
                }
                name == json_manifest_field(content.as_deref().unwrap(), "name")?
            }
            "python-project" => {
                if format != "pyproject-project-version" {
                    return Err("catalog Python target must use pyproject.toml".to_owned());
                }
                name == toml_string_in_section(content.as_deref().unwrap(), "project", "name")?
            }
            "go-module" => {
                if format != "go-version-file" || classification != "publishable-source-unit" {
                    return Err(
                        "catalog Go target must use the checked-in VERSION source".to_owned()
                    );
                }
                let go_mod = root.join(source_root).join("go.mod");
                let go_mod = fs::read_to_string(&go_mod).map_err(|error| {
                    format!("read Go module manifest {}: {error}", go_mod.display())
                })?;
                name == go_module_name(&go_mod)?
            }
            _ => return Err(format!("catalog target kind is unsupported: {kind}")),
        };
        if !matches_source {
            return Err(format!(
                "catalog target name {name} does not match its source declaration"
            ));
        }
    }
    Ok(())
}

fn require_catalog_path_within_source_root(
    source_root: &str,
    path: &str,
    label: &str,
) -> Result<(), String> {
    let path = Path::new(path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || !path.starts_with(Path::new(source_root))
        || path == Path::new(source_root)
    {
        return Err(format!(
            "catalog {label} path must remain within source root {source_root}"
        ));
    }
    Ok(())
}

fn version_declaration_from_source(content: &str, format: &str) -> Result<String, String> {
    match format {
        "cargo-package-version" => toml_string_in_section(content, "package", "version"),
        "pyproject-project-version" => toml_string_in_section(content, "project", "version"),
        "package-json-version" => json_manifest_field(content, "version"),
        "go-version-file" => strict_go_version_declaration(content),
        _ => Err("catalog version source format is invalid for a present source".to_owned()),
    }
}

fn validate_go_version_decision(root: &Path, selected_version: &str) -> Result<(), String> {
    let decision = read_json(&root.join("release/decisions/runtime-go-version-2026-07-23.json"))?;
    let decision = object(&decision, "Go runtime version decision")?;
    require_string(
        decision,
        "$id",
        "https://vexil.dev/release/decisions/runtime-go-version-2026-07-23.json",
    )?;
    require_string(decision, "recordKind", "package-maintenance-decision")?;
    require_string(decision, "decisionId", "runtime-go-version-2026-07-23")?;
    require_string(decision, "status", "approved")?;
    require_string(decision, "unitId", "vexil-runtime-go")?;
    require_string(decision, "versionSource", "packages/runtime-go/VERSION")?;
    require_string(
        decision,
        "canonicalTagNamespace",
        "packages/runtime-go/v<semver>",
    )?;
    if text(
        decision.get("selectedVersion"),
        "Go decision selected version",
    )? != selected_version
    {
        return Err(
            "Go VERSION must agree with the approved public maintenance decision".to_owned(),
        );
    }
    validate_strict_semver(selected_version)?;
    let approval = object(
        required_value(decision, "approval")?,
        "Go version decision approval",
    )?;
    require_string(approval, "actorId", "github:furkanmamuk")?;
    if text(approval.get("approvedAt"), "Go decision approval timestamp")?.is_empty()
        || text(approval.get("decision"), "Go decision approval text")?.is_empty()
    {
        return Err(
            "Go version decision approval must retain timestamp and decision text".to_owned(),
        );
    }
    Ok(())
}

pub fn strict_go_version_declaration(content: &str) -> Result<String, String> {
    if content.contains('\r')
        || !content.ends_with('\n')
        || content[..content.len() - 1].contains('\n')
    {
        return Err("Go VERSION must contain exactly one SemVer token followed by LF".to_owned());
    }
    let version = &content[..content.len() - 1];
    validate_strict_semver(version)?;
    Ok(version.to_owned())
}

fn validate_strict_semver(value: &str) -> Result<(), String> {
    let (core_and_pre, build) = match value.split_once('+') {
        Some((left, right)) if !right.contains('+') => (left, Some(right)),
        Some(_) => return Err("version must be strict SemVer".to_owned()),
        None => (value, None),
    };
    let (core, pre) = match core_and_pre.split_once('-') {
        Some((left, right)) if !right.contains('-') || !right.is_empty() => (left, Some(right)),
        Some(_) => return Err("version must be strict SemVer".to_owned()),
        None => (core_and_pre, None),
    };
    let numeric = |component: &str| {
        !component.is_empty()
            && component.bytes().all(|byte| byte.is_ascii_digit())
            && (component == "0" || !component.starts_with('0'))
    };
    if core.split('.').count() != 3 || !core.split('.').all(numeric) {
        return Err("version must be strict SemVer".to_owned());
    }
    let identifier = |component: &str, forbid_numeric_leading_zero: bool| {
        !component.is_empty()
            && component
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && (!forbid_numeric_leading_zero
                || !component.bytes().all(|byte| byte.is_ascii_digit())
                || component == "0"
                || !component.starts_with('0'))
    };
    if pre.is_some_and(|suffix| !suffix.split('.').all(|part| identifier(part, true)))
        || build.is_some_and(|suffix| !suffix.split('.').all(|part| identifier(part, false)))
    {
        return Err("version must be strict SemVer".to_owned());
    }
    Ok(())
}

fn cargo_package_name(content: &str) -> Result<String, String> {
    toml_string_in_section(content, "package", "name")
}

fn toml_string_in_section(content: &str, section: &str, field: &str) -> Result<String, String> {
    parse_toml(content)?
        .get(section)
        .and_then(TomlValue::as_table)
        .and_then(|table| table.get(field))
        .and_then(TomlValue::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("source manifest is missing string {section}.{field}"))
}

fn parse_toml(content: &str) -> Result<TomlTable, String> {
    content
        .parse::<TomlTable>()
        .map_err(|error| format!("parse TOML source declaration: {error}"))
}

fn cargo_binary_names(content: &str, source_root: &Path) -> Result<BTreeSet<String>, String> {
    let manifest = parse_toml(content)?;
    let mut names = BTreeSet::new();
    if source_root.join("src/main.rs").is_file() {
        names.insert(cargo_package_name(content)?);
    }
    if let Some(binaries) = manifest.get("bin").and_then(TomlValue::as_array) {
        for binary in binaries {
            let name = binary
                .as_table()
                .and_then(|binary| binary.get("name"))
                .and_then(TomlValue::as_str)
                .ok_or("Cargo [[bin]] target must declare a string name")?;
            names.insert(name.to_owned());
        }
    }
    Ok(names)
}

fn go_module_name(content: &str) -> Result<String, String> {
    for line in content.lines() {
        let mut fields = line.split_whitespace();
        if fields.next() == Some("module") {
            if let Some(module) = fields.next().filter(|module| !module.starts_with("//")) {
                return Ok(module.to_owned());
            }
            return Err("Go module declaration has no module path".to_owned());
        }
    }
    Err("Go module manifest has no module declaration".to_owned())
}

fn json_manifest_field(content: &str, field: &str) -> Result<String, String> {
    serde_json::from_str::<Value>(content)
        .map_err(|error| format!("parse package.json: {error}"))?
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("package.json is missing string {field}"))
}

pub fn validate_typescript_lockfile_agreement(
    lockfile_content: &str,
    package_version: &str,
) -> Result<(), String> {
    let lockfile: Value = serde_json::from_str(lockfile_content)
        .map_err(|error| format!("parse TypeScript package lockfile: {error}"))?;
    let root_package = lockfile
        .get("packages")
        .and_then(Value::as_object)
        .and_then(|packages| packages.get(""))
        .ok_or("TypeScript package-lock must include packages[\"\"] root package")?;
    if root_package.get("version").and_then(Value::as_str) != Some(package_version) {
        return Err("TypeScript package-lock root version must agree with package.json".to_owned());
    }
    Ok(())
}

pub fn validate_vexilc_version_display(source: &str, package_version: &str) -> Result<(), String> {
    let marker = "\"--version\" | \"-V\" =>";
    let (_, version_arm) = source
        .split_once(marker)
        .ok_or("vexilc must retain a --version command branch")?;
    let opening = version_arm
        .find('{')
        .ok_or("vexilc --version command branch must have a body")?;
    let mut depth = 0usize;
    let mut closing = None;
    for (offset, character) in version_arm[opening..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    closing = Some(opening + offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let closing = closing.ok_or("vexilc --version command branch is not closed")?;
    let body = &version_arm[opening + 1..closing];
    let normalized: String = body
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    if body.contains("//")
        || body.contains("/*")
        || normalized != "println!(\"vexilc{}\",env!(\"CARGO_PKG_VERSION\"));return;"
        || package_version.is_empty()
    {
        Err(
            "vexilc --version must display CARGO_PKG_VERSION from its Cargo package source"
                .to_owned(),
        )
    } else {
        Ok(())
    }
}

pub fn render_catalog_markdown(root: &Path, catalog: &Value) -> Result<String, String> {
    validate_catalog_schema(root, catalog)?;
    validate_catalog(root, catalog)?;
    let catalog = object(catalog, "release catalog")?;
    let mut markdown = String::from("# Release Unit Catalog\n\n> Generated view of [`release/catalog.json`](../../../../release/catalog.json). The JSON catalog is canonical; this Markdown is non-authoritative and parity-checked.\n\nThis is a source-led inventory and typed structural dependency graph, not a Release Manifest, publication assertion, provider-identity claim, Release Set decision, or version-selection decision. Formal specifications and documentation govern semantics; pinned code and tests establish the executable baseline. Historical tags, changelog headings, and registry observations remain evidence in [Release History](./history.md).\n\n## Units\n\n| Unit | Source root | Targets | Status | Source version observation | Canonical tag policy |\n|---|---|---|---|---|---|\n");
    for unit in array(catalog.get("units"), "release catalog units")? {
        let unit = object(unit, "release catalog unit")?;
        let version = object(
            required_value(unit, "versionSource")?,
            "catalog version source",
        )?;
        let targets = array(unit.get("targets"), "catalog targets")?
            .iter()
            .map(|target| {
                let target = object(target, "catalog target")?;
                Ok(format!(
                    "{} `{}`",
                    text(target.get("kind"), "target kind")?,
                    text(target.get("name"), "target name")?
                ))
            })
            .collect::<Result<Vec<_>, String>>()?
            .join("; ");
        let observed = version
            .get("observedDeclaration")
            .and_then(Value::as_str)
            .unwrap_or("none (required file absent)");
        let publication = object(required_value(unit, "publication")?, "catalog publication")?;
        markdown.push_str(&format!(
            "| `{}` | `{}` | {} | `{}` | `{}` in `{}` | `{}` |\n",
            text(unit.get("id"), "unit id")?,
            text(unit.get("sourceRoot"), "source root")?,
            targets,
            text(publication.get("status"), "publication status")?,
            observed,
            text(version.get("path"), "version path")?,
            match text(unit.get("canonicalTagNamespace"), "tag namespace")? {
                "not-applicable" => "not applicable (non-publishable)",
                namespace => namespace,
            }
        ));
    }
    let order = validate_and_derive_release_order(root, catalog)?;
    markdown.push_str("\n## Typed dependency graph\n\nEach edge is recorded on its dependent unit. `related-before-unit` means the related unit's version must be publicly resolvable before the declaring unit is published. `publish_before` edges cite their checked-in runtime manifest declaration; `compatibility` and `bundle` edges cite an approved public release-dependency-edge decision. The catalog stores edges in stable `edgeType`, related-unit, evidence-kind, path, and location order.\n\n| Dependency | Dependent | Type | Public source evidence |\n|---|---|---|---|\n");
    for unit in array(catalog.get("units"), "release catalog units")? {
        let unit = object(unit, "release catalog unit")?;
        let dependent = text(unit.get("id"), "release catalog unit id")?;
        for edge in array(unit.get("dependencyEdges"), "catalog dependency edges")? {
            let edge = object(edge, "catalog dependency edge")?;
            let evidence = object(
                required_value(edge, "sourceEvidence")?,
                "catalog dependency evidence",
            )?;
            markdown.push_str(&format!(
                "| `{}` | `{dependent}` | `{}` | `{}` `{}` |\n",
                text(edge.get("relatedUnitId"), "catalog related unit id")?,
                text(edge.get("edgeType"), "catalog dependency edge type")?,
                text(evidence.get("path"), "catalog dependency evidence path")?,
                text(
                    evidence.get("location"),
                    "catalog dependency evidence location"
                )?,
            ));
        }
    }
    markdown.push_str("\nOnly `publish_before` participates in structural ordering. `compatibility` requires an approved shared-evidence decision without imposing registry order; `bundle` requires an approved identity decision and never creates a second Release Unit.\n\n## Structural source order\n\nThe current all-unit structural order is derived from checked-in manifests and catalog edges only:\n\n");
    markdown.push_str(
        &order
            .iter()
            .map(|id| format!("`{id}`"))
            .collect::<Vec<_>>()
            .join(" → "),
    );
    markdown.push_str("\n\n## Boundary and validation\n\n`candidate-unreleased` means the Python source unit is planned work, not a PyPI availability claim. The Go module's checked-in `VERSION` source identifies only its source state; `go.mod` supplies the module target identity, not its version. `non-publishable` roots are deliberately cataloged so they cannot be silently mistaken for releases.\n\nA valid graph does not establish packageability, authorization, registry identity, publication eligibility, Release Set membership, Manifest approval, tags, or publication. Root project-wide `v<semver>` tags remain prohibited during recovery.\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nThe offline command validates source paths, runtime manifest declarations, typed graph agreement, deterministic structural order, unique unit identities, canonical tag policy, and byte-exact generated-view parity. It performs no provider query or release effect.\n");
    Ok(markdown)
}

fn validate_npm_publish_workflow(root: &Path) -> Result<(), String> {
    let workflow_path = root.join(".github/workflows/npm-publish.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("read {}: {error}", workflow_path.display()))?;
    validate_npm_publish_workflow_source(&workflow)
}

pub fn validate_npm_publish_workflow_source(workflow: &str) -> Result<(), String> {
    let lines: Vec<(usize, String)> = workflow
        .lines()
        .filter_map(|line| {
            let uncommented = line.split_once('#').map_or(line, |(before, _)| before);
            let trimmed = uncommented.trim_end();
            (!trimmed.trim().is_empty()).then(|| {
                (
                    trimmed.len() - trimmed.trim_start().len(),
                    trimmed.trim().to_owned(),
                )
            })
        })
        .collect();

    let mut tag_patterns = Vec::new();
    let mut in_on = false;
    let mut in_push = false;
    let mut in_tags = false;
    for (indent, value) in &lines {
        if *indent == 0 {
            in_on = value == "on:";
            in_push = false;
            in_tags = false;
            continue;
        }
        if !in_on {
            continue;
        }
        if *indent == 2 {
            in_push = value == "push:";
            in_tags = false;
            continue;
        }
        if in_push && *indent == 4 {
            in_tags = value == "tags:";
            continue;
        }
        if in_tags && *indent >= 6 {
            let pattern = value
                .strip_prefix("- ")
                .ok_or("npm workflow tags must be a YAML list")?
                .trim_matches('"')
                .to_owned();
            tag_patterns.push(pattern);
        }
    }
    if tag_patterns != ["vexil-runtime-ts-v*"] {
        return Err("npm publication-disabled workflow must use only the canonical TypeScript tag namespace".to_owned());
    }
    let material = lines
        .iter()
        .map(|(_, value)| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let lower_material = material.to_ascii_lowercase();
    if lower_material.contains("id-token: write")
        || lower_material.contains(": write")
        || material.contains("NPM_TOKEN")
        || material.contains("NODE_AUTH_TOKEN")
        || lower_material.contains("publish")
        || lower_material.contains("unpublish")
    {
        return Err("npm publication-disabled workflow must retain its no-credential, no-publication boundary".to_owned());
    }
    Ok(())
}

pub fn validate_catalog_documentation_parity(
    root: &Path,
    catalog: &Value,
    documentation: &str,
) -> Result<(), String> {
    if documentation != render_catalog_markdown(root, catalog)? {
        return Err(
            "documentation parity failure: docs/book/src/release/catalog.md is stale".to_owned(),
        );
    }
    Ok(())
}

/// Validates the append-only release-history surface offline.  It never contacts a
/// provider: callers that obtain a remote snapshot must do so through their own
/// explicit read-only collector and submit the resulting JSON for validation.
pub fn validate_history_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let history = root.join("release/history");
    let baseline = read_json(&history.join("baseline-tags.json"))?;
    validate_history_baseline_schema(root, &baseline)?;
    validate_history_baseline(&baseline)?;
    let ratifications = read_history_records(root, "ratifications")?;
    let baseline_object = object(&baseline, "history baseline")?;
    let baseline_status = text(baseline_object.get("status"), "history baseline status")?;
    if baseline_status == "ratified" {
        let digest = text(
            baseline_object.get("baselineDigest"),
            "history baseline digest",
        )?;
        let expected_ids: BTreeSet<&str> = array(
            baseline_object.get("ratificationIds"),
            "history ratification ids",
        )?
        .iter()
        .map(|value| text(Some(value), "history ratification id"))
        .collect::<Result<_, _>>()?;
        let mut roles = BTreeSet::new();
        let mut found_ids = BTreeSet::new();
        for ratification in &ratifications {
            validate_history_ratification_schema(root, ratification)?;
            let ratification = object(ratification, "history ratification")?;
            let id = text(
                ratification.get("ratificationId"),
                "history ratification id",
            )?;
            found_ids.insert(id);
            roles.insert(text(
                ratification.get("roleId"),
                "history ratification role",
            )?);
            if text(
                ratification.get("baselineDigest"),
                "ratification baseline digest",
            )? != digest
            {
                return Err(
                    "history ratification is not bound to the exact baseline digest".to_owned(),
                );
            }
            if text(
                ratification.get("observationScope"),
                "ratification observation scope",
            )? != history_baseline_observation_scope(baseline_object)?
            {
                return Err(
                    "history ratification is not bound to the baseline observation scope"
                        .to_owned(),
                );
            }
        }
        if roles != BTreeSet::from(["release-steward", "repository-administrator"])
            || found_ids != expected_ids
        {
            return Err("a ratified history baseline requires exactly the Release Steward and Repository Administrator assertions".to_owned());
        }
    } else if !ratifications.is_empty() {
        return Err(
            "unratified history baseline must not retain ratification assertions".to_owned(),
        );
    }

    let sources = read_json(&history.join("observation-sources.json"))?;
    validate_history_observation_sources_schema(root, &sources)?;
    let source_ids: BTreeSet<String> = array(
        object(&sources, "history observation source inventory")?.get("sources"),
        "history observation sources",
    )?
    .iter()
    .map(|source| {
        text(
            object(source, "history observation source")?.get("id"),
            "source id",
        )
        .map(str::to_owned)
    })
    .collect::<Result<_, _>>()?;

    let observations = read_history_records(root, "observations")?;
    let mut observation_ids = BTreeSet::new();
    let mut content_by_id = BTreeMap::new();
    for observation in &observations {
        validate_history_observation_schema(root, observation)?;
        let value = object(observation, "history observation")?;
        let observation_id = text(value.get("observationId"), "history observation id")?;
        if !observation_ids.insert(observation_id.to_owned()) {
            return Err("history observation identifiers must be unique".to_owned());
        }
        if !source_ids.contains(text(
            value.get("sourceId"),
            "history observation source id",
        )?) {
            return Err("history observation references an unknown source".to_owned());
        }
        let content_id = text(value.get("contentId"), "history observation content id")?;
        let identity = format!(
            "{}|{}|{}|{}",
            text(value.get("sourceId"), "history observation source id")?,
            text(value.get("query"), "history observation query")?,
            text(value.get("state"), "history observation state")?,
            value.get("claim").unwrap_or(&Value::Null)
        );
        if let Some(previous) = content_by_id.insert(content_id.to_owned(), identity.clone()) {
            if previous != identity {
                return Err(
                    "conflicting history observation content uses one immutable content id"
                        .to_owned(),
                );
            }
        }
    }

    let entries = read_history_records(root, "entries")?;
    let mut entry_ids = BTreeSet::new();
    for entry in &entries {
        validate_history_ledger_entry_schema(root, entry)?;
        let entry = object(entry, "history ledger entry")?;
        let entry_id = text(entry.get("entryId"), "history ledger entry id")?;
        if !entry_ids.insert(entry_id.to_owned()) {
            return Err("history ledger entry identifiers must be unique".to_owned());
        }
        for observation_id in array(entry.get("observationIds"), "ledger observation ids")? {
            if !observation_ids.contains(text(Some(observation_id), "ledger observation id")?) {
                return Err("history ledger entry references an unknown observation".to_owned());
            }
        }
        if entry.get("correctionOf").is_some()
            && text(entry.get("classification"), "ledger classification")? != "correction"
        {
            return Err("only a correction entry may reference a prior entry".to_owned());
        }
    }
    for entry in &entries {
        let entry = object(entry, "history ledger entry")?;
        if let Some(previous) = entry.get("correctionOf") {
            let previous = text(Some(previous), "corrected entry id")?;
            if !entry_ids.contains(previous) {
                return Err("history correction references an unknown prior entry".to_owned());
            }
        }
    }
    let rendered = render_history_ledger(&entries, &observations)?;
    let ledger = fs::read_to_string(history.join("ledger.md"))
        .map_err(|error| format!("read history ledger: {error}"))?;
    if ledger != rendered {
        return Err(
            "release/history/ledger.md differs from deterministic canonical history rendering"
                .to_owned(),
        );
    }

    let policy = read_json(&history.join("additive-repair-policy.json"))?;
    validate_additive_repair_policy(&policy)?;
    for proposal in read_history_records(root, "repair-proposals")? {
        validate_additive_repair_proposal_schema(root, &proposal)?;
        validate_additive_repair_preflight(&baseline, &policy, &proposal)?;
    }
    let reconciliation = read_json(&history.join("reconciliation-decision.json"))?;
    validate_history_reconciliation_decision_schema(root, &reconciliation)?;
    validate_reconciliation_decision(&reconciliation, &entries)?;
    Ok(())
}

pub fn validate_history_baseline(baseline: &Value) -> Result<(), String> {
    let baseline = object(baseline, "history baseline")?;
    let status = text(baseline.get("status"), "history baseline status")?;
    let tags = array(baseline.get("tags"), "history baseline tags")?;
    let mut names = BTreeSet::new();
    for tag in tags {
        let tag = object(tag, "history tag")?;
        let name = text(tag.get("name"), "history tag name")?;
        if !names.insert(name.to_owned()) {
            return Err("history baseline has duplicate tag names".to_owned());
        }
        for key in ["refTarget", "peeledCommit"] {
            if !is_full_object_id(text(tag.get(key), "history tag object id")?) {
                return Err(format!(
                    "history tag {name} has an abbreviated or invalid {key}"
                ));
            }
        }
        let kind = text(tag.get("kind"), "history tag kind")?;
        match (kind, tag.get("annotatedTag")) {
            ("annotated", Some(Value::String(value))) if is_full_object_id(value) => {}
            ("annotated", _) => {
                return Err(format!(
                    "annotated history tag {name} lacks its full tag object id"
                ))
            }
            ("lightweight", None) => {}
            ("lightweight", _) => {
                return Err(format!(
                    "lightweight history tag {name} must not invent an annotated tag object"
                ))
            }
            _ => return Err("history tag kind is not recognized".to_owned()),
        }
    }
    let ratifications = array(baseline.get("ratificationIds"), "history ratifications")?;
    let actual_digest = history_baseline_digest(baseline)?;
    match status {
        "awaiting-read-only-collection" => {
            if !tags.is_empty()
                || baseline.get("baselineDigest") != Some(&Value::Null)
                || !ratifications.is_empty()
            {
                return Err("an uncollected history baseline must not claim tags, a digest, or ratification".to_owned());
            }
        }
        "awaiting-ratification" => {
            if tags.is_empty()
                || !is_sha256(
                    baseline
                        .get("baselineDigest")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
                || !ratifications.is_empty()
            {
                return Err("a collected history baseline requires tags and a digest but no implied ratification".to_owned());
            }
            if baseline.get("baselineDigest").and_then(Value::as_str) != Some(&actual_digest) {
                return Err(
                    "history baseline digest does not bind the collected remote tag identities"
                        .to_owned(),
                );
            }
        }
        "ratified" => {
            if tags.is_empty()
                || !is_sha256(
                    baseline
                        .get("baselineDigest")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
                || ratifications.len() != 2
            {
                return Err(
                    "a ratified history baseline requires tags, digest, and two role assertions"
                        .to_owned(),
                );
            }
            if baseline.get("baselineDigest").and_then(Value::as_str) != Some(&actual_digest) {
                return Err(
                    "history baseline digest does not bind the collected remote tag identities"
                        .to_owned(),
                );
            }
        }
        _ => return Err("history baseline status is not recognized".to_owned()),
    }
    Ok(())
}

/// Returns the SHA-256 identity bound by historical-tag ratifications.
/// Approval fields and state are excluded so changing an assertion cannot change
/// the immutable remote observation it approves.
pub fn history_baseline_digest(baseline: &Map<String, Value>) -> Result<String, String> {
    let canonical = serde_json::json!({
        "$id": text(baseline.get("$id"), "history baseline id")?,
        "version": text(baseline.get("version"), "history baseline version")?,
        "recordKind": text(baseline.get("recordKind"), "history baseline record kind")?,
        "remote": baseline.get("remote").cloned().ok_or_else(|| "history baseline lacks remote".to_owned())?,
        "observedAt": baseline.get("observedAt").cloned().ok_or_else(|| "history baseline lacks observation time".to_owned())?,
        "tags": baseline.get("tags").cloned().ok_or_else(|| "history baseline lacks tags".to_owned())?,
    });
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("serialize canonical history baseline: {error}"))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

/// Returns the complete read-only observation scope approved by a ratification.
pub fn history_baseline_observation_scope(baseline: &Map<String, Value>) -> Result<String, String> {
    let remote = object(
        baseline
            .get("remote")
            .ok_or_else(|| "history baseline lacks remote".to_owned())?,
        "history baseline remote",
    )?;
    Ok(format!(
        "{} | {} | {}",
        text(remote.get("url"), "history remote URL")?,
        text(remote.get("query"), "history remote query")?,
        text(
            baseline.get("observedAt"),
            "history baseline observation time"
        )?
    ))
}

pub fn validate_history_tag_snapshot(baseline: &Value, observed: &Value) -> Result<(), String> {
    validate_history_baseline(baseline)?;
    let baseline = object(baseline, "history baseline")?;
    if text(baseline.get("status"), "history baseline status")? != "ratified" {
        return Err(
            "Historical Tag invariant is blocked until the baseline is ratified".to_owned(),
        );
    }
    let observed = object(observed, "observed history baseline")?;
    let mut observed_tags = BTreeMap::new();
    for tag in array(observed.get("tags"), "observed history tags")? {
        let tag = object(tag, "observed history tag")?;
        observed_tags.insert(text(tag.get("name"), "observed history tag name")?, tag);
    }
    for expected in array(baseline.get("tags"), "baseline history tags")? {
        let expected = object(expected, "baseline history tag")?;
        let name = text(expected.get("name"), "baseline history tag name")?;
        let Some(actual) = observed_tags.get(name) else {
            return Err(format!(
                "Historical Tag drift: {name} is absent; no repair is permitted"
            ));
        };
        for field in ["kind", "refTarget", "annotatedTag", "peeledCommit"] {
            if expected.get(field) != actual.get(field) {
                return Err(format!(
                    "Historical Tag drift: {name} {field} differs; no repair is permitted"
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_additive_repair_preflight(
    baseline: &Value,
    policy: &Value,
    proposal: &Value,
) -> Result<(), String> {
    let policy = object(policy, "additive repair policy")?;
    let allowed: BTreeSet<&str> = array(policy.get("allowedActions"), "allowed repair actions")?
        .iter()
        .map(|value| text(Some(value), "allowed repair action"))
        .collect::<Result<_, _>>()?;
    let prohibited: BTreeSet<&str> =
        array(policy.get("prohibitedActions"), "prohibited repair actions")?
            .iter()
            .map(|value| text(Some(value), "prohibited repair action"))
            .collect::<Result<_, _>>()?;
    let proposal = object(proposal, "additive repair proposal")?;
    for action in array(proposal.get("proposedActions"), "proposed repair actions")? {
        let action = text(Some(action), "proposed repair action")?;
        if prohibited.contains(action) || !allowed.contains(action) {
            return Err(format!(
                "additive repair preflight rejects {action} before any remote operation"
            ));
        }
    }
    if text(proposal.get("status"), "repair proposal status")? == "approved"
        && proposal.get("approval") == Some(&Value::Null)
    {
        return Err("an approved additive repair proposal requires recorded approval".to_owned());
    }
    validate_history_baseline(baseline)
}

/// Parses only the output of `git ls-remote --tags <remote>` into an unratified
/// collector result. The result is deliberately not a baseline record: a reviewer
/// must bind a digest and the two required role assertions before it can become
/// canonical history.
pub fn parse_history_tag_collection(
    remote: &str,
    stdout: &str,
    observed_at: &str,
) -> Result<Value, String> {
    let mut refs: BTreeMap<String, String> = BTreeMap::new();
    let mut peeled: BTreeMap<String, String> = BTreeMap::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let (object_id, reference) = line.split_once('\t').ok_or_else(|| {
            "read-only tag collector received malformed ls-remote output".to_owned()
        })?;
        if !is_full_object_id(object_id) || !reference.starts_with("refs/tags/") {
            return Err("read-only tag collector requires full tag object ids".to_owned());
        }
        let name = &reference["refs/tags/".len()..];
        if let Some(name) = name.strip_suffix("^{}") {
            if peeled
                .insert(name.to_owned(), object_id.to_owned())
                .is_some()
            {
                return Err(
                    "read-only tag collector received duplicate peeled identities".to_owned(),
                );
            }
        } else if refs.insert(name.to_owned(), object_id.to_owned()).is_some() {
            return Err("read-only tag collector received duplicate tag names".to_owned());
        }
    }
    let mut tags = Vec::new();
    for (name, ref_target) in refs {
        if let Some(peeled_commit) = peeled.remove(&name) {
            tags.push(serde_json::json!({"name": name, "kind": "annotated", "refTarget": ref_target, "annotatedTag": ref_target, "peeledCommit": peeled_commit}));
        } else {
            tags.push(serde_json::json!({"name": name, "kind": "lightweight", "refTarget": ref_target, "peeledCommit": ref_target}));
        }
    }
    if !peeled.is_empty() {
        return Err(
            "read-only tag collector received a peeled identity without its tag ref".to_owned(),
        );
    }
    Ok(serde_json::json!({
        "recordKind": "historical-tag-collection",
        "remote": remote,
        "query": "git ls-remote --tags",
        "observedAt": observed_at,
        "collectorVersion": env!("CARGO_PKG_VERSION"),
        "tags": tags,
        "nextStep": "Bind a SHA-256 digest and both required public role ratifications before creating a canonical baseline."
    }))
}

fn validate_additive_repair_policy(policy: &Value) -> Result<(), String> {
    let policy = object(policy, "additive repair policy")?;
    require_exact_keys(
        policy,
        &[
            "$schema",
            "$id",
            "version",
            "recordKind",
            "allowedActions",
            "prohibitedActions",
            "preflight",
        ],
        "additive repair policy",
    )?;
    require_string(
        policy,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
    )?;
    require_string(
        policy,
        "$id",
        "https://vexil.dev/release/history/additive-repair-policy.json",
    )?;
    require_string(policy, "recordKind", "additive-repair-policy")?;
    for action in [
        "move-tag",
        "delete-tag",
        "force-update-tag",
        "recreate-tag",
        "reuse-tag",
        "overwrite-artifact",
        "replace-artifact",
    ] {
        if !array(policy.get("prohibitedActions"), "prohibited repair actions")?
            .iter()
            .any(|value| value.as_str() == Some(action))
        {
            return Err(format!("additive repair policy must prohibit {action}"));
        }
    }
    Ok(())
}

fn validate_reconciliation_decision(decision: &Value, entries: &[Value]) -> Result<(), String> {
    let decision = object(decision, "history reconciliation decision")?;
    if text(decision.get("rootTagPolicy"), "root tag policy")? != "prohibited-during-recovery" {
        return Err(
            "history reconciliation must prohibit project-wide root v<semver> tags during recovery"
                .to_owned(),
        );
    }
    let status = text(decision.get("status"), "reconciliation status")?;
    let approval = decision.get("approval").unwrap_or(&Value::Null);
    if status == "approved" && approval.is_null() {
        return Err(
            "an approved reconciliation decision requires public approval evidence".to_owned(),
        );
    }
    if status != "approved" && !approval.is_null() {
        return Err(
            "an unapproved reconciliation decision must not carry approval evidence".to_owned(),
        );
    }
    let ledger_entries = array(
        decision.get("ledgerEntryIds"),
        "reconciliation ledger entries",
    )?;
    if status == "approved" && ledger_entries.is_empty() {
        return Err(
            "an approved reconciliation decision requires an accepted ledger entry".to_owned(),
        );
    }
    for entry_id in ledger_entries {
        let entry_id = text(Some(entry_id), "reconciliation ledger entry id")?;
        let entry = entries
            .iter()
            .find(|entry| {
                object(entry, "history ledger entry")
                    .and_then(|entry| text(entry.get("entryId"), "history ledger entry id"))
                    .is_ok_and(|id| id == entry_id)
            })
            .ok_or_else(|| {
                "reconciliation decision references an unknown ledger entry".to_owned()
            })?;
        if status == "approved"
            && text(
                object(entry, "history ledger entry")?.get("reviewState"),
                "history ledger review state",
            )? != "accepted"
        {
            return Err("reconciliation decision references an unknown ledger entry".to_owned());
        }
    }
    Ok(())
}

fn render_history_ledger(entries: &[Value], observations: &[Value]) -> Result<String, String> {
    let mut rendered = String::from("# Release History Ledger\n\n> Generated from canonical JSON under `release/history/`. This Markdown is non-authoritative and must match the validator's deterministic rendering.\n\n## Review state\n\n");
    if entries.is_empty() {
        rendered
            .push_str("No accepted, rejected, or unresolved ledger entries have been recorded.\n");
    } else {
        let mut rows: Vec<_> = entries
            .iter()
            .map(|entry| object(entry, "history ledger entry"))
            .collect::<Result<_, _>>()?;
        rows.sort_by_key(|entry| {
            text(entry.get("entryId"), "history ledger entry id")
                .unwrap_or_default()
                .to_owned()
        });
        for entry in rows {
            rendered.push_str(&format!(
                "- `{}` — {} (`{}`)\n",
                text(entry.get("entryId"), "history ledger entry id")?,
                text(entry.get("classification"), "history ledger classification")?,
                text(entry.get("reviewState"), "history ledger review state")?
            ));
        }
    }
    rendered.push_str("\n## Observations\n\n");
    if observations.is_empty() {
        rendered.push_str("No immutable source observations have been collected. The baseline remains awaiting read-only remote collection and digest-bound dual-role ratification.\n");
    } else {
        let mut rows: Vec<_> = observations
            .iter()
            .map(|observation| object(observation, "history observation"))
            .collect::<Result<_, _>>()?;
        rows.sort_by_key(|row| {
            text(row.get("observationId"), "history observation id")
                .unwrap_or_default()
                .to_owned()
        });
        for row in rows {
            rendered.push_str(&format!(
                "- `{}` — {} from `{}` (`{}`)\n",
                text(row.get("observationId"), "history observation id")?,
                text(row.get("state"), "history observation state")?,
                text(row.get("sourceId"), "history observation source")?,
                text(row.get("contentId"), "history observation content id")?
            ));
        }
    }
    Ok(rendered)
}

fn read_history_records(root: &Path, directory: &str) -> Result<Vec<Value>, String> {
    let path = root.join("release/history").join(directory);
    let mut records = Vec::new();
    for entry in fs::read_dir(&path).map_err(|error| format!("read {}: {error}", path.display()))? {
        let path = entry
            .map_err(|error| format!("read {} entry: {error}", path.display()))?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            records.push(read_json(&path)?);
        }
    }
    Ok(records)
}

fn is_full_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// Validates the public, offline external-control evidence package. It deliberately
/// validates records and committed workflow source only: neither source is
/// evidence that a provider-side setting has been remediated.
pub fn validate_external_controls_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let expected = read_json(&root.join("release/controls/expected-controls.json"))?;
    let baseline = read_json(&root.join("release/controls/observations/baseline-2026-07-13.json"))?;
    let remediation =
        read_json(&root.join("release/controls/remediation-plan-github-protections.json"))?;
    let custody = read_json(&root.join("release/identities/custody.json"))?;
    let exercise_plan = read_json(&root.join("release/exercises/revocation-exercise-plan.json"))?;
    let exercise_result =
        read_json(&root.join("release/exercises/revocation-exercise-result.json"))?;

    for (label, record) in [
        ("expected external controls", &expected),
        ("external-control baseline", &baseline),
        ("external-control remediation plan", &remediation),
        ("identity custody inventory", &custody),
        ("revocation exercise plan", &exercise_plan),
        ("revocation exercise result", &exercise_result),
    ] {
        ensure_no_private_leakage(&record.to_string())?;
        let object = object(record, label)?;
        if !object.contains_key("$schema")
            || !object.contains_key("$id")
            || !object.contains_key("version")
        {
            return Err(format!(
                "{label} must have public schema, identifier, and version fields"
            ));
        }
        let id = text(object.get("$id"), "public record id")?;
        if !id.starts_with("https://vexil.dev/release/") {
            return Err(format!("{label} must use a public vexil.dev identifier"));
        }
    }

    validate_external_control_schema(root, &expected)?;
    let expected_controls = expected_control_index(&expected)?;
    validate_external_observation_schema(root, &baseline)?;
    validate_observation_inventory(root, &expected_controls)?;
    validate_external_remediation_schema(root, &remediation)?;
    validate_identity_custody_schema(root, &custody)?;
    validate_revocation_exercise_schema(root, &exercise_plan)?;
    validate_revocation_exercise_schema(root, &exercise_result)?;
    validate_revocation_exercise_pair(&exercise_plan, &exercise_result)?;

    let expected_ids: BTreeSet<_> = expected_controls.keys().cloned().collect();
    let baseline_root = object(&baseline, "external-control baseline")?;
    let baseline_rows = array(baseline_root.get("results"), "baseline observation results")?;
    let mut baseline_ids = BTreeSet::new();
    for row in baseline_rows {
        let row = object(row, "baseline observation result")?;
        let id = text(row.get("assertionId"), "baseline assertion id")?.to_owned();
        if !baseline_ids.insert(id) {
            return Err(
                "baseline observation cannot contain conflicting assertion identities".to_owned(),
            );
        }
        if text(row.get("status"), "baseline result status")? == "compliant" {
            return Err(
                "the known recovery baseline cannot claim compliant provider controls".to_owned(),
            );
        }
    }
    if baseline_ids != expected_ids {
        return Err(
            "baseline observation must cover every expected control exactly once".to_owned(),
        );
    }
    let remediation_root = object(&remediation, "external-control remediation plan")?;
    let remediation_baseline = object(
        required_value(remediation_root, "baselineObservation")?,
        "remediation baseline observation",
    )?;
    let stable_identity = object(
        required_value(baseline_root, "stableIdentity")?,
        "baseline stable identity",
    )?;
    if text(
        remediation_baseline.get("normalizedStateDigest"),
        "remediation baseline digest",
    )? != text(
        stable_identity.get("normalizedStateDigest"),
        "baseline digest",
    )? {
        return Err("remediation plan must bind to the retained baseline stable digest".to_owned());
    }

    let expected_text = expected.to_string();
    for required in [
        "branch",
        "tag",
        "release",
        "environment",
        "workflow",
        "trusted",
        "revocation",
    ] {
        if !expected_text.to_ascii_lowercase().contains(required) {
            return Err(format!(
                "expected external controls omit required {required} assertion"
            ));
        }
    }
    let baseline_text = baseline.to_string().to_ascii_lowercase();
    if !baseline_text.contains("2026-07-13")
        || !baseline_text.contains("noncompliant")
        || baseline_text.contains("compliant") && !baseline_text.contains("noncompliant")
    {
        return Err("the retained recovery baseline must remain dated and noncompliant".to_owned());
    }
    let remediation_text = remediation.to_string().to_ascii_lowercase();
    if !(remediation_text.contains("unexecuted") || remediation_text.contains("not-executed"))
        || !remediation_text.contains("repository administrator")
        || !remediation_text.contains("historical")
    {
        return Err("remediation plan must retain the administrator boundary, unexecuted state, and historical-identity exclusion".to_owned());
    }
    let custody_text = custody.to_string().to_ascii_lowercase();
    for required in ["pypi", "unresolved", "continuity", "blocked", "trusted"] {
        if !custody_text.contains(required) {
            return Err(format!(
                "identity custody inventory must retain {required} as a fail-closed state"
            ));
        }
    }
    validate_workflow_static_isolation(root)?;
    Ok(())
}

pub fn validate_revocation_exercise_pair(plan: &Value, result: &Value) -> Result<(), String> {
    let plan = object(plan, "revocation exercise plan")?;
    let result = object(result, "revocation exercise result")?;
    if text(plan.get("recordKind"), "revocation plan kind")? != "revocation-exercise-plan"
        || text(result.get("recordKind"), "revocation result kind")? != "revocation-exercise-result"
    {
        return Err(
            "revocation exercise records must retain distinct plan and result kinds".to_owned(),
        );
    }
    let plan_status = text(plan.get("status"), "revocation plan status")?;
    let result_status = text(result.get("status"), "revocation result status")?;
    if plan_status == "blocked-unexecuted" && result_status != "unexecuted" {
        return Err("a blocked revocation plan cannot claim an executed result".to_owned());
    }
    if result_status != "executed-success" {
        return Ok(());
    }
    if plan_status != "approved" {
        return Err("an executed revocation result requires an approved plan".to_owned());
    }
    if !text(result.get("scope"), "revocation result scope")?
        .to_ascii_lowercase()
        .contains("deploy key")
    {
        return Err("an executed result must name the disposable deploy-key scope".to_owned());
    }

    let mut event_ids = BTreeSet::new();
    let mut actions_by_slot: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut key_ids_by_slot: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
    for event in array(result.get("events"), "revocation exercise events")? {
        let event = object(event, "revocation exercise event")?;
        let event_id = text(event.get("eventId"), "revocation event id")?.to_owned();
        if !event_ids.insert(event_id) {
            return Err("revocation exercise event identifiers must be unique".to_owned());
        }
        let action = text(event.get("action"), "revocation event action")?;
        let method = text(
            event.get("providerMethod"),
            "revocation event provider method",
        )?;
        let expected_method = match action {
            "preflight-absence" | "metadata-absent" => "GET",
            "created" => "POST",
            "authenticated" | "denied" => "SSH",
            "revoked" => "DELETE",
            _ => return Err("revocation exercise event action is not recognized".to_owned()),
        };
        if method != expected_method {
            return Err(format!(
                "revocation event {action} must retain {expected_method} evidence"
            ));
        }
        let reference = text(event.get("providerReference"), "revocation event reference")?;
        let lower_reference = reference.to_ascii_lowercase();
        if lower_reference.contains("private key")
            || lower_reference.contains("token")
            || lower_reference.contains("-----begin")
        {
            return Err("revocation evidence must not contain credential material".to_owned());
        }
        ensure_no_private_leakage(reference)?;
        let slot = text(event.get("targetSlot"), "revocation event target slot")?.to_owned();
        actions_by_slot
            .entry(slot.clone())
            .or_default()
            .insert(action.to_owned());
        if matches!(action, "created" | "revoked" | "metadata-absent") {
            let key_id = event
                .get("keyId")
                .and_then(Value::as_i64)
                .filter(|key_id| *key_id > 0)
                .ok_or_else(|| {
                    format!("revocation event {action} must retain an immutable key id")
                })?;
            key_ids_by_slot.entry(slot).or_default().insert(key_id);
        }
        if matches!(action, "created" | "authenticated") && event.get("keyFingerprint").is_none() {
            return Err(format!(
                "revocation event {action} must retain only the public key fingerprint"
            ));
        }
    }
    let required_actions: BTreeSet<String> = [
        "preflight-absence",
        "created",
        "authenticated",
        "revoked",
        "denied",
        "metadata-absent",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    for slot in ["key-a", "key-b"] {
        if actions_by_slot.get(slot) != Some(&required_actions) {
            return Err(format!(
                "executed revocation evidence must prove every lifecycle step for {slot}"
            ));
        }
        if key_ids_by_slot.get(slot).map(BTreeSet::len) != Some(1) {
            return Err(format!(
                "executed revocation evidence must retain one immutable key id for {slot}"
            ));
        }
    }
    if key_ids_by_slot.get("key-a") == key_ids_by_slot.get("key-b") {
        return Err(
            "the replacement deploy key must not reuse the revoked key identity".to_owned(),
        );
    }
    let evidence = object(
        required_value(result, "evidence")?,
        "revocation result evidence",
    )?;
    let digest = text(
        evidence.get("stableEvidenceDigest"),
        "revocation result evidence digest",
    )?;
    if !digest.starts_with("sha256:") {
        return Err("executed revocation evidence must retain a stable digest".to_owned());
    }
    Ok(())
}

fn expected_control_index(record: &Value) -> Result<BTreeMap<String, ExpectedControl>, String> {
    let rows = array(
        object(record, "expected external controls")?.get("assertions"),
        "expected external-control assertions",
    )?;
    let mut controls = BTreeMap::new();
    for row in rows {
        let row = object(row, "expected external-control assertion")?;
        let id = text(row.get("id"), "expected control id")?.to_owned();
        let query = object(required_value(row, "query")?, "expected control query")?;
        let method = text(query.get("method"), "expected control query method")?.to_owned();
        if method != "GET" {
            return Err("expected external-control queries must remain GET-only".to_owned());
        }
        let control = ExpectedControl {
            provider: text(row.get("provider"), "expected control provider")?.to_owned(),
            scope: text(row.get("scope"), "expected control scope")?.to_owned(),
            method,
            path: text(query.get("path"), "expected control query path")?.to_owned(),
        };
        if controls.insert(id, control).is_some() {
            return Err(
                "expected external-control identifiers must be stable and unique".to_owned(),
            );
        }
    }
    Ok(controls)
}

pub fn expected_observation_query(
    root: &Path,
    assertion_id: &str,
) -> Result<(String, String), String> {
    let expected = read_json(&root.join("release/controls/expected-controls.json"))?;
    let controls = expected_control_index(&expected)?;
    let control = controls
        .get(assertion_id)
        .ok_or_else(|| format!("unknown external-control assertion: {assertion_id}"))?;
    if control.method != "GET" || control.path.contains('{') {
        return Err(format!(
            "assertion {assertion_id} is not a directly observable GET endpoint"
        ));
    }
    Ok((control.provider.clone(), control.path.clone()))
}

pub fn validate_current_observation_record(root: &Path, record: &Value) -> Result<(), String> {
    validate_external_observation_schema(root, record)?;
    let expected = read_json(&root.join("release/controls/expected-controls.json"))?;
    let controls = expected_control_index(&expected)?;
    let observation = object(record, "external-control observation")?;
    if text(
        observation.get("evidenceState"),
        "observation evidence state",
    )? != "current"
    {
        return Err("semantic observation validation requires current evidence".to_owned());
    }
    let provider = text(observation.get("provider"), "observation provider")?;
    let scope = text(observation.get("scope"), "observation scope")?;
    let mut assertions = BTreeSet::new();
    for result in array(observation.get("results"), "observation results")? {
        let result = object(result, "observation result")?;
        let assertion_id = text(result.get("assertionId"), "observation assertion id")?;
        let expected = controls
            .get(assertion_id)
            .ok_or_else(|| format!("observation references unknown assertion: {assertion_id}"))?;
        if !assertions.insert(assertion_id) {
            return Err(format!("observation repeats assertion: {assertion_id}"));
        }
        if provider != expected.provider {
            return Err(format!(
                "current observation provider mismatches {assertion_id}"
            ));
        }
        if provider == "github" && scope != "vexil-lang/vexil" {
            return Err("current GitHub observation has an unexpected repository scope".to_owned());
        }
        if provider != "github" && scope != expected.scope {
            return Err(format!(
                "current observation scope mismatches {assertion_id}"
            ));
        }
        let query = object(required_value(result, "query")?, "observation query")?;
        if text(query.get("method"), "observation query method")? != expected.method
            || text(query.get("path"), "observation query path")? != expected.path
        {
            return Err(format!(
                "current observation query mismatches {assertion_id}"
            ));
        }
    }
    Ok(())
}

pub fn validate_workflow_static_isolation(root: &Path) -> Result<(), String> {
    let workflows = root.join(".github/workflows");
    for entry in fs::read_dir(&workflows)
        .map_err(|error| format!("read {}: {error}", workflows.display()))?
    {
        let path = entry
            .map_err(|error| format!("read {} entry: {error}", workflows.display()))?
            .path();
        if !matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("yml" | "yaml")
        ) {
            continue;
        }
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let lower = source.to_ascii_lowercase();
        if lower.contains("pull_request_target") {
            return Err(format!(
                "workflow {} must not run untrusted code through pull_request_target",
                path.display()
            ));
        }
        if lower.contains("permissions: write-all") || lower.contains("permissions: \"write-all\"")
        {
            return Err(format!(
                "workflow {} must not request write-all permissions",
                path.display()
            ));
        }
        let privileged = lower.contains("environment:")
            || lower.lines().any(|line| {
                let permission = line.trim();
                permission.contains(": write") || permission == "permissions: write-all"
            });
        if privileged {
            for line in source.lines().filter(|line| line.contains("uses:")) {
                let reference = line.split("uses:").nth(1).unwrap_or_default().trim();
                let revision = reference.rsplit_once('@').map(|(_, revision)| revision);
                if !revision.is_some_and(|revision| {
                    revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
                }) {
                    return Err(format!(
                        "privileged workflow {} must pin Action ref {reference} to a full commit SHA",
                        path.display()
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn validate_assignments_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let record = read_json(&root.join("release/stewardship/assignments.json"))?;
    validate_assignment_schema(root, &record)?;
    validate_assignments(&record)?;
    validate_assignment_documentation_parity(
        &record,
        &fs::read_to_string(root.join("docs/book/src/release/stewardship-continuity.md"))
            .map_err(|error| format!("read stewardship continuity documentation: {error}"))?,
    )?;
    validate_public_boundary(root)
}

pub fn validate_responsibilities_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let record = read_json(&root.join("release/stewardship/responsibilities.json"))?;
    validate_responsibility_schema(root, &record)?;
    validate_responsibilities(&record)?;
    validate_catalog_comparison(root, &record)?;
    let assignments = read_json(&root.join("release/stewardship/assignments.json"))?;
    validate_advisory_owners(&record, &assignments)?;
    validate_advisory_contract(root, &record)?;
    validate_responsibility_audit_surfaces(root, &record)?;
    validate_responsibility_documentation_parity(
        &record,
        &fs::read_to_string(root.join("docs/book/src/release/retired-bot-responsibilities.md"))
            .map_err(|error| format!("read retired-bot responsibility documentation: {error}"))?,
    )?;
    validate_advisory_runbook_parity(
        &record,
        &fs::read_to_string(root.join("release/runbooks/advisory-automation.md"))
            .map_err(|error| format!("read advisory runbook: {error}"))?,
        &fs::read_to_string(root.join("docs/book/src/release/advisory-automation.md"))
            .map_err(|error| format!("read advisory mdBook view: {error}"))?,
    )?;
    validate_public_boundary(root)
}

pub fn validate_privileged_operations_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let operations = read_json(&root.join("release/privileged/operations-contract.json"))?;
    validate_privileged_operation_schema(root, &operations)?;
    let responsibilities = read_json(&root.join("release/stewardship/responsibilities.json"))?;
    let assignments = read_json(&root.join("release/stewardship/assignments.json"))?;
    validate_privileged_operations(&operations, &responsibilities, &assignments)?;
    validate_privileged_audit_surfaces(root, &operations)?;
    validate_privileged_runbook_parity(
        &operations,
        &responsibilities,
        &fs::read_to_string(root.join("release/runbooks/privileged-readiness-and-fail-closed.md"))
            .map_err(|error| format!("read privileged runbook: {error}"))?,
        &fs::read_to_string(root.join("docs/book/src/release/privileged-operations.md"))
            .map_err(|error| format!("read privileged mdBook view: {error}"))?,
    )?;
    validate_public_boundary(root)
}

pub fn validate_stewardship_exercises_repository(root: &Path) -> Result<(), String> {
    validate_schema_syntax(root)?;
    let exercise =
        read_json(&root.join("release/exercises/tabletop-stewardship-continuity-2026-07-14.json"))?;
    let assignments = read_json(&root.join("release/stewardship/assignments.json"))?;
    let authority = read_json(&root.join("release/stewardship.json"))?;
    validate_stewardship_exercise_schema(root, &exercise)?;
    validate_stewardship_exercise(&exercise, &assignments)?;
    validate_exercise_runbooks(root, &authority)?;
    let documentation =
        fs::read_to_string(root.join("docs/book/src/release/stewardship-exercises.md"))
            .map_err(|error| format!("read stewardship exercise documentation: {error}"))?;
    if documentation != render_stewardship_exercises_markdown(&exercise)? {
        return Err(
            "documentation parity failure: docs/book/src/release/stewardship-exercises.md is stale"
                .to_owned(),
        );
    }
    validate_public_boundary(root)
}

pub fn validate_stewardship_exercise(exercise: &Value, assignments: &Value) -> Result<(), String> {
    let root = object(exercise, "stewardship exercise record")?;
    require_exact_keys(
        root,
        &[
            "$schema",
            "$id",
            "exerciseSchema",
            "version",
            "recordId",
            "kind",
            "mode",
            "exercisedAtUtc",
            "participants",
            "scenarios",
            "evidence",
        ],
        "stewardship exercise record",
    )?;
    require_string(
        root,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
    )?;
    require_string(
        root,
        "exerciseSchema",
        "https://vexil.dev/release/schemas/stewardship-exercise.schema.json",
    )?;
    require_string(root, "version", "1.0")?;
    require_string(root, "kind", "tabletop-stewardship-continuity")?;
    require_string(root, "mode", "tabletop-only-non-mutating")?;
    let id = text(root.get("$id"), "exercise id")?;
    if !id.starts_with("https://vexil.dev/release/exercises/") {
        return Err("exercise record must use a public canonical identifier".to_owned());
    }
    require_utc_timestamp(root.get("exercisedAtUtc"), "exercise UTC time")?;
    let assignment_rows = array(assignments.get("assignments"), "assignment rows")?;
    let known_assignments: BTreeSet<_> = assignment_rows
        .iter()
        .filter_map(|entry| {
            let entry = entry.as_object()?;
            Some((
                entry.get("assignmentId")?.as_str()?,
                entry.get("primaryActorId")?.as_str()?,
                entry.get("roleId")?.as_str()?,
            ))
        })
        .collect();
    let participants = array(root.get("participants"), "exercise participants")?;
    if participants.is_empty() {
        return Err("exercise must record participants".to_owned());
    }
    for participant in participants {
        validate_exercise_actor(participant, &known_assignments, "exercise participant")?;
    }
    let scenarios = array(root.get("scenarios"), "exercise scenarios")?;
    let required_scenarios: BTreeSet<_> = [
        "unavailable-owner",
        "suspected-credential-or-automation-compromise",
        "advisory-failure",
        "missing-provider-control",
    ]
    .into_iter()
    .collect();
    let mut seen = BTreeSet::new();
    let mut scenario_ids = BTreeSet::new();
    for scenario in scenarios {
        let scenario = object(scenario, "exercise scenario")?;
        require_exact_keys(
            scenario,
            &[
                "id",
                "scenario",
                "affectedAuthority",
                "procedureId",
                "allowedActions",
                "prohibitedActions",
                "providerBlockers",
                "observedGaps",
                "followUpOwner",
                "disposition",
            ],
            "exercise scenario",
        )?;
        let scenario_kind = text(scenario.get("scenario"), "scenario kind")?;
        if !required_scenarios.contains(scenario_kind) || !seen.insert(scenario_kind) {
            return Err(
                "exercise scenarios must contain each required scenario exactly once".to_owned(),
            );
        }
        let scenario_id = text(scenario.get("id"), "scenario id")?;
        if scenario_id.is_empty() || !scenario_ids.insert(scenario_id) {
            return Err("exercise scenario identifiers must be stable and unique".to_owned());
        }
        let (expected_authority, expected_procedure, expected_actions) =
            expected_exercise_boundary(scenario_kind)?;
        if text(scenario.get("affectedAuthority"), "affected authority")? != expected_authority
            || text(scenario.get("procedureId"), "procedure id")? != expected_procedure
        {
            return Err(
                "exercise scenario must use its designated authority and procedure".to_owned(),
            );
        }
        let actions = strings(scenario.get("allowedActions"), "exercise allowed actions")?;
        let allowed: BTreeSet<_> = [
            "stop",
            "revoke",
            "contain",
            "activate-succession",
            "perform-manually",
            "defer",
        ]
        .into_iter()
        .collect();
        if actions.is_empty() || actions.iter().any(|action| !allowed.contains(action)) {
            return Err(
                "exercise action exceeds tabletop emergency or advisory boundary".to_owned(),
            );
        }
        let actual_actions: BTreeSet<_> = actions.iter().copied().collect();
        let expected_actions: BTreeSet<_> = expected_actions.iter().copied().collect();
        if actual_actions != expected_actions {
            return Err("exercise scenario actions must match its designated boundary".to_owned());
        }
        if scenario_kind == "advisory-failure"
            && actions
                .iter()
                .any(|action| !["perform-manually", "defer"].contains(action))
        {
            return Err(
                "advisory fallback cannot use emergency or privileged authority".to_owned(),
            );
        }
        let prohibited = strings(
            scenario.get("prohibitedActions"),
            "exercise prohibited actions",
        )?;
        for prohibited_action in [
            "approve-publication",
            "rewrite-evidence",
            "declare-completion",
        ] {
            if !prohibited.contains(&prohibited_action) {
                return Err("exercise must prohibit publication, evidence rewrite, and completion declaration".to_owned());
            }
        }
        let blockers = array(scenario.get("providerBlockers"), "provider blockers")?;
        if blockers.is_empty() {
            return Err("exercise scenario must retain an external-control blocker".to_owned());
        }
        for blocker in blockers {
            let blocker = object(blocker, "provider blocker")?;
            require_exact_keys(
                blocker,
                &["control", "status", "requiredEvidence"],
                "provider blocker",
            )?;
            require_string(blocker, "status", "unverified-external-control-blocker")?;
            if !text(blocker.get("requiredEvidence"), "provider blocker evidence")?
                .contains("Verified")
            {
                return Err("provider blocker must name required control evidence".to_owned());
            }
        }
        if array(scenario.get("observedGaps"), "observed gaps")?.is_empty() {
            return Err("exercise scenario must record observed gaps".to_owned());
        }
        validate_exercise_actor(
            required_value(scenario, "followUpOwner")?,
            &known_assignments,
            "exercise follow-up owner",
        )?;
        require_string(scenario, "disposition", "blocked-pending-external-controls")?;
    }
    if seen != required_scenarios {
        return Err("exercise scenarios are incomplete".to_owned());
    }
    let evidence = object(required_value(root, "evidence")?, "exercise evidence")?;
    require_exact_keys(
        evidence,
        &["destination", "persistence", "secretsIncluded"],
        "exercise evidence",
    )?;
    require_string(
        evidence,
        "destination",
        "release/exercises/tabletop-stewardship-continuity-2026-07-14.json",
    )?;
    require_string(evidence, "persistence", "version-controlled-public-record")?;
    if evidence.get("secretsIncluded").and_then(Value::as_bool) != Some(false) {
        return Err("exercise evidence must not contain secrets".to_owned());
    }
    ensure_no_private_leakage(&exercise.to_string())
}

fn validate_exercise_actor(
    value: &Value,
    known: &BTreeSet<(&str, &str, &str)>,
    context: &str,
) -> Result<(), String> {
    let actor = object(value, context)?;
    require_exact_keys(actor, &["actorId", "assertedRole", "assignmentId"], context)?;
    let triple = (
        text(actor.get("assignmentId"), "assignment id")?,
        text(actor.get("actorId"), "actor id")?,
        text(actor.get("assertedRole"), "asserted role")?,
    );
    if !known.contains(&triple) {
        return Err(format!(
            "{context} must reference a current stewardship assignment"
        ));
    }
    Ok(())
}

fn expected_exercise_boundary(
    scenario: &str,
) -> Result<(&'static str, &'static str, &'static [&'static str]), String> {
    match scenario {
        "unavailable-owner" => Ok((
            "release-steward",
            "release-continuity-runbook",
            &["stop", "contain", "activate-succession"],
        )),
        "suspected-credential-or-automation-compromise" => Ok((
            "repository-administrator",
            "emergency-stop-runbook",
            &["stop", "revoke", "contain"],
        )),
        "advisory-failure" => Ok((
            "release-run-coordinator",
            "advisory-manual-fallback-runbook",
            &["perform-manually", "defer"],
        )),
        "missing-provider-control" => Ok((
            "repository-administrator",
            "trust-revocation-runbook",
            &["stop", "revoke", "contain", "activate-succession"],
        )),
        _ => Err("unknown stewardship exercise scenario".to_owned()),
    }
}

fn validate_exercise_runbooks(root: &Path, authority: &Value) -> Result<(), String> {
    validate_emergency_runbook_authority(authority)?;
    for (relative, procedure, roles, actions) in [
        (
            "release/runbooks/stewardship-succession.md",
            "release-continuity-runbook",
            &["repository-administrator"][..],
            &["stop", "contain", "activate-succession"][..],
        ),
        (
            "release/runbooks/unavailable-owner.md",
            "unavailable-owner-runbook",
            &["repository-administrator"][..],
            &["stop", "contain", "activate-succession"][..],
        ),
        (
            "release/runbooks/emergency-stop.md",
            "emergency-stop-runbook",
            &["repository-administrator"][..],
            &["stop", "revoke", "contain", "activate-succession"][..],
        ),
        (
            "release/runbooks/trust-revocation.md",
            "trust-revocation-runbook",
            &["repository-administrator"][..],
            &["stop", "revoke", "contain", "activate-succession"][..],
        ),
        (
            "release/runbooks/advisory-manual-fallback.md",
            "advisory-manual-fallback-runbook",
            &["release-run-coordinator", "repository-administrator"][..],
            &["perform-manually", "defer"][..],
        ),
    ] {
        let content = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("read {relative}: {error}"))?;
        validate_exercise_runbook_content(&content, procedure, relative)?;
        validate_exercise_runbook_boundary(&content, roles, actions, relative)?;
        if procedure == "emergency-stop-runbook" {
            validate_emergency_control_inventory(root, &content)?;
        }
    }
    Ok(())
}

fn validate_emergency_runbook_authority(authority: &Value) -> Result<(), String> {
    let roles = array(authority.get("roles"), "authority roles")?;
    let administrator = roles
        .iter()
        .filter_map(Value::as_object)
        .find(|role| role.get("id").and_then(Value::as_str) == Some("repository-administrator"))
        .ok_or_else(|| "authority record lacks Repository Administrator role".to_owned())?;
    let permitted: BTreeSet<_> = strings(
        administrator.get("permittedActions"),
        "administrator actions",
    )?
    .into_iter()
    .collect();
    let expected: BTreeSet<_> = ["stop", "revoke", "contain", "activate-succession"]
        .into_iter()
        .collect();
    if !expected.is_subset(&permitted) {
        return Err("authority record must permit the complete emergency boundary".to_owned());
    }
    let prohibited = strings(
        administrator.get("prohibitedActions"),
        "administrator prohibitions",
    )?;
    if [
        "approve-publication",
        "rewrite-evidence",
        "move-tag",
        "declare-completion",
    ]
    .iter()
    .any(|action| !prohibited.contains(action))
    {
        return Err("authority record must prohibit publication, evidence rewrite, tag repair, and completion declaration".to_owned());
    }
    Ok(())
}

fn validate_emergency_control_inventory(root: &Path, content: &str) -> Result<(), String> {
    let release_workflow = fs::read_to_string(root.join(".github/workflows/release.yml"))
        .map_err(|error| format!("read release workflow control surface: {error}"))?;
    let legacy_credential_route = "secrets.RELEASE_TOKEN || secrets.GITHUB_TOKEN";
    if release_workflow.contains(legacy_credential_route)
        && (!content.contains("RELEASE_TOKEN") || !content.contains("GITHUB_TOKEN"))
    {
        return Err(
            "emergency-stop inventory must name every current release credential route".to_owned(),
        );
    }
    if !release_workflow.contains(legacy_credential_route)
        && !content.contains("No active release credential route")
    {
        return Err("emergency-stop inventory must distinguish a removed committed credential route from live provider evidence".to_owned());
    }
    Ok(())
}

pub fn validate_exercise_runbook_boundary(
    content: &str,
    expected_roles: &[&str],
    expected_actions: &[&str],
    label: &str,
) -> Result<(), String> {
    let mut actions = BTreeSet::new();
    let mut in_decision_table = false;
    for line in content.lines().filter(|line| line.starts_with('|')) {
        if line.contains("| Decision point | Asserted role | Allowed action |") {
            in_decision_table = true;
            continue;
        }
        if !in_decision_table {
            continue;
        }
        let cells: Vec<_> = line.split('|').map(str::trim).collect();
        if cells.len() < 6 || cells[1] == "Decision point" || cells[1].starts_with("---") {
            continue;
        }
        if !expected_roles.contains(&cells[2].trim_matches('`')) {
            return Err(format!("runbook uses an unexpected asserted role: {label}"));
        }
        actions.extend(
            cells[3]
                .split(',')
                .map(str::trim)
                .filter(|action| !action.is_empty()),
        );
    }
    let expected: BTreeSet<_> = expected_actions.iter().copied().collect();
    if actions != expected {
        return Err(format!(
            "runbook actions exceed or omit its canonical tabletop boundary: {label}"
        ));
    }
    Ok(())
}

pub fn validate_exercise_runbook_content(
    content: &str,
    procedure: &str,
    label: &str,
) -> Result<(), String> {
    ensure_no_private_leakage(content)?;
    if !content.contains(procedure)
        || !content.contains("Asserted role")
        || !content.contains("Allowed action")
        || !content.contains("Prohibited action")
        || !content.contains("Evidence destination")
        || !content.contains("Prerequisite")
        || !content.contains("Stop condition")
        || !content.contains("Follow-up owner")
        || !content.contains("Tabletop-only")
    {
        return Err(format!(
            "runbook is missing a required decision point: {label}"
        ));
    }
    if !content
        .to_ascii_lowercase()
        .contains("unverified external-control blocker")
    {
        return Err(format!(
            "runbook must retain an explicit unverified external-control blocker: {label}"
        ));
    }
    if ["```sh", "```bash", "```zsh", "```powershell", "```ps1"]
        .iter()
        .any(|fence| content.to_ascii_lowercase().contains(fence))
    {
        return Err(format!(
            "runbook contains a forbidden executable or effectful instruction: {label}"
        ));
    }
    for forbidden in [
        "git tag",
        "git push",
        "git commit",
        "git update-ref",
        "gh release",
        "gh api",
        "gh workflow",
        "gh secret",
        "gh variable",
        "gh repo edit",
        "npm publish",
        "npm deprecate",
        "npm unpublish",
        "cargo publish",
        "cargo yank",
        "workflow_dispatch",
        "curl ",
        "wget ",
        "invoke-webrequest",
        "kubectl ",
        "terraform ",
        "token=",
    ] {
        if content.to_ascii_lowercase().contains(forbidden) {
            return Err(format!(
                "runbook contains a forbidden executable or effectful instruction: {label}"
            ));
        }
    }
    Ok(())
}

pub fn render_stewardship_exercises_markdown(exercise: &Value) -> Result<String, String> {
    validate_exercise_shape_for_render(exercise)?;
    let root = object(exercise, "exercise")?;
    let mut markdown = String::from("# Stewardship Continuity Tabletop Exercises\n\n> Generated public view of [`release/exercises/tabletop-stewardship-continuity-2026-07-14.json`](../../../../release/exercises/tabletop-stewardship-continuity-2026-07-14.json). The JSON record is canonical; this page is parity-checked and non-authoritative.\n\nThese are tabletop-only, non-mutating exercises, not Release Runs. They retain the historical absence of a distinct custodian; the current sole-maintainer policy does not make that absence a release gate. Independent external-control and release-evidence gates remain blocked.\n\n## Record\n\n");
    markdown.push_str(&format!("Record `{}` was exercised at `{}`. Evidence is retained as a version-controlled public record with no secrets.\n\n", text(root.get("recordId"), "record id")?, text(root.get("exercisedAtUtc"), "exercise time")?));
    markdown.push_str("## Scenarios\n\n| Scenario | Procedure | Allowed boundary | Disposition |\n|---|---|---|---|\n");
    for scenario in array(root.get("scenarios"), "scenarios")? {
        let scenario = object(scenario, "scenario")?;
        markdown.push_str(&format!(
            "| `{}` | `{}` | {} | `{}` |\n",
            text(scenario.get("scenario"), "scenario")?,
            text(scenario.get("procedureId"), "procedure")?,
            strings(scenario.get("allowedActions"), "allowed actions")?.join(", "),
            text(scenario.get("disposition"), "disposition")?
        ));
    }
    markdown.push_str("\n## Public runbooks\n\n- [Stewardship succession](../../../../release/runbooks/stewardship-succession.md)\n- [Unavailable owner](../../../../release/runbooks/unavailable-owner.md)\n- [Emergency stop](../../../../release/runbooks/emergency-stop.md)\n- [Trust revocation](../../../../release/runbooks/trust-revocation.md)\n- [Advisory manual fallback](../../../../release/runbooks/advisory-manual-fallback.md)\n\nEvery provider-specific action is an **unverified external-control blocker**. This evidence identifies future control categories; it does not test, configure, revoke, stop, publish, deploy, approve, or mutate any provider state.\n\n## Offline validation\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nThe validator checks canonical assignment linkage, action boundaries, explicit external-control blockers, public persistence, no secrets, required decision fields, and runbook safety. It does not invoke provider controls.\n");
    Ok(markdown)
}

fn validate_exercise_shape_for_render(exercise: &Value) -> Result<(), String> {
    let root = object(exercise, "exercise")?;
    for field in ["recordId", "exercisedAtUtc", "scenarios"] {
        required_value(root, field)?;
    }
    Ok(())
}

pub fn validate_contract(record: &Value) -> Result<(), String> {
    let root = object(record, "contract must be a JSON object")?;
    if root.contains_key("assignments") {
        return Err(
            "role assignments belong to the stewardship assignment record and cannot appear in the authority record"
                .to_owned(),
        );
    }
    require_exact_keys(root, &ROOT_FIELDS, "contract")?;
    require_string(
        root,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
    )?;
    require_string(root, "$id", "https://vexil.dev/release/stewardship.json")?;
    require_string(
        root,
        "contractSchema",
        "https://vexil.dev/release/schemas/stewardship.schema.json",
    )?;
    require_string(root, "version", "1.0")?;

    let roles = array(root.get("roles"), "roles")?;
    let mut by_id = HashMap::new();
    for role in roles {
        let fields = object(role, "role")?;
        require_exact_keys(fields, &ROLE_FIELDS, "role")?;
        let id = text(fields.get("id"), "role id")?;
        if by_id.insert(id, fields).is_some() {
            return Err(format!("duplicate role id: {id}"));
        }
        for field in [
            "label",
            "decisionScope",
            "continuityRequirement",
            "roleCombinationConstraints",
        ] {
            if text(fields.get(field), field)?.is_empty() {
                return Err(format!("role {id} has an empty {field}"));
            }
        }
        for field in [
            "permittedActions",
            "prohibitedActions",
            "approvalDuties",
            "auditSurface",
        ] {
            if array(fields.get(field), field)?.is_empty() {
                return Err(format!("role {id} has an empty {field}"));
            }
        }
        let permitted = strings(fields.get("permittedActions"), "permittedActions")?;
        let prohibited = strings(fields.get("prohibitedActions"), "prohibitedActions")?;
        for action in permitted.iter().chain(prohibited.iter()) {
            if !ACTIONS.contains(action) {
                return Err(format!("role {id} uses unknown action: {action}"));
            }
        }
        if permitted.iter().any(|action| prohibited.contains(action)) {
            return Err(format!("role {id} both permits and prohibits an action"));
        }
        if !text(
            fields.get("roleCombinationConstraints"),
            "role combination constraints",
        )?
        .contains("explicit asserted role")
        {
            return Err(format!(
                "role {id} must require an explicit asserted role when roles are combined"
            ));
        }
    }
    let actual: BTreeSet<_> = by_id.keys().copied().collect();
    let expected: BTreeSet<_> = REQUIRED_ROLE_IDS.into_iter().collect();
    if actual != expected {
        return Err(format!(
            "missing or unexpected required roles: expected {expected:?}, got {actual:?}"
        ));
    }
    require_actions(
        by_id["release-steward"],
        &[
            "approve-release-manifest",
            "authorize-privileged-release",
            "close-release-manifest",
        ],
        "release-steward",
    )?;
    require_actions(
        by_id["repository-administrator"],
        &["stop", "revoke", "contain", "activate-succession"],
        "repository-administrator",
    )?;
    require_actions(
        by_id["security-steward"],
        &[
            "disposition-vulnerability",
            "set-disclosure-remediation-policy",
            "grant-time-bounded-security-exception",
        ],
        "security-steward",
    )?;
    require_actions(
        by_id["package-steward"],
        &[
            "verify-assigned-release-unit",
            "verify-namespace-health",
            "verify-packaging-health",
        ],
        "package-steward",
    )?;
    require_actions(
        by_id["release-run-coordinator"],
        &["sequence-release-run", "execute-authorized-release-action"],
        "release-run-coordinator",
    )?;
    let emergency_forbidden = [
        "move-tag",
        "overwrite-artifact",
        "rewrite-evidence",
        "accept-security-risk",
        "approve-publication",
        "declare-completion",
    ];
    let admin_prohibited = strings(
        by_id["repository-administrator"].get("prohibitedActions"),
        "administrator prohibitedActions",
    )?;
    if !emergency_forbidden
        .iter()
        .all(|action| admin_prohibited.contains(action))
    {
        return Err("repository administrator emergency authority is over-broad".to_owned());
    }

    let authorization = object(
        required_value(root, "privilegedAuthorization")?,
        "privilegedAuthorization",
    )?;
    require_exact_keys(
        authorization,
        &[
            "requiredRole",
            "requiredRoleAssertion",
            "approvedReleaseManifest",
            "rejectedEvidence",
        ],
        "privilegedAuthorization",
    )?;
    require_string(authorization, "requiredRole", "release-steward")?;
    require_string(
        authorization,
        "requiredRoleAssertion",
        "explicit asserted role",
    )?;
    let manifest = object(
        required_value(authorization, "approvedReleaseManifest")?,
        "approvedReleaseManifest",
    )?;
    require_exact_keys(
        manifest,
        &["status", "identity", "digest"],
        "approvedReleaseManifest",
    )?;
    require_string(manifest, "status", "approved")?;
    for required in ["identity", "digest"] {
        if text(manifest.get(required), required)?.is_empty() {
            return Err(format!("approved release manifest requires {required}"));
        }
    }
    let rejected = strings(authorization.get("rejectedEvidence"), "rejectedEvidence")?;
    for evidence in [
        "tag",
        "bot",
        "workflow",
        "green-ci",
        "registry",
        "provider-approval",
        "private-build-artifact",
        "private-review-note",
        "non-public-workspace-input",
    ] {
        if !rejected.contains(&evidence) {
            return Err(format!("non-authority evidence is missing: {evidence}"));
        }
    }

    let non_authorities = strings(root.get("nonAuthorityClasses"), "nonAuthorityClasses")?;
    for class in [
        "bots",
        "workflows",
        "green-ci",
        "registries",
        "provider-approvals",
        "private-build-artifacts",
    ] {
        if !non_authorities.contains(&class) {
            return Err(format!("missing non-authority class: {class}"));
        }
    }
    let automation = object(
        required_value(root, "advisoryAutomation")?,
        "advisoryAutomation",
    )?;
    require_exact_keys(
        automation,
        &["allowedActions", "prohibitedActions"],
        "advisoryAutomation",
    )?;
    let allowed_advisory_actions = strings(
        automation.get("allowedActions"),
        "advisory automation allowedActions",
    )?;
    let expected_advisory_actions: BTreeSet<_> = [
        "validate",
        "triage",
        "label",
        "dependency-advice",
        "rehearse",
    ]
    .into_iter()
    .collect();
    if allowed_advisory_actions.len() != expected_advisory_actions.len()
        || allowed_advisory_actions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != expected_advisory_actions
    {
        return Err("advisory automation may only perform its fixed advisory actions".to_owned());
    }
    let automated_denials = strings(
        automation.get("prohibitedActions"),
        "advisory automation prohibitedActions",
    )?;
    for denied in [
        "move-tag",
        "authorize-privileged-release",
        "publish-package",
        "deploy",
        "change-protected-branch",
        "access-environment",
        "use-credential",
        "select-semantic-version",
        "select-release-set-scope",
        "accept-security-risk",
    ] {
        if !automated_denials.contains(&denied) {
            return Err(format!("advisory automation is not denied {denied}"));
        }
    }
    if allowed_advisory_actions
        .iter()
        .any(|action| automated_denials.contains(action))
    {
        return Err("advisory automation actions cannot be both allowed and prohibited".to_owned());
    }

    let governance = object(required_value(root, "governanceRoute")?, "governanceRoute")?;
    require_exact_keys(
        governance,
        &[
            "formalSourceOrder",
            "rfcRequiredFor",
            "publicReviewRequirement",
            "nonBypassStatement",
        ],
        "governanceRoute",
    )?;
    if strings(governance.get("formalSourceOrder"), "formalSourceOrder")?
        != [
            "spec/",
            "schemas/",
            "docs/",
            "implementation-and-tests",
            "release-metadata",
        ]
    {
        return Err("formal source-of-truth hierarchy has changed".to_owned());
    }
    let required_rfc_categories: BTreeSet<_> = [
        "language",
        "wire-format",
        "compiler",
        "generator",
        "runtime",
        "corpus/conformance",
        "public-api",
    ]
    .into_iter()
    .collect();
    let rfc_categories = strings(governance.get("rfcRequiredFor"), "rfcRequiredFor")?;
    if rfc_categories.len() != required_rfc_categories.len()
        || rfc_categories.iter().copied().collect::<BTreeSet<_>>() != required_rfc_categories
    {
        return Err("RFC-required governance categories have changed".to_owned());
    }
    let public_review = text(
        governance.get("publicReviewRequirement"),
        "publicReviewRequirement",
    )?;
    if !["GOVERNANCE.md remains binding", "14-day", "RFC process"]
        .iter()
        .all(|requirement| public_review.contains(requirement))
    {
        return Err("binding public-review requirement is absent".to_owned());
    }
    if !text(governance.get("nonBypassStatement"), "nonBypassStatement")?.contains("cannot bypass")
    {
        return Err("governance bypass protection is absent".to_owned());
    }
    let publication_block = text(root.get("publicationBlock"), "publicationBlock")?;
    if !(publication_block.contains("stewardship assignments")
        || publication_block.contains("sole-maintainer"))
        || !(publication_block.contains("external controls")
            || publication_block.contains("external-control"))
    {
        return Err(
            "publication block must name stewardship and external-control gates".to_owned(),
        );
    }
    ensure_no_private_leakage(&record.to_string())
}

pub fn validate_assignments(record: &Value) -> Result<(), String> {
    let root = object(record, "assignment record must be a JSON object")?;
    require_exact_keys(root, &ASSIGNMENT_ROOT_FIELDS, "assignment record")?;
    require_string(
        root,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
    )?;
    require_string(
        root,
        "$id",
        "https://vexil.dev/release/stewardship/assignments.json",
    )?;
    require_string(
        root,
        "assignmentSchema",
        "https://vexil.dev/release/schemas/stewardship-assignment.schema.json",
    )?;
    require_string(root, "version", "1.0")?;

    let identities = array(root.get("identities"), "identities")?;
    let mut identity_ids = BTreeSet::new();
    for identity in identities {
        let identity = object(identity, "identity")?;
        require_exact_keys(identity, &["id", "name", "email", "github"], "identity")?;
        let id = text(identity.get("id"), "identity id")?;
        if !id.starts_with("github:") || !identity_ids.insert(id) {
            return Err("identity ids must be unique GitHub governed identities".to_owned());
        }
        for field in ["name", "email", "github"] {
            if text(identity.get(field), field)?.is_empty() {
                return Err(format!("identity {id} has an empty {field}"));
            }
        }
    }

    let decision = object(required_value(root, "decision")?, "decision")?;
    require_exact_keys(
        decision,
        &["id", "status", "effectiveFrom", "reviewEvidence"],
        "decision",
    )?;
    let decision_status = text(decision.get("status"), "decision status")?;
    if ![
        "unresolved-continuity",
        "sole-maintainer-governance",
        "single-steward-custodian",
        "multi-steward-detached-approval",
    ]
    .contains(&decision_status)
    {
        return Err("unknown continuity decision status".to_owned());
    }
    require_date(decision.get("effectiveFrom"), "decision effectiveFrom")?;
    validate_evidence(
        required_value(decision, "reviewEvidence")?,
        &identity_ids,
        "decision review evidence",
        Some(text(decision.get("id"), "decision id")?),
        None,
    )?;
    let decision_evidence = object(
        required_value(decision, "reviewEvidence")?,
        "decision review evidence",
    )?;
    let decision_source = text(decision_evidence.get("source"), "decision review source")?;
    let assignments = array(root.get("assignments"), "assignments")?;
    let mut assignment_ids = BTreeSet::new();
    let mut assigned_roles = BTreeSet::new();
    let mut package_roots = BTreeSet::new();
    for assignment in assignments {
        let assignment = object(assignment, "assignment")?;
        if assignment.contains_key("permittedActions")
            || assignment.contains_key("prohibitedActions")
        {
            return Err("combined-role assignments cannot escalate role permissions".to_owned());
        }
        require_exact_keys(assignment, &ASSIGNMENT_FIELDS, "assignment")?;
        let assignment_id = text(assignment.get("assignmentId"), "assignment id")?;
        if assignment_id.is_empty() || !assignment_ids.insert(assignment_id) {
            return Err("assignment IDs must be stable and unique".to_owned());
        }
        let role_id = text(assignment.get("roleId"), "assignment role id")?;
        if !REQUIRED_ROLE_IDS.contains(&role_id) {
            return Err(format!("assignment uses unknown role: {role_id}"));
        }
        let actor_id = text(assignment.get("primaryActorId"), "assignment primary actor")?;
        if !identity_ids.contains(actor_id) {
            return Err(format!(
                "assignment {assignment_id} names an unknown primary identity"
            ));
        }
        let scope = object(required_value(assignment, "scope")?, "assignment scope")?;
        require_exact_keys(scope, &["kind", "root"], "assignment scope")?;
        let kind = text(scope.get("kind"), "assignment scope kind")?;
        let scope_root = text(scope.get("root"), "assignment scope root")?;
        if scope_root.is_empty() || scope_root == "*" || scope_root.eq_ignore_ascii_case("all") {
            return Err("Package Steward scope cannot use a vague catch-all root".to_owned());
        }
        if role_id == "package-steward" {
            if kind != "maintained-root" {
                return Err("Package Steward assignments must name a maintained root".to_owned());
            }
            if !package_roots.insert(scope_root) {
                return Err(format!(
                    "duplicate Package Steward root assignment: {scope_root}"
                ));
            }
        } else {
            let expected_kind = match role_id {
                "release-steward" => "release-manifest-lifecycle",
                "repository-administrator" => "repository",
                "security-steward" => "security-governance",
                "release-run-coordinator" => "release-run-execution",
                _ => unreachable!(),
            };
            if kind != expected_kind
                || (role_id != "release-steward" && !assigned_roles.insert(role_id))
            {
                return Err(format!(
                    "role {role_id} must have one independent scoped assignment"
                ));
            }
            assigned_roles.insert(role_id);
        }
        require_date(assignment.get("effectiveFrom"), "assignment effectiveFrom")?;
        validate_evidence(
            required_value(assignment, "reviewEvidence")?,
            &identity_ids,
            "assignment review evidence",
            Some(text(decision.get("id"), "decision id")?),
            Some(decision_source),
        )?;
        if text(
            assignment.get("continuityProcedure"),
            "continuity procedure",
        )?
        .is_empty()
        {
            return Err("assignment continuity procedure cannot be empty".to_owned());
        }
        require_string(assignment, "status", "active")?;
    }
    let expected_non_package: BTreeSet<_> = REQUIRED_ROLE_IDS
        .iter()
        .copied()
        .filter(|role| *role != "package-steward")
        .collect();
    if assigned_roles != expected_non_package {
        return Err("missing independently auditable required role assignment".to_owned());
    }
    let expected_roots: BTreeSet<_> = MAINTAINED_ROOTS.into_iter().collect();
    if package_roots != expected_roots {
        return Err(
            "Package Steward assignments must cover every current maintained root".to_owned(),
        );
    }

    let continuity = object(required_value(root, "continuity")?, "continuity")?;
    require_exact_keys(
        continuity,
        &[
            "qualifiedReleaseStewardActorIds",
            "custodian",
            "recoveryContact",
            "unavailableOwnerRoute",
            "detachedApproval",
        ],
        "continuity",
    )?;
    let qualified = strings(
        continuity.get("qualifiedReleaseStewardActorIds"),
        "qualified Release Stewards",
    )?;
    let qualified_set: BTreeSet<_> = qualified.iter().copied().collect();
    let release_steward_actor_ids: BTreeSet<_> = assignments
        .iter()
        .filter_map(|entry| {
            let entry = entry.as_object()?;
            (entry.get("roleId")?.as_str() == Some("release-steward"))
                .then(|| entry.get("primaryActorId")?.as_str())
                .flatten()
        })
        .collect();
    if qualified.is_empty()
        || qualified_set.len() != qualified.len()
        || !qualified_set.iter().all(|id| identity_ids.contains(id))
        || !qualified_set
            .iter()
            .all(|id| release_steward_actor_ids.contains(id))
        || release_steward_actor_ids != qualified_set
    {
        return Err("qualified Release Stewards must be distinct assigned identities".to_owned());
    }
    validate_recovery_contact(
        required_value(continuity, "recoveryContact")?,
        decision_status,
    )?;
    validate_unavailable_owner_route(required_value(continuity, "unavailableOwnerRoute")?)?;
    validate_continuity_state(
        decision_status,
        &qualified_set,
        continuity.get("custodian"),
        required_value(continuity, "detachedApproval")?,
        &identity_ids,
    )?;

    let readiness = object(
        required_value(root, "publicationReadiness")?,
        "publication readiness",
    )?;
    require_exact_keys(
        readiness,
        &["manifestApproval", "privilegedPublication", "reason"],
        "publication readiness",
    )?;
    require_string(readiness, "manifestApproval", "blocked")?;
    require_string(readiness, "privilegedPublication", "blocked")?;
    let reason = text(readiness.get("reason"), "publication readiness reason")?;
    let reason_lower = reason.to_ascii_lowercase();
    if reason.is_empty()
        || (decision_status == "unresolved-continuity" && !reason_lower.contains("continuity"))
        || (decision_status == "sole-maintainer-governance"
            && (reason_lower.contains("continuity")
                || reason_lower.contains("custodian")
                || !reason_lower.contains("external controls")
                || !reason_lower.contains("registry")))
    {
        return Err(
            "publication readiness must state the actual remaining fail-closed gates".to_owned(),
        );
    }

    let runbooks = array(root.get("futureRunbooks"), "future runbooks")?;
    if runbooks.iter().all(|runbook| {
        object(runbook, "future runbook")
            .and_then(|runbook| {
                require_exact_keys(runbook, &["id", "status"], "future runbook")?;
                Ok(text(runbook.get("id"), "future runbook id")? != "release-continuity-runbook")
            })
            .unwrap_or(true)
    }) {
        return Err("future continuity runbook identifier is missing".to_owned());
    }
    ensure_no_private_leakage(&record.to_string())
}

fn validate_evidence(
    value: &Value,
    identities: &BTreeSet<&str>,
    context: &str,
    expected_decision_id: Option<&str>,
    expected_source: Option<&str>,
) -> Result<(), String> {
    let evidence = object(value, context)?;
    require_exact_keys(
        evidence,
        &["decisionId", "source", "reviewedBy", "reviewedAt"],
        context,
    )?;
    for field in ["decisionId", "source", "reviewedBy"] {
        if text(evidence.get(field), field)?.is_empty() {
            return Err(format!("{context} {field} cannot be empty"));
        }
    }
    if !identities.contains(text(evidence.get("reviewedBy"), "reviewedBy")?) {
        return Err(format!("{context} reviewer is not a governed identity"));
    }
    require_date(evidence.get("reviewedAt"), "review evidence date")?;
    let decision_id = text(evidence.get("decisionId"), "review evidence decision id")?;
    if expected_decision_id.is_some_and(|expected| decision_id != expected) {
        return Err(format!("{context} must cite the canonical decision id"));
    }
    let source = text(evidence.get("source"), "review evidence source")?;
    if expected_source.is_some_and(|expected| source != expected) {
        return Err(format!("{context} must cite the canonical decision source"));
    }
    ensure_public_decision_source(source)
}

fn validate_unavailable_owner_route(value: &Value) -> Result<(), String> {
    let route = object(value, "unavailable owner route")?;
    require_exact_keys(
        route,
        &["allowedActions", "prohibitedActions", "outcome"],
        "unavailable owner route",
    )?;
    let allowed = strings(
        route.get("allowedActions"),
        "unavailable owner allowed actions",
    )?;
    let expected: BTreeSet<_> = ["stop", "revoke", "contain", "activate-succession"]
        .into_iter()
        .collect();
    if allowed.into_iter().collect::<BTreeSet<_>>() != expected {
        return Err("unavailable-owner path can only contain or activate succession".to_owned());
    }
    let prohibited = strings(
        route.get("prohibitedActions"),
        "unavailable owner prohibited actions",
    )?;
    for forbidden in [
        "approve-release-manifest",
        "authorize-privileged-release",
        "approve-publication",
        "move-tag",
        "overwrite-artifact",
        "rewrite-evidence",
        "accept-security-risk",
        "declare-completion",
    ] {
        if !prohibited.contains(&forbidden) {
            return Err(
                "unavailable-owner path must not authorize release or alter immutable history"
                    .to_owned(),
            );
        }
    }
    if !text(route.get("outcome"), "unavailable owner outcome")?
        .contains("cannot create release authority")
    {
        return Err(
            "unavailable-owner path must state that it cannot create release authority".to_owned(),
        );
    }
    Ok(())
}

fn validate_recovery_contact(value: &Value, decision_status: &str) -> Result<(), String> {
    let contact = object(value, "recovery contact")?;
    require_exact_keys(
        contact,
        &["status", "publicRoute", "outcome"],
        "recovery contact",
    )?;
    let expected_status = match decision_status {
        "sole-maintainer-governance" => "sole-maintainer-no-designated-successor",
        _ => "unresolved-no-distinct-custodian",
    };
    require_string(contact, "status", expected_status)?;
    require_string(
        contact,
        "publicRoute",
        "https://github.com/vexil-lang/vexil/issues/new/choose",
    )?;
    let outcome = text(contact.get("outcome"), "recovery contact outcome")?;
    if !outcome.contains("no recovery, Manifest, or publication authority") {
        return Err("recovery contact must fail closed without granting authority".to_owned());
    }
    Ok(())
}

fn validate_continuity_state(
    status: &str,
    qualified: &BTreeSet<&str>,
    custodian_value: Option<&Value>,
    detached_value: &Value,
    identities: &BTreeSet<&str>,
) -> Result<(), String> {
    let detached = object(detached_value, "detached approval")?;
    require_exact_keys(
        detached,
        &[
            "status",
            "manifestApproverActorId",
            "detachedApproverActorId",
            "rule",
        ],
        "detached approval",
    )?;
    let detached_status = text(detached.get("status"), "detached approval status")?;
    let manifest_approver = detached
        .get("manifestApproverActorId")
        .and_then(Value::as_str);
    let detached_approver = detached
        .get("detachedApproverActorId")
        .and_then(Value::as_str);
    if text(detached.get("rule"), "detached approval rule")?.is_empty() {
        return Err("detached approval rule cannot be empty".to_owned());
    }
    match status {
        "unresolved-continuity" => {
            if qualified.len() != 1 || !custodian_value.is_some_and(Value::is_null) {
                return Err(
                    "unresolved continuity must expose the missing distinct custodian".to_owned(),
                );
            }
            if detached_status != "not-applicable-without-second-qualified-release-steward"
                || manifest_approver.is_some()
                || detached_approver.is_some()
            {
                return Err(
                    "single-steward unresolved continuity cannot claim detached approval"
                        .to_owned(),
                );
            }
        }
        "sole-maintainer-governance" => {
            if qualified.len() != 1 || !custodian_value.is_some_and(Value::is_null) {
                return Err("sole-maintainer governance requires exactly one steward and no invented custodian".to_owned());
            }
            if detached_status != "not-applicable-without-second-qualified-release-steward"
                || manifest_approver.is_some()
                || detached_approver.is_some()
            {
                return Err("sole-maintainer governance cannot claim detached approval".to_owned());
            }
        }
        "single-steward-custodian" => {
            if qualified.len() != 1 {
                return Err(
                    "single-steward continuity requires exactly one qualified Release Steward"
                        .to_owned(),
                );
            }
            let custodian = object(
                custodian_value
                    .ok_or_else(|| "single-steward continuity requires a custodian".to_owned())?,
                "continuity custodian",
            )?;
            require_exact_keys(
                custodian,
                &[
                    "actorId",
                    "nonPublishingCapabilities",
                    "hasNormalPublicationCredential",
                ],
                "continuity custodian",
            )?;
            let custodian_id = text(custodian.get("actorId"), "custodian actor id")?;
            if !identities.contains(custodian_id) || qualified.contains(custodian_id) {
                return Err(
                    "single-steward continuity requires a distinct governed custodian".to_owned(),
                );
            }
            if custodian
                .get("hasNormalPublicationCredential")
                .and_then(Value::as_bool)
                != Some(false)
            {
                return Err(
                    "continuity custodian must not hold a normal publication credential".to_owned(),
                );
            }
            let capabilities = strings(
                custodian.get("nonPublishingCapabilities"),
                "custodian non-publishing capabilities",
            )?;
            let expected: BTreeSet<_> = [
                "recover-administration",
                "stop-automation",
                "revoke-trust",
                "initiate-succession",
            ]
            .into_iter()
            .collect();
            if capabilities.into_iter().collect::<BTreeSet<_>>() != expected {
                return Err("continuity custodian must have only the required non-publishing recovery capabilities".to_owned());
            }
            if detached_status != "not-applicable-without-second-qualified-release-steward"
                || manifest_approver.is_some()
                || detached_approver.is_some()
            {
                return Err("single-steward continuity cannot claim detached approval".to_owned());
            }
        }
        "multi-steward-detached-approval" => {
            if qualified.len() < 2 || !custodian_value.is_some_and(Value::is_null) {
                return Err("multi-steward continuity requires two qualified stewards and no single-steward custodian".to_owned());
            }
            if detached_status != "mandatory"
                || manifest_approver.is_none()
                || detached_approver.is_none()
                || manifest_approver == detached_approver
                || !qualified.contains(manifest_approver.unwrap())
                || !qualified.contains(detached_approver.unwrap())
            {
                return Err(
                    "detached approval requires an identity-distinct qualified approver".to_owned(),
                );
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn require_date(value: Option<&Value>, context: &str) -> Result<(), String> {
    let value = text(value, context)?;
    parse_iso_date(value)
        .map(|_| ())
        .map_err(|_| format!("{context} must be an ISO date"))
}

fn parse_iso_date(value: &str) -> Result<(u32, u32, u32), String> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 4 | 7) && !byte.is_ascii_digit())
    {
        return Err("invalid ISO date shape".to_owned());
    }
    let year = value[0..4].parse::<u32>().map_err(|_| "invalid year")?;
    let month = value[5..7].parse::<u32>().map_err(|_| "invalid month")?;
    let day = value[8..10].parse::<u32>().map_err(|_| "invalid day")?;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => return Err("invalid month".to_owned()),
    };
    if day == 0 || day > days {
        return Err("invalid day".to_owned());
    }
    Ok((year, month, day))
}

fn require_utc_timestamp(value: Option<&Value>, context: &str) -> Result<(), String> {
    let value = text(value, context)?;
    let bytes = value.as_bytes();
    if bytes.len() < 20 || bytes.get(10) != Some(&b'T') || bytes.last() != Some(&b'Z') {
        return Err(format!("{context} must be an ISO UTC timestamp"));
    }
    parse_iso_date(&value[..10]).map_err(|_| format!("{context} must be an ISO UTC timestamp"))?;
    let time = &value[11..value.len() - 1];
    let time_bytes = time.as_bytes();
    if time_bytes.len() < 8
        || time_bytes[2] != b':'
        || time_bytes[5] != b':'
        || time_bytes[..8]
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 2 | 5) && !byte.is_ascii_digit())
        || (time_bytes.len() > 8
            && (time_bytes[8] != b'.' || time_bytes[9..].iter().any(|byte| !byte.is_ascii_digit())))
    {
        return Err(format!("{context} must be an ISO UTC timestamp"));
    }
    let hour = time[..2]
        .parse::<u32>()
        .map_err(|_| format!("{context} must be an ISO UTC timestamp"))?;
    let minute = time[3..5]
        .parse::<u32>()
        .map_err(|_| format!("{context} must be an ISO UTC timestamp"))?;
    let second = time[6..8]
        .parse::<u32>()
        .map_err(|_| format!("{context} must be an ISO UTC timestamp"))?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err(format!("{context} must be an ISO UTC timestamp"));
    }
    Ok(())
}

fn current_utc_date() -> Result<String, String> {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock precedes the Unix epoch".to_owned())?
        .as_secs() as i64
        / 86_400;
    let (year, month, day) = civil_from_unix_days(days);
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn civil_from_unix_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    (year + if month <= 2 { 1 } else { 0 }, month, day)
}

pub fn validate_responsibilities(record: &Value) -> Result<(), String> {
    if normalize_responsibility_inventory(record)? != *record {
        return Err("responsibility inventory is not normalized by stable ID".to_owned());
    }
    let root = object(record, "responsibility inventory must be a JSON object")?;
    require_exact_keys(root, &INVENTORY_ROOT_FIELDS, "responsibility inventory")?;
    require_string(
        root,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
    )?;
    require_string(
        root,
        "$id",
        "https://vexil.dev/release/stewardship/responsibilities.json",
    )?;
    require_string(
        root,
        "inventorySchema",
        "https://vexil.dev/release/schemas/retired-bot-responsibility.schema.json",
    )?;
    require_string(root, "version", "1.0")?;

    let configuration = object(
        required_value(root, "historicalConfiguration")?,
        "historical configuration",
    )?;
    require_exact_keys(
        configuration,
        &["source", "status", "nonAuthorityStatement"],
        "historical configuration",
    )?;
    require_string(configuration, "source", ".vexilbot.toml")?;
    require_string(configuration, "status", "retired-historical-evidence")?;
    if !text(
        configuration.get("nonAuthorityStatement"),
        "historical configuration non-authority statement",
    )?
    .contains("not an order or Release Unit membership source")
    {
        return Err(
            "retired configuration cannot be an order or Release Unit membership source".to_owned(),
        );
    }

    let comparison = object(
        required_value(root, "manifestComparison")?,
        "manifest comparison",
    )?;
    require_exact_keys(
        comparison,
        &[
            "retiredConfigurationSource",
            "nonAuthorityStatement",
            "retiredConfiguredUnits",
            "catalogedSourceUnits",
            "mismatches",
        ],
        "manifest comparison",
    )?;
    require_string(comparison, "retiredConfigurationSource", ".vexilbot.toml")?;
    if !text(
        comparison.get("nonAuthorityStatement"),
        "manifest comparison non-authority statement",
    )?
    .contains("not an order or Release Unit membership source")
    {
        return Err(
            "retired configuration cannot be used as membership or order authority".to_owned(),
        );
    }
    let mismatches = array(comparison.get("mismatches"), "manifest mismatches")?;
    if !mismatches.iter().any(|mismatch| {
        object(mismatch, "manifest mismatch")
            .ok()
            .and_then(|entry| entry.get("unit"))
            .and_then(Value::as_str)
            == Some("crates/vexil-codegen-py")
    }) {
        return Err("manifest comparison is missing vexil-codegen-py discrepancy".to_owned());
    }
    for mismatch in mismatches {
        let mismatch = object(mismatch, "manifest mismatch")?;
        require_exact_keys(
            mismatch,
            &["id", "unit", "kind", "observedBehavior"],
            "manifest mismatch",
        )?;
        for field in ["id", "unit", "kind", "observedBehavior"] {
            if text(mismatch.get(field), field)?.is_empty() {
                return Err(format!("manifest mismatch has empty {field}"));
            }
        }
    }

    let responsibilities = array(root.get("responsibilities"), "responsibilities")?;
    let mut ids = BTreeSet::new();
    let mut classes = BTreeSet::new();
    let mut previous_id = "";
    for responsibility in responsibilities {
        let responsibility = object(responsibility, "responsibility")?;
        let expected_fields: &[&str] =
            if responsibility.get("privilegeClass").and_then(Value::as_str) == Some("advisory") {
                &RESPONSIBILITY_FIELDS
            } else {
                &[
                    "id",
                    "responsibilityClass",
                    "description",
                    "privilegeClass",
                    "historicalEvidence",
                    "affectedSurfaces",
                    "failureImpact",
                    "decisionOwner",
                    "dispositionStatus",
                    "privilegedDispositionId",
                ]
            };
        require_exact_keys(responsibility, expected_fields, "responsibility")?;
        let id = text(responsibility.get("id"), "responsibility id")?;
        if id.is_empty() || !ids.insert(id) {
            return Err(format!("duplicate responsibility ID: {id}"));
        }
        if id <= previous_id {
            return Err("responsibility inventory is not normalized by stable ID".to_owned());
        }
        previous_id = id;
        let class = text(
            responsibility.get("responsibilityClass"),
            "responsibility class",
        )?;
        classes.insert(class);
        if !REQUIRED_RESPONSIBILITY_CLASSES.contains(&class) {
            return Err(format!("unknown responsibility class: {class}"));
        }
        let privilege = text(responsibility.get("privilegeClass"), "privilege class")?;
        if !PRIVILEGE_CLASSES.contains(&privilege) {
            return Err(format!("unknown privilege class: {privilege}"));
        }
        for field in ["description", "failureImpact", "decisionOwner"] {
            if text(responsibility.get(field), field)?.is_empty() {
                return Err(format!("responsibility {id} has empty {field}"));
            }
        }
        let disposition_status = text(
            responsibility.get("dispositionStatus"),
            "disposition status",
        )?;
        if privilege == "advisory" {
            validate_advisory_disposition(responsibility, id, disposition_status)?;
        } else {
            require_string(
                responsibility,
                "dispositionStatus",
                "owned-fail-closed-procedure",
            )?;
            if text(
                responsibility.get("privilegedDispositionId"),
                "privileged disposition ID",
            )?
            .is_empty()
            {
                return Err(format!(
                    "privileged/policy responsibility {id} needs exactly one privileged disposition"
                ));
            }
            if responsibility.contains_key("advisoryDisposition") {
                return Err(format!(
                    "privileged/policy responsibility {id} cannot use an advisory disposition"
                ));
            }
        }
        if array(responsibility.get("affectedSurfaces"), "affected surfaces")?.is_empty() {
            return Err(format!("responsibility {id} has no affected surfaces"));
        }
        let evidence = array(
            responsibility.get("historicalEvidence"),
            "historical evidence",
        )?;
        if evidence.is_empty() {
            return Err(format!("responsibility {id} has no historical evidence"));
        }
        for evidence in evidence {
            let evidence = object(evidence, "historical evidence")?;
            require_exact_keys(
                evidence,
                &["source", "observedBehavior"],
                "historical evidence",
            )?;
            let source = text(evidence.get("source"), "evidence source")?;
            if source.is_empty()
                || text(evidence.get("observedBehavior"), "observed behavior")?.is_empty()
            {
                return Err(format!("responsibility {id} has incomplete evidence"));
            }
            if source.contains("restricted-workspace-reference") {
                return Err(
                    "restricted workspace sources cannot be public responsibility evidence"
                        .to_owned(),
                );
            }
            ensure_no_private_leakage(source)?;
        }
    }
    for class in REQUIRED_RESPONSIBILITY_CLASSES {
        if !classes.contains(class) {
            return Err(format!("known responsibility class is missing: {class}"));
        }
    }
    let normalization = object(required_value(root, "normalization")?, "normalization")?;
    require_exact_keys(
        normalization,
        &["ordering", "duplicatePolicy"],
        "normalization",
    )?;
    require_string(normalization, "ordering", "stable-id-ascending")?;
    require_string(
        normalization,
        "duplicatePolicy",
        "reject-conflicting-duplicates",
    )?;
    ensure_no_private_leakage(&record.to_string())
}

fn validate_catalog_comparison(root: &Path, record: &Value) -> Result<(), String> {
    let comparison = object(
        required_value(
            object(record, "responsibility inventory")?,
            "manifestComparison",
        )?,
        "manifest comparison",
    )?;
    let actual: BTreeSet<_> = strings(
        comparison.get("catalogedSourceUnits"),
        "cataloged source units",
    )?
    .into_iter()
    .collect();
    let declared = array(
        comparison.get("catalogedSourceUnits"),
        "cataloged source units",
    )?;
    if actual.len() != declared.len() {
        return Err("cataloged source units must not contain duplicates".to_owned());
    }
    let catalog = read_json(&root.join("release/catalog.json"))?;
    let units = array(catalog.get("units"), "release catalog units")?;
    let mut expected = BTreeSet::new();
    for unit in units {
        let unit = object(unit, "release catalog unit")?;
        let source_root = text(unit.get("sourceRoot"), "release catalog source root")?;
        let status = text(
            object(required_value(unit, "publication")?, "catalog publication")?.get("status"),
            "catalog publication status",
        )?;
        if status != "non-publishable"
            && (source_root.starts_with("crates/") || source_root.starts_with("packages/"))
        {
            expected.insert(source_root);
        }
    }
    if actual != expected {
        return Err(
            "cataloged source-unit comparison must exactly match maintained catalog source roots"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_advisory_disposition(
    responsibility: &Map<String, Value>,
    id: &str,
    disposition_status: &str,
) -> Result<(), String> {
    if !ADVISORY_DISPOSITIONS.contains(&disposition_status) {
        return Err(format!(
            "advisory responsibility {id} must have exactly one disposition"
        ));
    }
    let disposition = object(
        required_value(responsibility, "advisoryDisposition")?,
        "advisory disposition",
    )?;
    let kind = text(disposition.get("kind"), "advisory disposition kind")?;
    if kind != disposition_status || !ADVISORY_DISPOSITIONS.contains(&kind) {
        return Err(format!(
            "advisory responsibility {id} has an unknown or mismatched disposition"
        ));
    }
    for field in [
        "owner",
        "rationale",
        "minimumPermissions",
        "auditEvidence",
        "failureBehavior",
        "fallback",
        "nonAuthorityBoundary",
    ] {
        if !disposition.contains_key(field) {
            return Err(format!(
                "advisory responsibility {id} has incomplete disposition"
            ));
        }
    }
    validate_assignment_reference(
        required_value(disposition, "owner")?,
        "advisory disposition owner",
    )?;
    let owner = object(
        required_value(disposition, "owner")?,
        "advisory disposition owner",
    )?;
    if text(owner.get("actorId"), "advisory owner actor")?
        != text(responsibility.get("decisionOwner"), "decision owner")?
    {
        return Err(format!(
            "advisory responsibility {id} owner must match decision owner"
        ));
    }
    for field in [
        "rationale",
        "auditEvidence",
        "failureBehavior",
        "nonAuthorityBoundary",
    ] {
        if text(disposition.get(field), field)?.is_empty() {
            return Err(format!("advisory responsibility {id} has empty {field}"));
        }
    }
    ensure_no_private_leakage(text(
        disposition.get("auditEvidence"),
        "advisory audit evidence",
    )?)?;
    let boundary = text(
        disposition.get("nonAuthorityBoundary"),
        "non-authority boundary",
    )?;
    for required in [
        "scope",
        "version",
        "risk",
        "Manifest",
        "privileged gate",
        "publication",
    ] {
        if !boundary.contains(required) {
            return Err(format!(
                "advisory responsibility {id} non-authority boundary is incomplete"
            ));
        }
    }
    let permissions = strings(
        disposition.get("minimumPermissions"),
        "advisory minimum permissions",
    )?;
    let permission_set: BTreeSet<_> = permissions.iter().copied().collect();
    if permission_set.len() != permissions.len()
        || !permission_set
            .iter()
            .all(|permission| ADVISORY_PERMISSION_INTENTS.contains(permission))
    {
        return Err(format!(
            "advisory responsibility {id} requests prohibited permission intent"
        ));
    }
    let fallback = object(
        required_value(disposition, "fallback")?,
        "advisory fallback",
    )?;
    require_exact_keys(
        fallback,
        &[
            "decision",
            "owner",
            "evidenceDestination",
            "noPrivilegeBoundary",
        ],
        "advisory fallback",
    )?;
    if !["perform-manually", "defer"]
        .contains(&text(fallback.get("decision"), "fallback decision")?)
    {
        return Err(format!(
            "advisory responsibility {id} fallback is not perform/defer"
        ));
    }
    validate_assignment_reference(required_value(fallback, "owner")?, "fallback owner")?;
    for field in ["evidenceDestination", "noPrivilegeBoundary"] {
        if text(fallback.get(field), field)?.is_empty() {
            return Err(format!(
                "advisory responsibility {id} fallback is incomplete"
            ));
        }
    }
    if !text(fallback.get("noPrivilegeBoundary"), "fallback boundary")?
        .contains("no privileged access")
    {
        return Err(format!(
            "advisory responsibility {id} fallback must have no privileged access"
        ));
    }
    match kind {
        "maintained-replacement" => {
            require_exact_keys(
                disposition,
                &[
                    "kind",
                    "owner",
                    "rationale",
                    "minimumPermissions",
                    "auditEvidence",
                    "failureBehavior",
                    "fallback",
                    "nonAuthorityBoundary",
                    "automation",
                ],
                "maintained advisory replacement",
            )?;
            let automation = object(
                required_value(disposition, "automation")?,
                "advisory automation",
            )?;
            require_exact_keys(
                automation,
                &[
                    "source",
                    "deploymentState",
                    "trigger",
                    "inputs",
                    "permissionIntents",
                    "auditSurface",
                    "noLiveEffects",
                ],
                "advisory automation",
            )?;
            for field in ["source", "trigger", "auditSurface"] {
                if text(automation.get(field), field)?.is_empty() {
                    return Err(format!(
                        "advisory responsibility {id} automation is incomplete"
                    ));
                }
            }
            ensure_no_private_leakage(text(automation.get("source"), "automation source")?)?;
            require_string(automation, "deploymentState", "not-deployed")?;
            if automation.get("noLiveEffects").and_then(Value::as_bool) != Some(true) {
                return Err(format!(
                    "advisory responsibility {id} automation must have no live effects"
                ));
            }
            if array(automation.get("inputs"), "automation inputs")?.is_empty() {
                return Err(format!(
                    "advisory responsibility {id} automation needs inputs"
                ));
            }
            let automation_permissions = strings(
                automation.get("permissionIntents"),
                "automation permission intents",
            )?;
            if automation_permissions.is_empty()
                || automation_permissions != permissions
                || !automation_permissions
                    .iter()
                    .all(|permission| ADVISORY_PERMISSION_INTENTS.contains(permission))
            {
                return Err(format!(
                    "advisory responsibility {id} automation has non-minimal permissions"
                ));
            }
        }
        "owned-manual-procedure" => {
            require_exact_keys(
                disposition,
                &[
                    "kind",
                    "owner",
                    "rationale",
                    "minimumPermissions",
                    "auditEvidence",
                    "failureBehavior",
                    "fallback",
                    "nonAuthorityBoundary",
                    "manualProcedure",
                ],
                "owned manual advisory procedure",
            )?;
            if !permissions.is_empty() {
                return Err(format!(
                    "advisory responsibility {id} manual procedure needs no automation permissions"
                ));
            }
            let procedure = object(
                required_value(disposition, "manualProcedure")?,
                "manual advisory procedure",
            )?;
            require_exact_keys(
                procedure,
                &["decision", "evidenceDestination", "noPrivilegeBoundary"],
                "manual advisory procedure",
            )?;
            require_string(procedure, "decision", "perform-or-defer-manually")?;
            if text(
                procedure.get("evidenceDestination"),
                "manual procedure evidence",
            )?
            .is_empty()
                || !text(
                    procedure.get("noPrivilegeBoundary"),
                    "manual procedure boundary",
                )?
                .contains("no privileged access")
            {
                return Err(format!(
                    "advisory responsibility {id} manual procedure is incomplete"
                ));
            }
        }
        "approved-retirement" => {
            require_exact_keys(
                disposition,
                &[
                    "kind",
                    "owner",
                    "rationale",
                    "minimumPermissions",
                    "auditEvidence",
                    "failureBehavior",
                    "fallback",
                    "nonAuthorityBoundary",
                    "retirement",
                ],
                "approved advisory retirement",
            )?;
            if !permissions.is_empty() {
                return Err(format!(
                    "advisory responsibility {id} retirement needs no automation permissions"
                ));
            }
            let retirement = object(
                required_value(disposition, "retirement")?,
                "advisory retirement",
            )?;
            require_exact_keys(
                retirement,
                &[
                    "publicDecision",
                    "lostBehavior",
                    "impact",
                    "residualRisk",
                    "approverActorId",
                ],
                "advisory retirement",
            )?;
            let decision = object(
                required_value(retirement, "publicDecision")?,
                "retirement public decision",
            )?;
            require_exact_keys(
                decision,
                &["id", "source", "status"],
                "retirement public decision",
            )?;
            require_string(decision, "status", "accepted")?;
            let source = text(decision.get("source"), "retirement public decision source")?;
            if !source.starts_with("docs/") {
                return Err(format!(
                    "advisory responsibility {id} retirement needs public decision evidence"
                ));
            }
            ensure_no_private_leakage(source)?;
            for field in ["lostBehavior", "impact", "residualRisk", "approverActorId"] {
                if text(retirement.get(field), field)?.is_empty() {
                    return Err(format!(
                        "advisory responsibility {id} retirement is incomplete"
                    ));
                }
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn validate_assignment_reference(value: &Value, context: &str) -> Result<(), String> {
    let reference = object(value, context)?;
    require_exact_keys(reference, &["actorId", "roleId", "assignmentId"], context)?;
    for field in ["actorId", "roleId", "assignmentId"] {
        if text(reference.get(field), field)?.is_empty() {
            return Err(format!("{context} {field} cannot be empty"));
        }
    }
    if !text(reference.get("actorId"), "assignment actor")?.starts_with("github:")
        || !REQUIRED_ROLE_IDS.contains(&text(reference.get("roleId"), "assignment role")?)
    {
        return Err(format!("{context} must name a stewardship role assertion"));
    }
    Ok(())
}

fn validate_advisory_owners(record: &Value, assignments: &Value) -> Result<(), String> {
    let assignment_rows = array(
        assignments.get("assignments"),
        "assignment record assignments",
    )?;
    let known: BTreeSet<_> = assignment_rows
        .iter()
        .filter_map(|row| {
            let row = row.as_object()?;
            Some((
                row.get("assignmentId")?.as_str()?,
                row.get("primaryActorId")?.as_str()?,
                row.get("roleId")?.as_str()?,
            ))
        })
        .collect();
    for responsibility in array(record.get("responsibilities"), "responsibilities")? {
        let responsibility = object(responsibility, "responsibility")?;
        if responsibility.get("privilegeClass").and_then(Value::as_str) != Some("advisory") {
            continue;
        }
        let disposition = object(
            required_value(responsibility, "advisoryDisposition")?,
            "advisory disposition",
        )?;
        for (label, reference) in [
            ("owner", required_value(disposition, "owner")?),
            (
                "fallback owner",
                required_value(
                    object(required_value(disposition, "fallback")?, "fallback")?,
                    "owner",
                )?,
            ),
        ] {
            let reference = object(reference, label)?;
            let triple = (
                text(reference.get("assignmentId"), "assignment id")?,
                text(reference.get("actorId"), "assignment actor")?,
                text(reference.get("roleId"), "assignment role")?,
            );
            if !known.contains(&triple) {
                return Err(format!(
                    "advisory {label} does not resolve to a stewardship assignment"
                ));
            }
        }
    }
    Ok(())
}

fn validate_advisory_contract(root: &Path, record: &Value) -> Result<(), String> {
    let contract = read_json(&root.join("release/advisory/automation-contract.json"))?;
    let contract = object(&contract, "advisory automation contract")?;
    require_exact_keys(
        contract,
        &["$schema", "id", "status", "purpose", "contracts"],
        "advisory automation contract",
    )?;
    require_string(
        contract,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
    )?;
    require_string(contract, "id", "advisory-automation-contract-2026-07-14")?;
    require_string(contract, "status", "not-deployed")?;
    let contracts = array(contract.get("contracts"), "advisory automation contracts")?;
    let mut by_id = HashMap::new();
    let mut by_effects = HashMap::new();
    for entry in contracts {
        let entry = object(entry, "advisory automation contract entry")?;
        require_exact_keys(
            entry,
            &[
                "id",
                "allowedPermissionIntents",
                "effects",
                "prohibitedEffects",
            ],
            "advisory automation contract entry",
        )?;
        let id = text(entry.get("id"), "advisory automation contract id")?;
        let permissions = strings(
            entry.get("allowedPermissionIntents"),
            "advisory automation contract permissions",
        )?;
        if permissions.is_empty()
            || !permissions
                .iter()
                .all(|permission| ADVISORY_PERMISSION_INTENTS.contains(permission))
        {
            return Err("advisory automation contract has prohibited permission intent".to_owned());
        }
        let prohibited = strings(
            entry.get("prohibitedEffects"),
            "advisory automation prohibited effects",
        )?;
        let effects = strings(entry.get("effects"), "advisory automation effects")?;
        if effects.is_empty()
            || !effects
                .iter()
                .all(|effect| ADVISORY_EFFECTS.contains(effect))
            || effects.iter().any(|effect| prohibited.contains(effect))
        {
            return Err("advisory automation contract has a non-advisory effect".to_owned());
        }
        for effect in [
            "select-scope",
            "select-version",
            "accept-risk",
            "approve-manifest",
            "satisfy-privileged-gate",
            "trigger-publication",
        ] {
            if !prohibited.contains(&effect) {
                return Err(
                    "advisory automation contract must fail closed for authority effects"
                        .to_owned(),
                );
            }
        }
        if by_id.insert(id, permissions).is_some() || by_effects.insert(id, effects).is_some() {
            return Err("duplicate advisory automation contract ID".to_owned());
        }
    }
    for responsibility in array(record.get("responsibilities"), "responsibilities")? {
        let responsibility = object(responsibility, "responsibility")?;
        if responsibility
            .get("dispositionStatus")
            .and_then(Value::as_str)
            != Some("maintained-replacement")
        {
            continue;
        }
        let disposition = object(
            required_value(responsibility, "advisoryDisposition")?,
            "advisory disposition",
        )?;
        let automation = object(
            required_value(disposition, "automation")?,
            "advisory automation",
        )?;
        let source = text(automation.get("source"), "advisory automation source")?;
        let contract_id = source
            .strip_prefix("release/advisory/automation-contract.json#")
            .ok_or_else(|| {
                "advisory automation source must reference the repository-owned contract".to_owned()
            })?;
        let declared = by_id.get(contract_id).ok_or_else(|| {
            "advisory automation source references an unknown contract".to_owned()
        })?;
        let expected_contract = match text(responsibility.get("id"), "responsibility ID")? {
            "RBR-005" => "triage-routing",
            "RBR-006" => "label-routing",
            _ => {
                return Err(
                    "maintained advisory replacement has no approved contract mapping".to_owned(),
                )
            }
        };
        if contract_id != expected_contract {
            return Err("advisory replacement is bound to the wrong behavior contract".to_owned());
        }
        let expected_effects: &[&str] = match contract_id {
            "triage-routing" => &["advisory-route", "maintainer-review-note"],
            "label-routing" => &["advisory-label"],
            _ => unreachable!("contract ID was validated above"),
        };
        let actual_effects = by_effects.get(contract_id).ok_or_else(|| {
            "advisory automation source references a contract without effects".to_owned()
        })?;
        if actual_effects.as_slice() != expected_effects {
            return Err("advisory replacement contract declares the wrong behavior".to_owned());
        }
        if strings(
            automation.get("permissionIntents"),
            "automation permissions",
        )? != *declared
        {
            return Err(
                "advisory automation permissions differ from the repository-owned contract"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

fn validate_responsibility_audit_surfaces(root: &Path, record: &Value) -> Result<(), String> {
    for responsibility in array(record.get("responsibilities"), "responsibilities")? {
        let responsibility = object(responsibility, "responsibility")?;
        let Some(disposition) = responsibility.get("advisoryDisposition") else {
            continue;
        };
        let disposition = object(disposition, "advisory disposition")?;
        validate_public_markdown_reference(
            root,
            text(disposition.get("auditEvidence"), "advisory audit evidence")?,
            "advisory audit evidence",
        )?;
        let fallback = object(
            required_value(disposition, "fallback")?,
            "advisory fallback",
        )?;
        validate_public_markdown_reference(
            root,
            text(
                fallback.get("evidenceDestination"),
                "advisory fallback evidence",
            )?,
            "advisory fallback evidence",
        )?;
        if let Some(automation) = disposition.get("automation") {
            let automation = object(automation, "advisory automation")?;
            validate_public_markdown_reference(
                root,
                text(
                    automation.get("auditSurface"),
                    "advisory automation audit surface",
                )?,
                "advisory automation audit surface",
            )?;
        }
    }
    Ok(())
}

fn validate_privileged_audit_surfaces(root: &Path, record: &Value) -> Result<(), String> {
    let operations = array(record.get("operations"), "privileged operations")?;
    for operation in operations {
        let operation = object(operation, "privileged operation")?;
        validate_public_markdown_reference(
            root,
            text(operation.get("auditSurface"), "privileged audit surface")?,
            "privileged audit surface",
        )?;
    }
    Ok(())
}

fn validate_public_markdown_reference(
    root: &Path,
    reference: &str,
    label: &str,
) -> Result<(), String> {
    ensure_no_private_leakage(reference)?;
    let (relative, fragment) = reference
        .split_once('#')
        .ok_or_else(|| format!("{label} must identify a public Markdown fragment"))?;
    if relative.is_empty()
        || fragment.is_empty()
        || !relative.starts_with("docs/book/src/release/")
        || relative.starts_with('/')
        || relative.contains('\\')
        || relative
            .split('/')
            .any(|part| part == ".." || part.is_empty())
    {
        return Err(format!(
            "{label} is not a safe public documentation reference"
        ));
    }
    let markdown = fs::read_to_string(root.join(relative))
        .map_err(|error| format!("read {label}: {error}"))?;
    let anchor = format!("<a id=\"{}\"></a>", fragment.to_ascii_lowercase());
    if !markdown.to_ascii_lowercase().contains(&anchor) {
        return Err(format!(
            "{label} does not resolve to a public Markdown anchor"
        ));
    }
    Ok(())
}

pub fn validate_privileged_operations(
    record: &Value,
    responsibilities: &Value,
    assignments: &Value,
) -> Result<(), String> {
    let root = object(record, "privileged operations contract")?;
    require_exact_keys(
        root,
        &PRIVILEGED_OPERATION_ROOT_FIELDS,
        "privileged operations contract",
    )?;
    require_string(
        root,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
    )?;
    require_string(
        root,
        "$id",
        "https://vexil.dev/release/privileged/operations-contract.json",
    )?;
    require_string(root, "version", "1.0")?;
    require_string(
        root,
        "inventorySource",
        "release/stewardship/responsibilities.json",
    )?;
    let non_authority = text(root.get("nonAuthorityStatement"), "non-authority statement")?;
    for prohibited_source in [
        "Historical bot configuration",
        "historical behavior",
        "green CI",
        "tags",
        "provider approval",
        "CODEOWNERS",
        "private planning artifacts",
    ] {
        if !non_authority.contains(prohibited_source) {
            return Err(
                "privileged operations must reject stale or non-authoritative release authority"
                    .to_owned(),
            );
        }
    }
    let known_assignments: BTreeSet<_> = array(
        assignments.get("assignments"),
        "assignment record assignments",
    )?
    .iter()
    .filter_map(|row| {
        let row = row.as_object()?;
        Some((
            row.get("assignmentId")?.as_str()?,
            row.get("primaryActorId")?.as_str()?,
            row.get("roleId")?.as_str()?,
        ))
    })
    .collect();
    let expected: HashMap<_, _> = array(
        responsibilities.get("responsibilities"),
        "responsibility inventory responsibilities",
    )?
    .iter()
    .filter_map(|row| {
        let row = row.as_object()?;
        let privilege = row.get("privilegeClass")?.as_str()?;
        if privilege == "advisory" {
            return None;
        }
        Some((
            row.get("id")?.as_str()?,
            (
                privilege,
                row.get("decisionOwner")?.as_str()?,
                row.get("privilegedDispositionId")?.as_str()?,
            ),
        ))
    })
    .collect();
    let operations = array(root.get("operations"), "privileged operations")?;
    let mut operation_ids = BTreeSet::new();
    let mut responsibility_ids = BTreeSet::new();
    for operation in operations {
        let operation = object(operation, "privileged operation")?;
        require_exact_keys(
            operation,
            &PRIVILEGED_OPERATION_FIELDS,
            "privileged operation",
        )?;
        let id = text(operation.get("id"), "privileged operation id")?;
        if id.is_empty() || !operation_ids.insert(id) {
            return Err("privileged operation IDs must be stable and unique".to_owned());
        }
        let responsibility_id = text(
            operation.get("responsibilityId"),
            "operation responsibility ID",
        )?;
        let (expected_class, expected_owner, expected_operation_id) =
            expected.get(responsibility_id).ok_or_else(|| {
                "privileged operation uses an advisory or unknown responsibility".to_owned()
            })?;
        if id != *expected_operation_id || !responsibility_ids.insert(responsibility_id) {
            return Err("every privileged/policy responsibility must map exactly once".to_owned());
        }
        require_string(operation, "kind", "owned-fail-closed-procedure")?;
        require_string(operation, "authorityClass", expected_class)?;
        let expected_role = expected_privileged_owner_role(responsibility_id)?;
        validate_operation_owner(
            operation.get("owner"),
            &known_assignments,
            expected_owner,
            expected_role,
        )?;
        validate_operation_target(operation.get("target"))?;
        let target = object(
            required_value(operation, "target")?,
            "privileged operation target",
        )?;
        let target_identity = text(target.get("identity"), "target identity")?;
        let owner = object(
            required_value(operation, "owner")?,
            "privileged operation owner",
        )?;
        let owner_actor = text(owner.get("actorId"), "operation owner actor")?;
        let permissions = strings(operation.get("minimumPermissions"), "minimum permissions")?;
        if permissions.is_empty()
            || permissions
                .iter()
                .any(|permission| !is_narrow_privileged_permission(permission))
        {
            return Err(
                "privileged operation requests a broad personal credential or permission"
                    .to_owned(),
            );
        }
        for field in [
            "auditSurface",
            "hybridBoundary",
            "preEffectStopCondition",
            "failureBehavior",
            "fallback",
            "effectPolicy",
        ] {
            if text(operation.get(field), field)?.is_empty() {
                return Err(format!("privileged operation {id} has empty {field}"));
            }
        }
        let boundary = text(operation.get("hybridBoundary"), "hybrid boundary")?;
        if !boundary.contains("Advisory stages receive no privileged environment or credential")
            || !boundary.contains("approved immutable inputs")
        {
            return Err("advisory and privileged stages must remain isolated".to_owned());
        }
        validate_operation_inputs(operation.get("requiredInputs"), target_identity)?;
        validate_operation_authentication(
            operation.get("authentication"),
            target_identity,
            owner_actor,
        )?;
        require_string(operation, "currentReadiness", "blocked")?;
        if strings(
            operation.get("blockingPrerequisites"),
            "blocking prerequisites",
        )?
        .is_empty()
        {
            return Err("blocked privileged operation must retain visible blockers".to_owned());
        }
        if !text(
            operation.get("preEffectStopCondition"),
            "pre-effect stop condition",
        )?
        .to_ascii_lowercase()
        .contains("before")
            || !text(operation.get("failureBehavior"), "failure behavior")?
                .contains("no effect event or external effect")
            || !text(operation.get("effectPolicy"), "effect policy")?
                .contains("No effect is authorized while currentReadiness is blocked")
        {
            return Err("failed readiness must retain the blocker with no effect".to_owned());
        }
    }
    if responsibility_ids.len() != expected.len() {
        return Err("every privileged/policy responsibility must map exactly once".to_owned());
    }
    ensure_no_private_leakage(&record.to_string())
}

fn validate_operation_owner(
    value: Option<&Value>,
    known_assignments: &BTreeSet<(&str, &str, &str)>,
    expected_owner: &str,
    expected_role: &str,
) -> Result<(), String> {
    let value = value.ok_or_else(|| "privileged operation owner is missing".to_owned())?;
    validate_assignment_reference(value, "privileged operation owner")?;
    let owner = object(value, "privileged operation owner")?;
    let triple = (
        text(owner.get("assignmentId"), "operation owner assignment")?,
        text(owner.get("actorId"), "operation owner actor")?,
        text(owner.get("roleId"), "operation owner role")?,
    );
    if triple.1 != expected_owner
        || triple.2 != expected_role
        || !known_assignments.contains(&triple)
    {
        return Err(
            "privileged operation owner does not resolve to the reviewed assignment".to_owned(),
        );
    }
    Ok(())
}

fn expected_privileged_owner_role(responsibility_id: &str) -> Result<&'static str, String> {
    match responsibility_id {
        "RBR-003" | "RBR-004" => Ok("release-steward"),
        "RBR-008" => Ok("security-steward"),
        "RBR-009" => Ok("repository-administrator"),
        _ => Err("privileged responsibility has no approved role boundary".to_owned()),
    }
}

fn is_narrow_privileged_permission(permission: &str) -> bool {
    matches!(
        permission,
        "publish:exact-approved-release-unit" | "repository-metadata:read"
    ) || permission
        .strip_prefix("contents:write:refs/tags/")
        .is_some_and(|reference| {
            reference.starts_with("exact-approved-manifest-")
                && !reference.is_empty()
                && !reference.contains('*')
        })
}

fn validate_operation_target(value: Option<&Value>) -> Result<(), String> {
    let target = object(
        value.ok_or_else(|| "privileged operation target is missing".to_owned())?,
        "privileged operation target",
    )?;
    require_exact_keys(
        target,
        &["identity", "protectedAuthority"],
        "privileged operation target",
    )?;
    for field in ["identity", "protectedAuthority"] {
        if text(target.get(field), field)?.is_empty() {
            return Err(
                "privileged operation needs a target-specific protected identity".to_owned(),
            );
        }
    }
    if text(target.get("identity"), "target identity")?.contains('*') {
        return Err(
            "privileged operation target must be exact-manifest-bound, never wildcarded".to_owned(),
        );
    }
    Ok(())
}

fn validate_operation_inputs(value: Option<&Value>, target_identity: &str) -> Result<(), String> {
    let inputs = object(
        value.ok_or_else(|| "privileged operation inputs are missing".to_owned())?,
        "privileged operation inputs",
    )?;
    require_exact_keys(
        inputs,
        &[
            "manifestDigest",
            "releaseStewardApproval",
            "targetIdentity",
            "currentManifest",
            "releaseUnitCatalogEdges",
            "futureControls",
            "immutableCandidateInputs",
        ],
        "privileged operation inputs",
    )?;
    for field in ["manifestDigest", "releaseStewardApproval", "targetIdentity"] {
        if text(inputs.get(field), field)?.is_empty() {
            return Err(
                "potential effects require a manifest digest, approval, and target identity"
                    .to_owned(),
            );
        }
    }
    if text(inputs.get("targetIdentity"), "target identity")? != target_identity {
        return Err("required target identity must match the operation target".to_owned());
    }
    validate_pending_evidence(
        inputs.get("currentManifest"),
        "canonical-release-manifest",
        "current Manifest",
    )?;
    validate_pending_evidence(
        inputs.get("releaseUnitCatalogEdges"),
        "typed-release-unit-catalog-edges",
        "typed Release Unit Catalog edges",
    )?;
    let controls = array(inputs.get("futureControls"), "future controls")?;
    if !controls.iter().any(|control| {
        object(control, "future control")
            .ok()
            .is_some_and(|control| {
                control.get("id").and_then(Value::as_str) == Some("external-controls")
                    && control.get("status").and_then(Value::as_str)
                        == Some("required-not-yet-verified")
            })
    }) {
        return Err("potential effects require typed pending external-control evidence".to_owned());
    }
    if strings(
        inputs.get("immutableCandidateInputs"),
        "immutable candidate inputs",
    )?
    .is_empty()
    {
        return Err("potential effects require immutable later candidate inputs".to_owned());
    }
    Ok(())
}

fn validate_pending_evidence(
    value: Option<&Value>,
    expected_kind: &str,
    label: &str,
) -> Result<(), String> {
    let evidence = object(
        value.ok_or_else(|| format!("potential effects require {label} evidence"))?,
        label,
    )?;
    require_exact_keys(evidence, &["kind", "status"], label)?;
    require_string(evidence, "kind", expected_kind)?;
    require_string(evidence, "status", "required-not-yet-available")
}

fn validate_operation_authentication(
    value: Option<&Value>,
    target_identity: &str,
    owner_actor: &str,
) -> Result<(), String> {
    let authentication = object(
        value.ok_or_else(|| "privileged authentication route is missing".to_owned())?,
        "privileged authentication",
    )?;
    require_exact_keys(
        authentication,
        &[
            "acceptedMechanisms",
            "personalAccessTokens",
            "bootstrapException",
        ],
        "privileged authentication",
    )?;
    let mechanisms = strings(
        authentication.get("acceptedMechanisms"),
        "accepted authentication mechanisms",
    )?;
    if mechanisms.is_empty()
        || mechanisms
            .iter()
            .any(|mechanism| *mechanism != "OIDC" && *mechanism != "provider trusted publishing")
    {
        return Err("privileged operation must require trusted identity or OIDC".to_owned());
    }
    require_string(authentication, "personalAccessTokens", "rejected")?;
    let bootstrap = object(
        authentication
            .get("bootstrapException")
            .ok_or_else(|| "bootstrap exception is missing".to_owned())?,
        "bootstrap exception",
    )?;
    match text(bootstrap.get("status"), "bootstrap exception status")? {
        "not-approved" => require_exact_keys(bootstrap, &["status"], "bootstrap exception"),
        "approved" => {
            require_exact_keys(
                bootstrap,
                &[
                    "status",
                    "targetScope",
                    "custodian",
                    "expiresOn",
                    "revocationPath",
                    "auditSurface",
                ],
                "bootstrap exception",
            )?;
            for field in ["targetScope", "custodian", "revocationPath", "auditSurface"] {
                if text(bootstrap.get(field), field)?.is_empty() {
                    return Err(
                        "approved bootstrap exception must be scoped, revocable, and auditable"
                            .to_owned(),
                    );
                }
            }
            require_date(bootstrap.get("expiresOn"), "bootstrap exception expiry")?;
            if text(bootstrap.get("expiresOn"), "bootstrap exception expiry")?
                <= current_utc_date()?.as_str()
            {
                return Err("bootstrap exception must not be expired".to_owned());
            }
            if text(bootstrap.get("targetScope"), "bootstrap target scope")? != target_identity
                || text(bootstrap.get("custodian"), "bootstrap custodian")? != owner_actor
            {
                return Err(
                    "approved bootstrap exception must bind the operation target and owner"
                        .to_owned(),
                );
            }
            Ok(())
        }
        _ => Err("bootstrap exception must be absent or separately approved".to_owned()),
    }
}

pub fn normalize_responsibility_inventory(record: &Value) -> Result<Value, String> {
    let mut normalized = record.clone();
    let root = normalized
        .as_object_mut()
        .ok_or_else(|| "responsibility inventory must be a JSON object".to_owned())?;
    let responsibilities = root
        .get_mut("responsibilities")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "responsibilities must be an array".to_owned())?;
    responsibilities.sort_by(|left, right| {
        left.get("id")
            .and_then(Value::as_str)
            .cmp(&right.get("id").and_then(Value::as_str))
    });
    let mismatches = root
        .get_mut("manifestComparison")
        .and_then(Value::as_object_mut)
        .and_then(|comparison| comparison.get_mut("mismatches"))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "manifest mismatches must be an array".to_owned())?;
    mismatches.sort_by(|left, right| {
        left.get("id")
            .and_then(Value::as_str)
            .cmp(&right.get("id").and_then(Value::as_str))
    });
    Ok(normalized)
}

pub fn render_responsibility_markdown(record: &Value) -> Result<String, String> {
    let root = object(record, "responsibility inventory")?;
    let responsibilities = array(root.get("responsibilities"), "responsibilities")?;
    let comparison = object(
        required_value(root, "manifestComparison")?,
        "manifest comparison",
    )?;
    let mut markdown = String::from("# Retired-Bot Responsibility Inventory\n\n> Generated view of [`release/stewardship/responsibilities.json`](../../../../release/stewardship/responsibilities.json). The JSON inventory is canonical; this Markdown is non-authoritative and parity-checked.\n\nThe retired [`.vexilbot.toml`](../../../../.vexilbot.toml) is historical evidence only: it is **not an order or Release Unit membership source**. Advisory responsibilities have exactly one public disposition; privileged and policy responsibilities have exactly one owned fail-closed procedure and remain blocked pending later controls.\n\n## Inventory\n\n| ID | Responsibility | Privilege class | Failure impact | Decision owner | Status |\n|---|---|---|---|---|---|\n");
    for responsibility in responsibilities {
        let responsibility = object(responsibility, "responsibility")?;
        markdown.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            text(responsibility.get("id"), "responsibility id")?,
            text(responsibility.get("description"), "description")?,
            text(responsibility.get("privilegeClass"), "privilege class")?,
            text(responsibility.get("failureImpact"), "failure impact")?,
            text(responsibility.get("decisionOwner"), "decision owner")?,
            text(
                responsibility.get("dispositionStatus"),
                "disposition status"
            )?,
        ));
    }
    markdown.push_str("\n## Source-unit comparison\n\nThe source-led [Release Unit Catalog](./catalog.md) determines the maintained-unit inventory, direct version-source observations, and current status. This comparison only records gaps in the retired configuration; it does not make a manifest-bearing component publishable, eligible, ordered, or released.\n\n| Mismatch ID | Unit | Observed historical gap |\n|---|---|---|\n");
    for mismatch in array(comparison.get("mismatches"), "manifest mismatches")? {
        let mismatch = object(mismatch, "manifest mismatch")?;
        markdown.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            text(mismatch.get("id"), "mismatch id")?,
            text(mismatch.get("unit"), "mismatch unit")?,
            text(mismatch.get("observedBehavior"), "mismatch observation")?,
        ));
    }
    markdown.push_str("\n## Evidence and use\n\nEach canonical item carries source-attributed observed behavior and affected public surfaces. The inventory is offline, deterministic, and does not inspect or change provider state. Validation rejects non-public workspace evidence, missing known responsibility classes, duplicate stable IDs, missing evidence or decision owner, unapproved advisory dispositions, forbidden permissions, configuration-as-authority claims, and advisory authority claims.\n\nFor the advisory-only operations view, see [Advisory Automation and Manual Fallbacks](./advisory-automation.md). For privileged and policy blockers, see [Privileged and Policy Operations](./privileged-operations.md).\n\n## Validation\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nThe command validates the canonical inventory and its generated mdBook view without network access or provider effects.\n");
    Ok(markdown)
}

pub fn validate_responsibility_documentation_parity(
    record: &Value,
    documentation: &str,
) -> Result<(), String> {
    if documentation != render_responsibility_markdown(record)? {
        return Err("documentation parity failure: docs/book/src/release/retired-bot-responsibilities.md is stale".to_owned());
    }
    Ok(())
}

pub fn render_advisory_runbook_markdown(record: &Value) -> Result<String, String> {
    let responsibilities = array(record.get("responsibilities"), "responsibilities")?;
    let mut markdown = String::from("# Advisory Automation and Manual Fallbacks\n\nThis runbook is generated from [`release/stewardship/responsibilities.json`](../stewardship/responsibilities.json). It is public guidance, not an approval, Manifest, release control plane, or provider configuration. All entries are offline declarations with no deployed automation and no live effects.\n\n## Operating boundary\n\nAdvice may identify, triage, label, comment, or report. It cannot select scope or version, accept risk, approve a Manifest, satisfy a privileged gate, trigger publication, change protected branches, access environments or credentials, or create release authority. If an advisory mechanism is unavailable, its named owner must perform the stated manual fallback or defer and record evidence; the fallback has no privileged access.\n\n## Advisory dispositions\n\n| ID | Disposition | Owner role assertion | Minimum permissions | Failure behavior | Manual fallback |\n|---|---|---|---|---|---|\n");
    for responsibility in responsibilities {
        let responsibility = object(responsibility, "responsibility")?;
        markdown.push_str(&format!(
            "<a id=\"{}\"></a>\n",
            text(responsibility.get("id"), "responsibility ID")?.to_ascii_lowercase()
        ));
    }
    for responsibility in responsibilities {
        let responsibility = object(responsibility, "responsibility")?;
        if text(responsibility.get("privilegeClass"), "privilege class")? != "advisory" {
            continue;
        }
        let disposition = object(
            required_value(responsibility, "advisoryDisposition")?,
            "advisory disposition",
        )?;
        let owner = object(required_value(disposition, "owner")?, "advisory owner")?;
        let fallback = object(
            required_value(disposition, "fallback")?,
            "advisory fallback",
        )?;
        markdown.push_str(&format!(
            "| `{}` | {} | `{}` (`{}`) | {} | {} | {} by `{}` |\n",
            text(responsibility.get("id"), "responsibility id")?,
            text(disposition.get("kind"), "advisory kind")?,
            text(owner.get("roleId"), "owner role")?,
            text(owner.get("assignmentId"), "owner assignment")?,
            strings(disposition.get("minimumPermissions"), "minimum permissions")?.join(", "),
            text(disposition.get("failureBehavior"), "failure behavior")?,
            text(fallback.get("decision"), "fallback decision")?,
            text(
                object(required_value(fallback, "owner")?, "fallback owner")?.get("assignmentId"),
                "fallback assignment"
            )?,
        ));
    }
    markdown.push_str("\n## Retirement evidence\n\n");
    for responsibility in responsibilities {
        let responsibility = object(responsibility, "responsibility")?;
        if responsibility
            .get("dispositionStatus")
            .and_then(Value::as_str)
            != Some("approved-retirement")
        {
            continue;
        }
        let disposition = object(
            required_value(responsibility, "advisoryDisposition")?,
            "advisory disposition",
        )?;
        let retirement = object(required_value(disposition, "retirement")?, "retirement")?;
        let decision = object(
            required_value(retirement, "publicDecision")?,
            "public decision",
        )?;
        markdown.push_str(&format!(
            "- `{}`: decision `{}` is **{}** at `{}`; approver `{}`. Lost behavior: {} Residual risk: {}\n",
            text(responsibility.get("id"), "responsibility id")?,
            text(decision.get("id"), "decision id")?,
            text(decision.get("status"), "decision status")?,
            text(decision.get("source"), "decision source")?,
            text(retirement.get("approverActorId"), "retirement approver")?,
            text(retirement.get("lostBehavior"), "lost behavior")?,
            text(retirement.get("residualRisk"), "residual risk")?,
        ));
    }
    markdown.push_str("\n## Verification\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nThis validation is deterministic and self-contained. It does not inspect or mutate providers.\n");
    Ok(markdown)
}

pub fn render_advisory_mdbook_markdown(record: &Value) -> Result<String, String> {
    let runbook = render_advisory_runbook_markdown(record)?;
    Ok(runbook.replace(
        "[`release/stewardship/responsibilities.json`](../stewardship/responsibilities.json)",
        "[`release/stewardship/responsibilities.json`](../../../../release/stewardship/responsibilities.json)",
    ))
}

pub fn validate_advisory_runbook_parity(
    record: &Value,
    runbook: &str,
    documentation: &str,
) -> Result<(), String> {
    if runbook != render_advisory_runbook_markdown(record)? {
        return Err(
            "runbook parity failure: release/runbooks/advisory-automation.md is stale".to_owned(),
        );
    }
    if documentation != render_advisory_mdbook_markdown(record)? {
        return Err(
            "documentation parity failure: docs/book/src/release/advisory-automation.md is stale"
                .to_owned(),
        );
    }
    Ok(())
}

pub fn render_privileged_runbook_markdown(
    operations: &Value,
    responsibilities: &Value,
) -> Result<String, String> {
    let mut markdown = String::from("# Privileged Readiness and Fail-Closed Procedures\n\nThis runbook is generated from [`release/privileged/operations-contract.json`](../privileged/operations-contract.json). It records controlled replacement procedures for privileged and policy responsibilities; it is not a Manifest, approval, credential, workflow, release, or provider configuration. Every recorded operation is currently **blocked**.\n\n");
    markdown.push_str(&render_privileged_operations_body(
        operations,
        responsibilities,
        "Release Unit Catalog",
        "[GOVERNANCE.md](../../GOVERNANCE.md)",
    )?);
    Ok(markdown)
}

fn render_privileged_operations_body(
    operations: &Value,
    _responsibilities: &Value,
    catalog_reference: &str,
    governance_reference: &str,
) -> Result<String, String> {
    let root = object(operations, "privileged operations contract")?;
    let rows = array(root.get("operations"), "privileged operations")?;
    let mut markdown = format!("## Non-authority rule\n\nHistorical bot configuration, historical behavior, green CI, tags, provider approval settings, CODEOWNERS, and private process artifacts are not release authority. The {catalog_reference} inventories source units and provisional typed edges, but its target categories and entries do not establish authorization, publication eligibility, release ordering, or Manifest membership. Dependency ordering and release preparation must use a current Manifest and typed Release Unit Catalog edges when those later controls exist; until then this runbook remains a visible blocking procedure.\n\n## Universal pre-effect gate\n\nNo tag, GitHub release, package, deployment, environment, protected-branch, or credential effect is permitted unless an exact approved Manifest digest, verified Release Steward approval bound to that digest, target-specific protected identity, verified external controls, and immutable candidate inputs all exist and match. Absence, uncertainty, staleness, or mismatch stops before the first effect and produces no effect event or external effect.\n\nAdvisory stages receive no privileged environment or credential. A separately scoped privileged stage may consume only approved immutable inputs after every required gate is verified. Broad or long-lived personal access tokens are rejected. Supported targets require OIDC or provider trusted publishing; a different route would require a separately approved, target-scoped, expiring, revocable, and auditable bootstrap exception.\n\n## Current owned blocking procedures\n\n| ID | Responsibility | Owner assertion | Target | Minimum permissions | Visible blockers | Fallback |\n|---|---|---|---|---|---|---|\n");
    for operation in rows {
        let operation = object(operation, "privileged operation")?;
        markdown.push_str(&format!(
            "<a id=\"{}\"></a>\n",
            text(operation.get("responsibilityId"), "responsibility ID")?.to_ascii_lowercase()
        ));
    }
    for operation in rows {
        let operation = object(operation, "privileged operation")?;
        let owner = object(
            required_value(operation, "owner")?,
            "privileged operation owner",
        )?;
        let target = object(
            required_value(operation, "target")?,
            "privileged operation target",
        )?;
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            text(operation.get("id"), "operation id")?,
            text(operation.get("responsibilityId"), "responsibility ID")?,
            text(owner.get("assignmentId"), "owner assignment")?,
            text(target.get("identity"), "target identity")?,
            strings(operation.get("minimumPermissions"), "minimum permissions")?.join(", "),
            strings(
                operation.get("blockingPrerequisites"),
                "blocking prerequisites"
            )?
            .join("; "),
            text(operation.get("fallback"), "fallback")?,
        ));
    }
    markdown.push_str(&format!("\n## Procedure boundary\n\nEach row is an owned fail-closed procedure with exactly one responsibility ID. It requires the current Manifest and typed catalog edges rather than `.vexilbot.toml` or historical behavior. The runbook does not make any procedure operationally ready: external controls, authorization, registry identity, and candidate evidence remain explicit blockers. A green test or workflow cannot complete a blocked operation.\n\nFor compatibility and policy decisions, follow {governance_reference}; this runbook neither changes nor bypasses its BDFL, RFC, or breaking-change commitments.\n\n## Validation\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nThe command validates this public contract offline and fails closed. It does not change a workflow, environment, credential, tag, registry, provider, or release.\n"));
    Ok(markdown)
}

pub fn render_privileged_mdbook_markdown(
    operations: &Value,
    responsibilities: &Value,
) -> Result<String, String> {
    let mut markdown = String::from("# Privileged and Policy Operations\n\n> Generated public view of the fail-closed privileged operations contract. The canonical record is [`release/privileged/operations-contract.json`](../../../../release/privileged/operations-contract.json); this Markdown is parity-checked and non-authoritative.\n\nThis runbook is generated from [`release/privileged/operations-contract.json`](../../../../release/privileged/operations-contract.json). It records controlled replacement procedures for privileged and policy responsibilities; it is not a Manifest, approval, credential, workflow, release, or provider configuration. Every recorded operation is currently **blocked**.\n\n");
    markdown.push_str(&render_privileged_operations_body(
        operations,
        responsibilities,
        "[Release Unit Catalog](./catalog.md)",
        "[GOVERNANCE.md](../../../../GOVERNANCE.md)",
    )?);
    Ok(markdown)
}

pub fn validate_privileged_runbook_parity(
    operations: &Value,
    responsibilities: &Value,
    runbook: &str,
    documentation: &str,
) -> Result<(), String> {
    if runbook != render_privileged_runbook_markdown(operations, responsibilities)? {
        return Err("runbook parity failure: release/runbooks/privileged-readiness-and-fail-closed.md is stale".to_owned());
    }
    if documentation != render_privileged_mdbook_markdown(operations, responsibilities)? {
        return Err(
            "documentation parity failure: docs/book/src/release/privileged-operations.md is stale"
                .to_owned(),
        );
    }
    Ok(())
}

pub fn render_markdown(record: &Value) -> Result<String, String> {
    let root = object(record, "contract")?;
    let roles = array(root.get("roles"), "roles")?;
    let mut markdown = String::from("# Stewardship Authority Model\n\n> Generated view of [`release/stewardship.json`](../../../../release/stewardship.json). The JSON record is canonical; this Markdown is non-authoritative and parity-checked.\n\n## Authority boundary\n\nOnly an explicit **Release Steward** role assertion bound to an approved Release Manifest identity and digest can authorize privileged effects. Tags, bots, workflows, green CI, registries, provider approvals, and private build artifacts are non-authoritative evidence or tooling.\n\n| Role | Decision scope | Permitted actions |\n|---|---|---|\n");
    for role in roles {
        let role = object(role, "role")?;
        markdown.push_str(&format!(
            "| {} | {} | {} |\n",
            text(role.get("label"), "label")?,
            text(role.get("decisionScope"), "decisionScope")?,
            strings(role.get("permittedActions"), "permittedActions")?.join(", ")
        ));
    }
    markdown.push_str("\n## Boundaries and continuity\n\nAdvisory automation may validate, triage, label, advise on dependencies, and rehearse only. It has no release, package, deployment, protected-branch, environment, credential, version-selection, Release Set scope-selection, or risk-acceptance authority. A Repository Administrator may only stop, revoke, contain, and activate succession in an emergency; it may not move tags, overwrite artifacts, rewrite evidence, accept security risk, approve publication, or declare completion.\n\nRoles may be combined, but permissions never union implicitly: each action requires an explicit asserted role. Role assignments are deliberately absent from this contract and are recorded separately. Contract validation does not prove live workflow or provider enforcement. The reviewed sole-maintainer policy does not prove readiness; publication remains blocked until independent Manifest, registry identity, external-control, security, rehearsal, and closeout gates are verified.\n\n## Offline validation\n\nFrom the repository root, run the repository-local validator without network access:\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nIt validates schema syntax, the canonical record, semantic authority invariants, documentation parity, and the public/private boundary.\n\n## Compatibility governance\n\nThis contract does not replace the BDFL, RFC, public-review, or breaking-change rules in [the governance policy](../../../../GOVERNANCE.md). Language, wire-format, compiler, generator, runtime, corpus/conformance, and public API changes continue through that existing route.\n");
    Ok(markdown)
}

pub fn render_assignment_markdown(record: &Value) -> Result<String, String> {
    let root = object(record, "assignment record")?;
    let decision = object(required_value(root, "decision")?, "decision")?;
    let decision_evidence = object(
        required_value(decision, "reviewEvidence")?,
        "decision review evidence",
    )?;
    let decision_source = text(decision_evidence.get("source"), "decision review source")?;
    let identities = array(root.get("identities"), "identities")?;
    let assignments = array(root.get("assignments"), "assignments")?;
    let readiness = object(
        required_value(root, "publicationReadiness")?,
        "publication readiness",
    )?;
    let continuity = object(required_value(root, "continuity")?, "continuity")?;
    let mut names = HashMap::new();
    for identity in identities {
        let identity = object(identity, "identity")?;
        names.insert(
            text(identity.get("id"), "identity id")?,
            format!(
                "{} ([github.com/{}](https://github.com/{}), {})",
                text(identity.get("name"), "identity name")?,
                text(identity.get("github"), "identity GitHub")?,
                text(identity.get("github"), "identity GitHub")?,
                text(identity.get("email"), "identity email")?
            ),
        );
    }
    let mut markdown = String::from("# Named Stewardship Continuity\n\n> Generated view of [`release/stewardship/assignments.json`](../../../../release/stewardship/assignments.json). The JSON assignment record is canonical; this Markdown is non-authoritative and parity-checked.\n\n## Reviewed public decision\n\n");
    markdown.push_str(&format!(
        "Decision `{}` is effective from {} and has status **{}**. Its authoritative review evidence is [GitHub issue #{}]({}).\n\n",
        text(decision.get("id"), "decision id")?,
        text(decision.get("effectiveFrom"), "decision effective date")?,
        text(decision.get("status"), "decision status")?,
        decision_source.rsplit('/').next().unwrap_or_default(),
        decision_source,
    ));
    markdown
        .push_str("## Current primary assignments\n\n| Role | Primary | Scope |\n|---|---|---|\n");
    for assignment in assignments {
        let assignment = object(assignment, "assignment")?;
        let scope = object(required_value(assignment, "scope")?, "assignment scope")?;
        let primary = text(assignment.get("primaryActorId"), "assignment primary")?;
        let name = names
            .get(primary)
            .ok_or_else(|| format!("missing display identity for {primary}"))?;
        markdown.push_str(&format!(
            "| {} | {} | `{}` |\n",
            text(assignment.get("roleId"), "assignment role")?,
            name,
            text(scope.get("root"), "assignment scope root")?
        ));
    }
    markdown.push_str("\nEach row is an independently auditable role assertion. Combining these assignments does not union permissions: every action remains constrained by the explicit role assertion in the [Stewardship Authority Model](./stewardship.md).\n\n");
    let sole_maintainer =
        text(decision.get("status"), "decision status")? == "sole-maintainer-governance";
    markdown.push_str(if sole_maintainer {
        "## Sole-maintainer policy\n\n"
    } else {
        "## Unresolved continuity gate\n\n"
    });
    let custodian = continuity.get("custodian").unwrap_or(&Value::Null);
    if custodian.is_null() && sole_maintainer {
        markdown.push_str("No distinct recovery custodian is designated by the reviewed sole-maintainer policy. The unavailable-owner route is containment or documented succession only: it may stop, revoke, contain, or activate succession, but cannot create release authority, move tags, overwrite artifacts, rewrite evidence, accept risk, or declare completion.\n\n");
    } else if custodian.is_null() {
        markdown.push_str("No distinct non-publishing recovery custodian has been approved. The unavailable-owner route is containment or documented succession only: it may stop, revoke, contain, or activate succession, but cannot create release authority, move tags, overwrite artifacts, rewrite evidence, accept risk, or declare completion.\n\n");
    }
    let recovery = object(
        required_value(continuity, "recoveryContact")?,
        "recovery contact",
    )?;
    markdown.push_str(&format!(
        "## Recovery contact route\n\n{} Record containment and request a reviewed successor through [the public decision route]({}); this route grants no recovery, Manifest, or publication authority.\n\n",
        if sole_maintainer { "No successor is currently designated." } else { "No distinct custodian is currently approved." },
        text(recovery.get("publicRoute"), "recovery contact route")?
    ));
    markdown.push_str(&format!(
        "**Manifest approval: {}. Privileged publication: {}.** {}\n\n",
        text(readiness.get("manifestApproval"), "manifest approval")?,
        text(
            readiness.get("privilegedPublication"),
            "privileged publication"
        )?,
        text(readiness.get("reason"), "publication reason")?
    ));
    markdown.push_str("If a second qualified Release Steward is recorded, detached approval by an identity distinct from the Manifest approver becomes mandatory; provider self-review settings alone are not evidence. A future [release-continuity-runbook](#future-runbook) is reserved for the unavailable-owner and succession procedure.\n\n## Future runbook\n\nThe stable identifier `release-continuity-runbook` is reserved for the public unavailable-owner and succession runbook. It does not create a custodian or authorize a release.\n\n## Validation\n\nFrom a clean public checkout, run:\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nThe validator checks the authority contract, public role assignments, every currently maintained Package Steward root, documentation parity, and the remaining fail-closed publication gates. It does not change provider settings or create a release.\n\nThis decision preserves the BDFL, RFC, and breaking-change rules in [GOVERNANCE.md](../../../../GOVERNANCE.md).\n");
    Ok(markdown)
}

pub fn validate_documentation_parity(record: &Value, documentation: &str) -> Result<(), String> {
    let expected = render_markdown(record)?;
    if documentation != expected {
        return Err(
            "documentation parity failure: docs/book/src/release/stewardship.md is stale"
                .to_owned(),
        );
    }
    Ok(())
}

pub fn validate_assignment_documentation_parity(
    record: &Value,
    documentation: &str,
) -> Result<(), String> {
    let expected = render_assignment_markdown(record)?;
    if documentation != expected {
        return Err(
            "documentation parity failure: docs/book/src/release/stewardship-continuity.md is stale"
                .to_owned(),
        );
    }
    Ok(())
}

pub fn ensure_no_private_leakage(content: &str) -> Result<(), String> {
    let lower = content.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let has_windows_drive_path = bytes.windows(3).enumerate().any(|(index, window)| {
        window[0].is_ascii_alphabetic()
            && window[1] == b':'
            && matches!(window[2], b'\\' | b'/')
            && (index == 0
                || matches!(
                    bytes[index - 1],
                    b' ' | b'\t' | b'\n' | b'`' | b'\'' | b'"' | b'(' | b'['
                ))
    });
    if lower.contains("c:\\users\\")
        || lower.contains("/users/")
        || lower.contains("/home/")
        || lower.contains("restricted-workspace-reference")
        || lower.contains("_bmad")
        || lower.contains("\\\\")
        || has_windows_drive_path
    {
        return Err(
            "public/private boundary failure: private absolute path or restricted workspace reference found"
                .to_owned(),
        );
    }
    Ok(())
}

fn ensure_public_decision_source(source: &str) -> Result<(), String> {
    ensure_no_private_leakage(source)?;
    let prefix = "https://github.com/vexil-lang/vexil/issues/";
    let issue_number = source
        .strip_prefix(prefix)
        .filter(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()));
    if issue_number.is_none() {
        return Err(
            "review evidence source must be a public vexil-lang/vexil GitHub decision issue"
                .to_owned(),
        );
    }
    Ok(())
}

pub fn validate_contract_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/stewardship.schema.json",
        record,
        "stewardship authority record",
    )
}

pub fn validate_assignment_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/stewardship-assignment.schema.json",
        record,
        "stewardship assignment record",
    )
}

pub fn validate_responsibility_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/retired-bot-responsibility.schema.json",
        record,
        "retired-bot responsibility inventory",
    )
}

pub fn validate_privileged_operation_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/privileged-operation.schema.json",
        record,
        "privileged operation contract",
    )
}

pub fn validate_stewardship_exercise_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/stewardship-exercise.schema.json",
        record,
        "stewardship exercise record",
    )
}

pub fn validate_external_control_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/external-control.schema.json",
        record,
        "expected external controls",
    )
}

pub fn validate_external_observation_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/external-observation.schema.json",
        record,
        "external-control observation",
    )
}

fn validate_observation_inventory(
    root: &Path,
    expected_controls: &BTreeMap<String, ExpectedControl>,
) -> Result<(), String> {
    let directory = root.join("release/controls/observations");
    let mut paths = fs::read_dir(&directory)
        .map_err(|error| format!("cannot read observation inventory: {error}"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate observation inventory: {error}"))?;
    paths.sort();

    let mut baseline_seen = false;
    let mut current_assertions = BTreeSet::new();
    let mut baseline_observed_at = None;
    for path in paths
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
    {
        let record = read_json(&path)?;
        validate_external_observation_schema(root, &record)?;
        ensure_no_private_leakage(&record.to_string())?;

        let observation = object(&record, "external-control observation")?;
        let collection = object(
            required_value(observation, "collection")?,
            "external-control observation collection",
        )?;
        match text(
            collection.get("credentialMode"),
            "observation credential mode",
        )? {
            "no-write-capable-credential" => {}
            "owner-authorized-write-capable-read-only" => {
                let authorization = object(
                    required_value(collection, "credentialAuthorization")?,
                    "owner-authorized observation credential exception",
                )?;
                if text(authorization.get("status"), "credential exception status")?
                    != "explicit-owner-authorized-procedural-audit-exception"
                    || text(
                        authorization.get("allowedOperations"),
                        "credential exception operations",
                    )? != "GET only"
                    || authorization
                        .get("leastPrivilegeEnforced")
                        .and_then(Value::as_bool)
                        != Some(false)
                {
                    return Err("owner-authorized credential exception must retain its GET-only and least-privilege-deviation evidence".to_owned());
                }
            }
            other => return Err(format!("unsupported observation credential mode: {other}")),
        }

        let evidence_state = text(
            observation.get("evidenceState"),
            "observation evidence state",
        )?;
        let observed_at = text(observation.get("observedAtUtc"), "observation time")?;
        let provider = text(observation.get("provider"), "observation provider")?;
        let scope = text(observation.get("scope"), "observation scope")?;
        let mut record_assertions = BTreeSet::new();
        for result in array(observation.get("results"), "observation results")? {
            let result = object(result, "observation result")?;
            let assertion_id = text(result.get("assertionId"), "observation assertion id")?;
            let expected = expected_controls.get(assertion_id).ok_or_else(|| {
                format!("observation references unknown assertion: {assertion_id}")
            })?;
            if !record_assertions.insert(assertion_id) {
                return Err(format!("observation repeats assertion: {assertion_id}"));
            }
            if evidence_state == "current" {
                if provider != expected.provider {
                    return Err(format!(
                        "current observation provider mismatches {assertion_id}"
                    ));
                }
                if provider == "github" && scope != "vexil-lang/vexil" {
                    return Err(
                        "current GitHub observation has an unexpected repository scope".to_owned(),
                    );
                }
                if provider != "github" && scope != expected.scope {
                    return Err(format!(
                        "current observation scope mismatches {assertion_id}"
                    ));
                }
                let query = object(required_value(result, "query")?, "observation query")?;
                if text(query.get("method"), "observation query method")? != expected.method
                    || text(query.get("path"), "observation query path")? != expected.path
                {
                    return Err(format!(
                        "current observation query mismatches {assertion_id}"
                    ));
                }
                if !current_assertions.insert(assertion_id.to_owned()) {
                    return Err(format!("current observations conflict for {assertion_id}"));
                }
            }
        }
        match evidence_state {
            "baseline" => {
                if baseline_seen {
                    return Err(
                        "external-control inventory must retain exactly one baseline".to_owned(),
                    );
                }
                baseline_seen = true;
                baseline_observed_at = Some(observed_at.to_owned());
            }
            "superseded" => {}
            "current" => {
                if let Some(baseline) = &baseline_observed_at {
                    if observed_at <= baseline.as_str() {
                        return Err(
                            "current observation must be newer than the baseline".to_owned()
                        );
                    }
                }
            }
            _ => {
                return Err(
                    "observation evidence state must be baseline, superseded, or current"
                        .to_owned(),
                )
            }
        }
    }
    if !baseline_seen {
        return Err("external-control inventory must retain a baseline observation".to_owned());
    }
    Ok(())
}

pub fn validate_external_remediation_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/external-remediation.schema.json",
        record,
        "external-control remediation",
    )
}

pub fn validate_identity_custody_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/identity-custody.schema.json",
        record,
        "identity custody inventory",
    )
}

pub fn validate_revocation_exercise_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/revocation-exercise.schema.json",
        record,
        "revocation exercise",
    )
}

pub fn validate_history_baseline_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/history-baseline.schema.json",
        record,
        "history baseline",
    )
}

pub fn validate_history_ratification_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/history-ratification.schema.json",
        record,
        "history ratification",
    )
}

pub fn validate_history_observation_sources_schema(
    root: &Path,
    record: &Value,
) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/history-observation-sources.schema.json",
        record,
        "history observation source inventory",
    )
}

pub fn validate_history_observation_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/history-observation.schema.json",
        record,
        "history observation",
    )
}

pub fn validate_history_ledger_entry_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/history-ledger-entry.schema.json",
        record,
        "history ledger entry",
    )
}

pub fn validate_additive_repair_proposal_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/additive-repair-proposal.schema.json",
        record,
        "additive repair proposal",
    )
}

pub fn validate_history_reconciliation_decision_schema(
    root: &Path,
    record: &Value,
) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/history-reconciliation-decision.schema.json",
        record,
        "history reconciliation decision",
    )
}

pub fn validate_catalog_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/catalog.schema.json",
        record,
        "release catalog",
    )
}

pub fn validate_version_rationale_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/version-rationale.schema.json",
        record,
        "release unit version rationale",
    )
}

pub fn validate_catalog_lifecycle_schema(root: &Path, record: &Value) -> Result<(), String> {
    validate_schema_instance(
        root,
        "release/schemas/catalog-lifecycle.schema.json",
        record,
        "catalog lifecycle ledger",
    )
}

fn validate_schema_instance(
    root: &Path,
    schema_relative: &str,
    instance: &Value,
    instance_label: &str,
) -> Result<(), String> {
    let schema = read_json(&root.join(schema_relative))?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| format!("compile {schema_relative}: {error}"))?;
    if let Some(error) = validator.iter_errors(instance).next() {
        return Err(format!("{instance_label} fails {schema_relative}: {error}"));
    }
    Ok(())
}

fn validate_schema_syntax(root: &Path) -> Result<(), String> {
    for (relative, id) in [
        (
            "release/schemas/stewardship.schema.json",
            "https://vexil.dev/release/schemas/stewardship.schema.json",
        ),
        (
            "release/schemas/stewardship-assignment.schema.json",
            "https://vexil.dev/release/schemas/stewardship-assignment.schema.json",
        ),
        (
            "release/schemas/retired-bot-responsibility.schema.json",
            "https://vexil.dev/release/schemas/retired-bot-responsibility.schema.json",
        ),
        (
            "release/schemas/privileged-operation.schema.json",
            "https://vexil.dev/release/schemas/privileged-operation.schema.json",
        ),
        (
            "release/schemas/stewardship-exercise.schema.json",
            "https://vexil.dev/release/schemas/stewardship-exercise.schema.json",
        ),
        (
            "release/schemas/external-control.schema.json",
            "https://vexil.dev/release/schemas/external-control.schema.json",
        ),
        (
            "release/schemas/external-observation.schema.json",
            "https://vexil.dev/release/schemas/external-observation.schema.json",
        ),
        (
            "release/schemas/external-remediation.schema.json",
            "https://vexil.dev/release/schemas/external-remediation.schema.json",
        ),
        (
            "release/schemas/identity-custody.schema.json",
            "https://vexil.dev/release/schemas/identity-custody.schema.json",
        ),
        (
            "release/schemas/revocation-exercise.schema.json",
            "https://vexil.dev/release/schemas/revocation-exercise.schema.json",
        ),
        (
            "release/schemas/history-baseline.schema.json",
            "https://vexil.dev/release/schemas/history-baseline.schema.json",
        ),
        (
            "release/schemas/history-ratification.schema.json",
            "https://vexil.dev/release/schemas/history-ratification.schema.json",
        ),
        (
            "release/schemas/history-observation-sources.schema.json",
            "https://vexil.dev/release/schemas/history-observation-sources.schema.json",
        ),
        (
            "release/schemas/history-observation.schema.json",
            "https://vexil.dev/release/schemas/history-observation.schema.json",
        ),
        (
            "release/schemas/history-ledger-entry.schema.json",
            "https://vexil.dev/release/schemas/history-ledger-entry.schema.json",
        ),
        (
            "release/schemas/additive-repair-proposal.schema.json",
            "https://vexil.dev/release/schemas/additive-repair-proposal.schema.json",
        ),
        (
            "release/schemas/history-reconciliation-decision.schema.json",
            "https://vexil.dev/release/schemas/history-reconciliation-decision.schema.json",
        ),
        (
            "release/schemas/catalog.schema.json",
            "https://vexil.dev/release/schemas/catalog.schema.json",
        ),
        (
            "release/schemas/catalog-lifecycle.schema.json",
            "https://vexil.dev/release/schemas/catalog-lifecycle.schema.json",
        ),
        (
            "release/schemas/version-rationale.schema.json",
            "https://vexil.dev/release/schemas/version-rationale.schema.json",
        ),
        (
            "release/schemas/release-manifest-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-manifest-1.0.schema.json",
        ),
        (
            "release/schemas/release-evidence-set-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-evidence-set-1.0.schema.json",
        ),
        (
            "release/schemas/release-detached-approval-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-detached-approval-1.0.schema.json",
        ),
        (
            "release/schemas/release-approval-disposition-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-approval-disposition-1.0.schema.json",
        ),
        (
            "release/schemas/privileged-run-start-authorization-1.0.schema.json",
            "https://vexil.dev/release/schemas/privileged-run-start-authorization-1.0.schema.json",
        ),
        (
            "release/schemas/release-adapter-result-envelope-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-adapter-result-envelope-1.0.schema.json",
        ),
        (
            "release/schemas/release-run-event-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-run-event-1.0.schema.json",
        ),
        (
            "release/schemas/release-run-evidence-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-run-evidence-1.0.schema.json",
        ),
        (
            "release/schemas/release-closeout-1.0.schema.json",
            "https://vexil.dev/release/schemas/release-closeout-1.0.schema.json",
        ),
    ] {
        let schema_value = read_json(&root.join(relative))?;
        let schema = object(&schema_value, "schema")?;
        require_string(
            schema,
            "$schema",
            "https://json-schema.org/draft/2020-12/schema",
        )?;
        require_string(schema, "$id", id)?;
        if !schema.contains_key("additionalProperties") {
            return Err(format!(
                "schema must use a closed-object strategy: {relative}"
            ));
        }
        jsonschema::draft202012::new(&schema_value)
            .map_err(|error| format!("compile {relative}: {error}"))?;
    }
    Ok(())
}

fn validate_public_boundary(root: &Path) -> Result<(), String> {
    validate_public_text_tree(&root.join("release"))?;
    validate_public_text_tree(&root.join("docs/book/src/release"))?;
    for relative in ["GOVERNANCE.md", "docs/book/src/SUMMARY.md"] {
        ensure_no_private_leakage(
            &fs::read_to_string(root.join(relative))
                .map_err(|error| format!("read {relative}: {error}"))?,
        )?;
    }
    Ok(())
}

fn validate_public_text_tree(path: &Path) -> Result<(), String> {
    for entry in fs::read_dir(path).map_err(|error| format!("read {}: {error}", path.display()))? {
        let entry = entry.map_err(|error| format!("read {} entry: {error}", path.display()))?;
        let entry_path = entry.path();
        if entry.file_name() == "target" || entry.file_name() == "validator" {
            continue;
        }
        if entry_path.is_dir() {
            validate_public_text_tree(&entry_path)?;
        } else {
            ensure_no_private_leakage(
                &fs::read_to_string(&entry_path)
                    .map_err(|error| format!("read {}: {error}", entry_path.display()))?,
            )?;
        }
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let content =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("parse {}: {error}", path.display()))
}
fn required_value<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a Value, String> {
    object
        .get(key)
        .ok_or_else(|| format!("missing required field: {key}"))
}
fn object<'a>(value: &'a Value, context: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{context} must be an object"))
}
fn array<'a>(value: Option<&'a Value>, context: &str) -> Result<&'a Vec<Value>, String> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context} must be an array"))
}
fn text<'a>(value: Option<&'a Value>, context: &str) -> Result<&'a str, String> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{context} must be a string"))
}
fn strings<'a>(value: Option<&'a Value>, context: &str) -> Result<Vec<&'a str>, String> {
    array(value, context)?
        .iter()
        .map(|entry| text(Some(entry), context))
        .collect()
}
fn require_string(object: &Map<String, Value>, key: &str, expected: &str) -> Result<(), String> {
    if text(object.get(key), key)? == expected {
        Ok(())
    } else {
        Err(format!("{key} must equal {expected}"))
    }
}
fn require_exact_keys(
    object: &Map<String, Value>,
    expected: &[&str],
    context: &str,
) -> Result<(), String> {
    let actual: BTreeSet<_> = object.keys().map(String::as_str).collect();
    let expected: BTreeSet<_> = expected.iter().copied().collect();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{context} has missing or unknown fields: expected {expected:?}, got {actual:?}"
        ))
    }
}
fn require_actions(role: &Map<String, Value>, expected: &[&str], id: &str) -> Result<(), String> {
    let actual: BTreeSet<_> = strings(role.get("permittedActions"), "permittedActions")?
        .into_iter()
        .collect();
    let expected: BTreeSet<_> = expected.iter().copied().collect();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("role {id} has an invalid permitted action set"))
    }
}

#[cfg(test)]
mod catalog_manifest_tests {
    use super::{
        catalog_publish_before_edges, go_module_name, manifest_publish_before_edges,
        release_order_from_edges, resolve_known_manifest_dependency, toml_string_in_section,
        CatalogTarget, PublishBeforeEdge,
    };
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;

    #[test]
    fn parses_valid_toml_strings_with_comments_and_single_quotes() {
        let content = "[project]\nname = 'vexil_runtime' # direct source declaration\n";
        assert_eq!(
            toml_string_in_section(content, "project", "name").unwrap(),
            "vexil_runtime"
        );
    }

    #[test]
    fn parses_go_module_declarations_with_permitted_whitespace() {
        let content = "// generated fixture\n\tmodule\tgithub.com/vexil-lang/vexil/packages/runtime-go // source identity\n";
        assert_eq!(
            go_module_name(content).unwrap(),
            "github.com/vexil-lang/vexil/packages/runtime-go"
        );
    }

    #[test]
    fn publishable_cargo_path_dependencies_require_registry_versions() {
        let root = std::env::temp_dir().join(format!(
            "vexil-graph-missing-version-{}",
            std::process::id()
        ));
        let manifest_path = root.join("crates/dependent/Cargo.toml");
        fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
        fs::write(
            &manifest_path,
            "[package]\nname = \"dependent\"\nversion = \"0.1.0\"\n\n[dependencies]\nsource = { path = \"../source\" }\n",
        )
        .unwrap();
        let units = vec![json!({
            "id": "dependent", "kind": "rust-package", "sourceRoot": "crates/dependent",
            "publication": {"status": "source-inventory-only"}, "targets": []
        })];
        let mut targets = BTreeMap::new();
        targets.insert(
            ("cargo-package".to_owned(), "source".to_owned()),
            CatalogTarget {
                id: "source".to_owned(),
                status: "source-inventory-only".to_owned(),
                source_root: "crates/source".to_owned(),
            },
        );
        let error = manifest_publish_before_edges(&root, &units, &targets)
            .expect_err("an unversioned publishable Cargo path dependency must fail");
        fs::remove_dir_all(&root).unwrap();
        assert!(error.contains("dependent"));
        assert!(error.contains("crates/dependent/Cargo.toml#dependencies.source"));
    }

    #[test]
    fn cargo_path_dependencies_must_resolve_to_the_catalog_source_root() {
        let root = std::env::temp_dir().join(format!(
            "vexil-graph-path-provenance-{}",
            std::process::id()
        ));
        let manifest_path = root.join("crates/dependent/Cargo.toml");
        fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
        fs::create_dir_all(root.join("crates/source")).unwrap();
        fs::create_dir_all(root.join("crates/wrong-source")).unwrap();
        fs::write(
            &manifest_path,
            "[package]\nname = \"dependent\"\nversion = \"0.1.0\"\n\n[dependencies]\nsource = { path = \"../wrong-source\", version = \"0.1.0\" }\n",
        )
        .unwrap();
        let units = vec![json!({
            "id": "dependent", "kind": "rust-package", "sourceRoot": "crates/dependent",
            "publication": {"status": "source-inventory-only"}, "targets": []
        })];
        let targets = BTreeMap::from([(
            ("cargo-package".to_owned(), "source".to_owned()),
            CatalogTarget {
                id: "source".to_owned(),
                status: "source-inventory-only".to_owned(),
                source_root: "crates/source".to_owned(),
            },
        )]);
        let error = manifest_publish_before_edges(&root, &units, &targets)
            .expect_err("Cargo path provenance must match the catalog unit root");
        fs::remove_dir_all(&root).unwrap();
        assert!(error.contains("does not resolve to catalog source root crates/source"));
    }

    #[test]
    fn target_specific_cargo_runtime_dependencies_become_publish_before_edges() {
        let root = std::env::temp_dir().join(format!(
            "vexil-graph-target-dependencies-{}",
            std::process::id()
        ));
        let manifest_path = root.join("crates/dependent/Cargo.toml");
        fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
        fs::create_dir_all(root.join("crates/source")).unwrap();
        fs::write(
            &manifest_path,
            "[package]\nname = \"dependent\"\nversion = \"0.1.0\"\n\n[target.'cfg(unix)'.dependencies]\nsource = { path = \"../source\", version = \"0.1.0\" }\n",
        )
        .unwrap();
        let units = vec![json!({
            "id": "dependent", "kind": "rust-package", "sourceRoot": "crates/dependent",
            "publication": {"status": "source-inventory-only"}, "targets": []
        })];
        let targets = BTreeMap::from([(
            ("cargo-package".to_owned(), "source".to_owned()),
            CatalogTarget {
                id: "source".to_owned(),
                status: "source-inventory-only".to_owned(),
                source_root: "crates/source".to_owned(),
            },
        )]);
        let edges = manifest_publish_before_edges(&root, &units, &targets).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(edges.len(), 1);
        assert!(edges
            .iter()
            .any(|edge| edge.location == "target.cfg(unix).dependencies.source"));
    }

    #[test]
    fn workspace_and_build_cargo_path_dependencies_become_publish_before_edges() {
        let root = std::env::temp_dir().join(format!(
            "vexil-graph-workspace-build-dependencies-{}",
            std::process::id()
        ));
        let manifest_path = root.join("crates/dependent/Cargo.toml");
        fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
        fs::create_dir_all(root.join("crates/source")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = []\nexclude = []\n\n[workspace.dependencies]\nsource = { path = \"crates/source\", version = \"0.1.0\" }\n",
        )
        .unwrap();
        fs::write(
            &manifest_path,
            "[package]\nname = \"dependent\"\nversion = \"0.1.0\"\n\n[build-dependencies]\nsource = { workspace = true }\n",
        )
        .unwrap();
        let units = vec![json!({
            "id": "dependent", "kind": "rust-package", "sourceRoot": "crates/dependent",
            "publication": {"status": "source-inventory-only"}, "targets": []
        })];
        let targets = BTreeMap::from([(
            ("cargo-package".to_owned(), "source".to_owned()),
            CatalogTarget {
                id: "source".to_owned(),
                status: "source-inventory-only".to_owned(),
                source_root: "crates/source".to_owned(),
            },
        )]);
        let edges = manifest_publish_before_edges(&root, &units, &targets).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert!(edges.iter().any(|edge| {
            edge.location == "build-dependencies.source"
                && edge.source_kind == "cargo-build-dependency"
        }));
    }

    #[test]
    fn normalized_python_target_names_must_not_be_ambiguous() {
        let targets = BTreeMap::from([
            (
                ("python-project".to_owned(), "vexil-runtime".to_owned()),
                CatalogTarget {
                    id: "one".to_owned(),
                    status: "source-inventory-only".to_owned(),
                    source_root: "packages/one".to_owned(),
                },
            ),
            (
                ("python-project".to_owned(), "vexil_runtime".to_owned()),
                CatalogTarget {
                    id: "two".to_owned(),
                    status: "source-inventory-only".to_owned(),
                    source_root: "packages/two".to_owned(),
                },
            ),
        ]);
        let error = resolve_known_manifest_dependency(
            &targets,
            "python-project",
            "vexil_runtime",
            "dependent",
            "packages/dependent/pyproject.toml",
            "project.dependencies.vexil_runtime",
        )
        .expect_err("normalized Python package names must resolve uniquely");
        assert!(error.contains("ambiguous normalized Python target"));
    }

    #[test]
    fn non_ordering_edges_require_matching_public_decision_records() {
        let root = std::env::temp_dir().join(format!(
            "vexil-graph-decision-record-{}",
            std::process::id()
        ));
        let decision_path = root.join("release/decisions/compatibility.json");
        fs::create_dir_all(decision_path.parent().unwrap()).unwrap();
        fs::write(
            &decision_path,
            r#"{"recordKind":"release-dependency-edge-decision","status":"approved","decisionId":"compatibility-1","edgeType":"compatibility","dependentUnitId":"dependent","relatedUnitId":"related"}"#,
        )
        .unwrap();
        let units = vec![
            json!({
                "id": "dependent", "sourceRoot": "crates/dependent", "dependencyEdges": [{
                    "edgeType": "compatibility", "relatedUnitId": "related", "direction": "related-before-unit",
                    "sourceEvidence": {"sourceKind": "release-dependency-edge-decision", "path": "release/decisions/compatibility.json", "location": "compatibility-1"}
                }]
            }),
            json!({"id": "related", "sourceRoot": "crates/related", "dependencyEdges": []}),
        ];
        catalog_publish_before_edges(&root, &units)
            .expect("a matching approved public decision record must validate");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn publish_before_cycle_reports_only_the_participating_directed_edges() {
        let units = vec![
            json!({"id": "a", "publication": {"status": "source-inventory-only"}}),
            json!({"id": "b", "publication": {"status": "source-inventory-only"}}),
            json!({"id": "tail", "publication": {"status": "source-inventory-only"}}),
        ];
        let edges = BTreeSet::from([
            PublishBeforeEdge {
                dependency_id: "a".into(),
                dependent_id: "b".into(),
                source_kind: "fixture".into(),
                manifest_path: "crates/a/Cargo.toml".into(),
                location: "dependencies.b".into(),
            },
            PublishBeforeEdge {
                dependency_id: "b".into(),
                dependent_id: "a".into(),
                source_kind: "fixture".into(),
                manifest_path: "crates/b/Cargo.toml".into(),
                location: "dependencies.a".into(),
            },
            PublishBeforeEdge {
                dependency_id: "b".into(),
                dependent_id: "tail".into(),
                source_kind: "fixture".into(),
                manifest_path: "crates/tail/Cargo.toml".into(),
                location: "dependencies.b".into(),
            },
        ]);
        let error = release_order_from_edges(&units, &edges)
            .expect_err("a publish_before cycle must block release order");
        assert!(error.contains("a -> b"));
        assert!(error.contains("b -> a"));
        assert!(!error.contains("b -> tail"));
    }
}
