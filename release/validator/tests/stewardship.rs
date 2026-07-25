use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn security_inventory_covers_every_maintained_dependency_and_workflow_surface() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory = fs::read_to_string(root.join("release/security/inventory.toml"))
        .expect("read checked-in security inventory");
    vexil_release_governance_validator::validate_security_inventory(&root, &inventory)
        .expect("complete security inventory must validate");
    vexil_release_governance_validator::validate_security_inventory(
        &root,
        &inventory.replacen("Cargo.lock", "_bmad/private-lock", 1),
    )
    .expect_err("private inventory inputs must fail");
    vexil_release_governance_validator::validate_security_inventory(
        &root,
        &inventory.replacen("id = \"github-actions\"", "id = \"missing-actions\"", 1),
    )
    .expect_err("every maintained workflow surface must remain inventoried");
    vexil_release_governance_validator::validate_security_inventory(
        &root,
        &inventory.replacen("status = \"scanned\"", "status = \"passing\"", 1),
    )
    .expect_err("unknown inventory status must not become a pass");
}

#[test]
fn non_publishing_rehearsal_contract_rejects_authority_and_unexplained_drift() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let digest = "a".repeat(64);
    let manifest = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-manifest-1.1.schema.json",
        "recordKind":"release-manifest", "schemaVersion":"1.1", "manifestId":"rehearsal-fixture",
        "baseCommit":"b".repeat(40), "evidenceSetId":"rehearsal-evidence", "evidenceSetDigest":digest,
        "stateSchema":{"id":"release/state","version":"1.0","digest":"c".repeat(64)},
        "reducer":{"id":"release/reducer","version":"1.0","digest":"d".repeat(64)},
        "releaseUnits":[{"unitId":"runtime","sourceCommit":"e".repeat(40),"previousVersion":null,"proposedVersion":"0.1.0","versionSource":{"path":"packages/runtime-ts/package.json","observedDeclaration":"0.1.0"},"versionRationale":{"id":"runtime-rationale","digest":"f".repeat(64)},"changeUnits":[{"id":"unit","digest":"0".repeat(64)}],"canonicalTag":"runtime-v0.1.0","targets":[{"kind":"fixture","name":"fixture","mandatory":true}]}],
        "publicationOrder":["runtime"],
        "approvalPolicy":{"id":"release/approval","version":"1.0","digest":"1".repeat(64)},
        "failurePolicy":{"id":"release/failure","version":"1.0","digest":"2".repeat(64)},
        "recoveryPolicy":{"id":"release/recovery","version":"1.0","digest":"3".repeat(64)},
        "closeoutRequirements":{"id":"release/closeout","version":"1.0","digest":"4".repeat(64)},
        "security":{"id":"release/security","version":"1.0","digest":"5".repeat(64)},
        "candidate":{"id":"release/candidate","version":"1.0","digest":"6".repeat(64)},
        "rehearsal":{"id":"release/rehearsal","version":"1.0","digest":"7".repeat(64)},
        "registryCustody":{"id":"release/custody","version":"1.0","digest":"8".repeat(64)},
        "historicalTagSnapshot":{"id":"release/history","version":"1.0","digest":"9".repeat(64)},
        "compatibilityEvidence":{"id":"release/compatibility","version":"1.0","digest":"0".repeat(64)},"supersedes":null
    });
    let manifest_bytes = canonical_json(&manifest);
    let fixture_digest = "b".repeat(64);
    let fixtures = [
        ("collision", "tag"),
        ("conflicting-content", "registry"),
        ("duplicate-operation", "tag"),
        ("downstream-failure", "documentation"),
        ("recovery", "registry"),
        ("supersession", "tag"),
        ("unknown-outcome", "registry"),
    ]
    .map(|(scenario, target)| {
        vexil_release_governance_validator::NonPublishingRehearsalFixture {
            scenario,
            target,
            evidence_digest: &fixture_digest,
        }
    });
    let request = vexil_release_governance_validator::NonPublishingRehearsalRequest {
        manifest_bytes: &manifest_bytes,
        candidate_bundle_digest: &"c".repeat(64),
        candidate_subject_digest: &"d".repeat(64),
        candidate_attestation_digest: &"e".repeat(64),
        typed_graph_digest: &"f".repeat(64),
        requested_capabilities: &["read", "build", "test", "package", "attest"],
        fixtures: &fixtures,
    };
    let plan =
        vexil_release_governance_validator::prepare_non_publishing_rehearsal(&root, &request)
            .expect("complete inert rehearsal inputs must validate");
    assert_eq!(plan.fixture_evidence.len(), 7);
    let mut first = BTreeMap::new();
    first.insert("runtime.tgz".to_owned(), b"candidate".to_vec());
    vexil_release_governance_validator::compare_rehearsal_build_outputs(&first, &first)
        .expect("identical isolated candidate bytes must compare");
    let mut drifted = first.clone();
    drifted.insert("runtime.tgz".to_owned(), b"drift".to_vec());
    vexil_release_governance_validator::compare_rehearsal_build_outputs(&first, &drifted)
        .expect_err("unexplained candidate drift must block rehearsal");
    let forbidden = vexil_release_governance_validator::NonPublishingRehearsalRequest {
        requested_capabilities: &["publish"],
        ..request
    };
    vexil_release_governance_validator::prepare_non_publishing_rehearsal(&root, &forbidden)
        .expect_err("non-publishing rehearsal cannot obtain publication authority");
}

#[test]
fn local_git_tag_adapter_is_immutable_and_has_no_remote_target() {
    let fixture_root = std::env::temp_dir().join(format!(
        "vexil-local-tag-adapter-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    let seed = fixture_root.join("seed");
    let bare = fixture_root.join("fixture.git");
    fs::create_dir_all(&seed).expect("create tag fixture seed");
    for args in [
        vec!["init", "--quiet"],
        vec!["config", "user.name", "Vexil fixture"],
        vec!["config", "user.email", "fixture@invalid.example"],
    ] {
        let output = Command::new("git")
            .current_dir(&seed)
            .args(args)
            .output()
            .expect("configure tag fixture seed");
        assert!(output.status.success());
    }
    fs::write(seed.join("README.md"), "fixture\n").expect("write tag fixture source");
    for args in [
        vec!["add", "README.md"],
        vec!["commit", "--quiet", "-m", "fixture source"],
    ] {
        let output = Command::new("git")
            .current_dir(&seed)
            .args(args)
            .output()
            .expect("commit tag fixture source");
        assert!(output.status.success());
    }
    let head = String::from_utf8(
        Command::new("git")
            .current_dir(&seed)
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("resolve tag fixture source commit")
            .stdout,
    )
    .expect("decode tag fixture source commit")
    .trim()
    .to_owned();
    let output = Command::new("git")
        .args(["clone", "--bare", "--quiet"])
        .arg(&seed)
        .arg(&bare)
        .output()
        .expect("create bare tag fixture");
    assert!(output.status.success());
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(&bare)
        .args(["remote", "remove", "origin"])
        .output()
        .expect("remove bare fixture remote");
    assert!(output.status.success());

    let manifest_digest = "a".repeat(64);
    let request = vexil_release_governance_validator::LocalGitTagFixtureRequest {
        bare_repository: &bare,
        unit_id: "runtime-fixture",
        version: "0.1.0",
        tag_name: "runtime-fixture-v0.1.0",
        manifest_digest: &manifest_digest,
        source_commit: &head,
    };
    let plan = vexil_release_governance_validator::prepare_local_git_tag_fixture(&request)
        .expect("local bare tag fixture must prepare");
    assert_eq!(
        vexil_release_governance_validator::probe_local_git_tag_fixture(&request, &plan),
        Ok(vexil_release_governance_validator::LocalGitTagFixtureProbe::Absent)
    );
    assert!(matches!(
        vexil_release_governance_validator::publish_local_git_tag_fixture(&request, &plan),
        Ok(vexil_release_governance_validator::LocalGitTagFixtureProbe::Matching { .. })
    ));
    assert!(matches!(
        vexil_release_governance_validator::verify_local_git_tag_fixture(&request, &plan),
        Ok(vexil_release_governance_validator::LocalGitTagFixtureProbe::Matching { .. })
    ));
    assert_eq!(
        vexil_release_governance_validator::classify_local_git_tag_fixture_failure(
            "tag fixture refuses to overwrite conflicting tag identity"
        ),
        vexil_release_governance_validator::LocalGitTagFixtureFailure::TerminalConflict
    );
    assert_eq!(
        vexil_release_governance_validator::classify_local_git_tag_fixture_failure("git exited 1"),
        vexil_release_governance_validator::LocalGitTagFixtureFailure::Unknown
    );
    assert!(matches!(
        vexil_release_governance_validator::publish_local_git_tag_fixture(&request, &plan),
        Ok(vexil_release_governance_validator::LocalGitTagFixtureProbe::Matching { .. })
    ));
    let reserved = vexil_release_governance_validator::LocalGitTagFixtureRequest {
        tag_name: "v0.1.0",
        ..request
    };
    vexil_release_governance_validator::prepare_local_git_tag_fixture(&reserved)
        .expect_err("root project tag must not be a fixture target");
    fs::remove_dir_all(&fixture_root).expect("remove local tag fixture");
}

#[test]
fn canonical_release_records_bind_the_retained_record_graph_and_reject_boundary_drift() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let isolated =
        std::env::temp_dir().join(format!("vexil-canonical-records-{}", std::process::id()));
    let _ = fs::remove_dir_all(&isolated);
    fs::create_dir_all(isolated.join("release/schemas")).unwrap();
    fs::create_dir_all(isolated.join("release/stewardship")).unwrap();
    fs::create_dir_all(isolated.join("release/evidence-sets/evidence-fixture")).unwrap();
    fs::create_dir_all(isolated.join("release/manifests/manifest-fixture/approvals")).unwrap();
    fs::create_dir_all(isolated.join("release/manifests/manifest-fixture/approval-dispositions"))
        .unwrap();
    fs::create_dir_all(isolated.join("release/runs/run-fixture/evidence/adapter-results")).unwrap();
    fs::create_dir_all(isolated.join("release/reducers")).unwrap();
    for schema in fs::read_dir(root.join("release/schemas")).unwrap() {
        let schema = schema.unwrap().path();
        fs::copy(
            &schema,
            isolated
                .join("release/schemas")
                .join(schema.file_name().unwrap()),
        )
        .unwrap();
    }
    let evidence_source = b"fixture evidence\n";
    fs::write(isolated.join("release/catalog.json"), evidence_source).unwrap();
    fs::copy(
        root.join("release/stewardship.json"),
        isolated.join("release/stewardship.json"),
    )
    .unwrap();
    fs::copy(
        root.join("release/stewardship/assignments.json"),
        isolated.join("release/stewardship/assignments.json"),
    )
    .unwrap();
    let state_schema_source = b"fixture state schema\n";
    fs::write(
        isolated.join("release/schemas/run-state-1.0.schema.json"),
        state_schema_source,
    )
    .unwrap();
    let reducer_source = b"fixture reducer\n";
    fs::write(
        isolated.join("release/reducers/run-state-1.0.wasm"),
        reducer_source,
    )
    .unwrap();
    let state_schema = serde_json::json!({"digest":digest(state_schema_source),"id":"release/schemas/run-state-1.0.schema.json","version":"1.0"});
    let reducer = serde_json::json!({"digest":digest(reducer_source),"id":"release/reducers/run-state-1.0.wasm","version":"1.0"});
    let evidence_set = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-evidence-set-1.0.schema.json",
        "entries":[{"contentDigest":digest(evidence_source),"id":"catalog","kind":"release-catalog","path":"release/catalog.json"}],"reviewedAt":"2026-07-25T00:00:00Z",
        "steward":{"actor":"github:furkanmamuk","assignment":"assignment-release-steward-2026-07-14","role":"release-steward"},
        "evidenceSetId":"evidence-fixture","recordKind":"release-evidence-set","schemaVersion":"1.0"
    });
    let evidence_set_bytes = canonical_json(&evidence_set);
    let evidence_set_digest = digest(&evidence_set_bytes);
    fs::write(
        isolated.join("release/evidence-sets/evidence-fixture/evidence-set.json"),
        &evidence_set_bytes,
    )
    .unwrap();
    let manifest = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-manifest-1.0.schema.json",
        "evidenceSetDigest":evidence_set_digest,"evidenceSetId":"evidence-fixture","manifestId":"manifest-fixture",
        "recordKind":"release-manifest","reducer":reducer,"schemaVersion":"1.0","stateSchema":state_schema
    });
    let manifest_bytes = canonical_json(&manifest);
    let manifest_digest = digest(&manifest_bytes);
    fs::write(
        isolated.join("release/manifests/manifest-fixture/manifest.json"),
        &manifest_bytes,
    )
    .unwrap();
    let approval = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-detached-approval-1.0.schema.json",
        "approvalId":"approval-fixture","approvedAt":"2026-07-25T00:00:00Z",
        "approver":{"actor":"github:furkanmamuk","assignment":"assignment-release-steward-2026-07-14","role":"release-steward"},
        "evidenceSetDigest":evidence_set_digest,"evidenceSetId":"evidence-fixture","expiresAt":"2026-07-26T00:00:00Z",
        "governanceDigest":vexil_release_governance_validator::governance_revision_v1(&isolated).unwrap(),"manifestDigest":manifest_digest,"manifestId":"manifest-fixture","recordKind":"release-detached-approval","schemaVersion":"1.0"
    });
    let approval_bytes = canonical_json(&approval);
    let approval_digest = digest(&approval_bytes);
    fs::write(
        isolated.join("release/manifests/manifest-fixture/approvals/approval-fixture.json"),
        &approval_bytes,
    )
    .unwrap();
    let disposition = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-approval-disposition-1.0.schema.json",
        "approvalDigest":approval_digest,"approvalId":"approval-fixture","disposition":"withdrawn",
        "dispositionId":"withdrawal-fixture","effectiveAt":"2026-07-25T00:01:00Z","authority":{"actor":"github:furkanmamuk","assignment":"assignment-repository-administrator-2026-07-14","role":"repository-administrator"},"reason":"fixture withdrawal","sourceEvidence":{"collector":"fixture","observedAt":"2026-07-25T00:01:00Z","source":"https://example.invalid/evidence/withdrawal-fixture"},
        "recordKind":"release-approval-disposition","schemaVersion":"1.0"
    });
    fs::write(
        isolated.join(
            "release/manifests/manifest-fixture/approval-dispositions/withdrawal-fixture.json",
        ),
        canonical_json(&disposition),
    )
    .unwrap();
    let authorization = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/privileged-run-start-authorization-1.0.schema.json",
        "authorizationId":"authorization-fixture","evidenceSetDigest":evidence_set_digest,"evidenceSetId":"evidence-fixture",
        "executionPrincipal":{"actor":"github:furkanmamuk","assignment":"assignment-release-run-coordinator-2026-07-14","role":"release-run-coordinator"},
        "expiresAt":"2026-07-26T00:00:00Z","issuedAt":"2026-07-25T00:00:00Z",
        "issuer":{"actor":"github:furkanmamuk","assignment":"assignment-release-steward-2026-07-14","role":"release-steward"},"allowedOperations":["publish-runtime"],"allowedPermissions":["packages:write"],"allowedTargets":["registry:crates-io"],"allowedUnits":["vexil-runtime"],
        "manifestDigest":manifest_digest,"manifestId":"manifest-fixture","notBefore":"2026-07-25T00:00:00Z",
        "recordKind":"privileged-run-start-authorization","runId":"run-fixture","schemaVersion":"1.0","workload":"production-release","protectedEnvironment":"production","materializationPath":"release/runs/run-fixture/start-authorization.json",
        "selectedApprovals":[{"approvalId":"approval-fixture","approvalDigest":approval_digest,"disposition":{"id":"withdrawal-fixture","digest":digest(&canonical_json(&disposition))}}],"governanceRevision":{"id":"governance-revision-v1","digest":vexil_release_governance_validator::governance_revision_v1(&isolated).unwrap()},
        "historicalTagBaseline":{"id":"baseline-tags","digest":"1".repeat(64)},"historicalTagSnapshot":{"id":"tag-snapshot","digest":"2".repeat(64)},"security":{"findingsDigest":"3".repeat(64),"exceptionsDigest":"4".repeat(64)},"candidate":{"bundleId":"candidate-bundle","bundleDigest":"5".repeat(64),"subjectDigest":"6".repeat(64),"attestationDigest":"7".repeat(64)},"targetControlEvidence":[{"target":"registry:crates-io","targetControlDigest":"8".repeat(64),"permissionDigest":"9".repeat(64)}],"stateSchema":state_schema,"reducer":reducer
    });
    let authorization_bytes = canonical_json(&authorization);
    let authorization_digest = digest(&authorization_bytes);
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        &authorization_bytes,
    )
    .unwrap();
    let event = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-run-event-1.0.schema.json",
        "actor":"github:furkanmamuk","attempt":1,"observedAt":"2026-07-25T00:00:00Z",
        "authorizationDigest":authorization_digest,"authorizationId":"authorization-fixture","evidenceSetDigest":evidence_set_digest,"evidenceSetId":"evidence-fixture","executionPrincipal":{"actor":"github:furkanmamuk","assignment":"assignment-release-run-coordinator-2026-07-14","role":"release-run-coordinator"},"manifestDigest":manifest_digest,"manifestId":"manifest-fixture","operationId":"publish-runtime","priorEventDigest":null,
        "payloadDigest":"d".repeat(64),"recordKind":"release-run-event","reducer":reducer,"result":"recorded","runId":"run-fixture","schemaVersion":"1.0","sequenceNumber":1,
        "stateSchema":state_schema
    });
    let canonical = canonical_json(&event);
    let stream = isolated.join("release/runs/run-fixture/events.jsonl");
    fs::write(&stream, &canonical).unwrap();
    let adapter_result = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-adapter-result-envelope-1.0.schema.json",
        "adapter":{"id":"registry-adapter","version":"1.0"},"adapterResultId":"adapter-result-fixture",
        "observedAt":"2026-07-25T00:00:00Z","observedDigest":"e".repeat(64),"observedIdentity":"package-fixture",
        "authorizationDigest":authorization_digest,"authorizationId":"authorization-fixture","evidenceSetDigest":evidence_set_digest,"evidenceSetId":"evidence-fixture","manifestDigest":manifest_digest,"manifestId":"manifest-fixture","operationId":"publish-runtime","outcome":"unknown","recordKind":"release-adapter-result-envelope",
        "retryClassification":"safe-to-retry","runId":"run-fixture","schemaVersion":"1.0","source":"fixture"
    });
    let adapter_result_bytes = canonical_json(&adapter_result);
    let adapter_result_digest = digest(&adapter_result_bytes);
    fs::write(
        isolated
            .join("release/runs/run-fixture/evidence/adapter-results/adapter-result-fixture.json"),
        &adapter_result_bytes,
    )
    .unwrap();
    let run_evidence = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-run-evidence-1.0.schema.json",
        "contentDigest":"f".repeat(64),"evidenceId":"run-evidence-fixture","kind":"adapter-log","operationId":"publish-runtime",
        "authorizationDigest":authorization_digest,"authorizationId":"authorization-fixture","evidenceSetDigest":evidence_set_digest,"evidenceSetId":"evidence-fixture","manifestDigest":manifest_digest,"manifestId":"manifest-fixture","recordKind":"release-run-evidence","runId":"run-fixture","schemaVersion":"1.0"
    });
    let run_evidence_bytes = canonical_json(&run_evidence);
    let run_evidence_digest = digest(&run_evidence_bytes);
    fs::write(
        isolated.join("release/runs/run-fixture/evidence/run-evidence-fixture.json"),
        &run_evidence_bytes,
    )
    .unwrap();
    let closeout = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-closeout-1.0.schema.json",
        "authorizationDigest":authorization_digest,"authorizationId":"authorization-fixture","closeoutId":"closeout-fixture","closedAt":"2026-07-25T00:02:00Z","disposition":"aborted","evidenceSetDigest":evidence_set_digest,"evidenceSetId":"evidence-fixture","manifestDigest":manifest_digest,"manifestId":"manifest-fixture",
        "evidence":[{"digest":adapter_result_digest,"id":"adapter-result-fixture","kind":"release-adapter-result-envelope"},{"digest":run_evidence_digest,"id":"run-evidence-fixture","kind":"release-run-evidence"}],"recordKind":"release-closeout","runId":"run-fixture","schemaVersion":"1.0",
        "steward":{"actor":"github:furkanmamuk","assignment":"assignment-release-steward-2026-07-14","role":"release-steward"}
    });
    let closeout_bytes = canonical_json(&closeout);
    fs::write(
        isolated.join("release/runs/run-fixture/closeout.json"),
        &closeout_bytes,
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect("a retained canonical release-record graph must validate");
    let mut missing_disposition_authority = disposition.clone();
    missing_disposition_authority
        .as_object_mut()
        .unwrap()
        .remove("authority");
    vexil_release_governance_validator::validate_canonical_release_record_schema(
        &isolated,
        &missing_disposition_authority,
    )
    .expect_err("an approval disposition must name its Repository Administrator authority");
    let mut forged_issuer = authorization.clone();
    forged_issuer["issuer"]["actor"] = serde_json::json!("github:unassigned");
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        canonical_json(&forged_issuer),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("a start authorization issuer must be an active Release Steward");
    let mut uncovered_target = authorization.clone();
    uncovered_target["allowedTargets"] = serde_json::json!(["registry:other"]);
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        canonical_json(&uncovered_target),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("every allowed target must have exact target-control evidence");
    let mut duplicate_target_evidence = authorization.clone();
    let duplicate = duplicate_target_evidence["targetControlEvidence"][0].clone();
    duplicate_target_evidence["targetControlEvidence"] = serde_json::json!([
        duplicate,
        {"permissionDigest":"0".repeat(64),"target":"registry:crates-io","targetControlDigest":"1".repeat(64)}
    ]);
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        canonical_json(&duplicate_target_evidence),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("each target must have exactly one control-and-permission evidence record");
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        &authorization_bytes,
    )
    .unwrap();
    let mut mismatched_event = event.clone();
    mismatched_event["evidenceSetDigest"] = serde_json::json!("0".repeat(64));
    fs::write(&stream, canonical_json(&mismatched_event)).unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("a Run event must bind its authorization's exact evidence set");
    fs::write(&stream, &canonical).unwrap();
    let mut forged_closeout = closeout.clone();
    forged_closeout["steward"]["assignment"] = serde_json::json!("unassigned-steward");
    fs::write(
        isolated.join("release/runs/run-fixture/closeout.json"),
        canonical_json(&forged_closeout),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("a closeout must name an active Release Steward");
    fs::write(
        isolated.join("release/runs/run-fixture/closeout.json"),
        &closeout_bytes,
    )
    .unwrap();
    let mut alternate_evidence_set = evidence_set.clone();
    alternate_evidence_set["evidenceSetId"] = serde_json::json!("evidence-other");
    let alternate_evidence_set_bytes = canonical_json(&alternate_evidence_set);
    let alternate_evidence_set_digest = digest(&alternate_evidence_set_bytes);
    fs::create_dir_all(isolated.join("release/evidence-sets/evidence-other")).unwrap();
    fs::write(
        isolated.join("release/evidence-sets/evidence-other/evidence-set.json"),
        &alternate_evidence_set_bytes,
    )
    .unwrap();
    let mut alternate_manifest = manifest.clone();
    alternate_manifest["evidenceSetId"] = serde_json::json!("evidence-other");
    alternate_manifest["evidenceSetDigest"] = serde_json::json!(alternate_evidence_set_digest);
    alternate_manifest["manifestId"] = serde_json::json!("manifest-other");
    let alternate_manifest_bytes = canonical_json(&alternate_manifest);
    let alternate_manifest_digest = digest(&alternate_manifest_bytes);
    fs::create_dir_all(isolated.join("release/manifests/manifest-other/approvals")).unwrap();
    fs::write(
        isolated.join("release/manifests/manifest-other/manifest.json"),
        &alternate_manifest_bytes,
    )
    .unwrap();
    let mut alternate_approval = approval.clone();
    alternate_approval["approvalId"] = serde_json::json!("approval-other");
    alternate_approval["evidenceSetId"] = serde_json::json!("evidence-other");
    alternate_approval["evidenceSetDigest"] = serde_json::json!(alternate_evidence_set_digest);
    alternate_approval["manifestId"] = serde_json::json!("manifest-other");
    alternate_approval["manifestDigest"] = serde_json::json!(alternate_manifest_digest);
    let alternate_approval_bytes = canonical_json(&alternate_approval);
    let alternate_approval_digest = digest(&alternate_approval_bytes);
    fs::write(
        isolated.join("release/manifests/manifest-other/approvals/approval-other.json"),
        &alternate_approval_bytes,
    )
    .unwrap();
    let mut cross_manifest_approval = authorization.clone();
    cross_manifest_approval["selectedApprovals"] = serde_json::json!([{
        "approvalId":"approval-other",
        "approvalDigest":alternate_approval_digest,
        "disposition":null
    }]);
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        canonical_json(&cross_manifest_approval),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err(
            "selected approvals must bind the start authorization Manifest and evidence set",
        );
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        &authorization_bytes,
    )
    .unwrap();
    let mut misplaced_authorization = authorization.clone();
    misplaced_authorization["materializationPath"] =
        serde_json::json!("release/runs/other-run/start-authorization.json");
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        canonical_json(&misplaced_authorization),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("start authorization must bind its exact public Run location");
    let mut misbound_approval = authorization.clone();
    misbound_approval["selectedApprovals"][0]["approvalDigest"] = serde_json::json!("0".repeat(64));
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        canonical_json(&misbound_approval),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("start authorization must bind the exact selected approval bytes");
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        &authorization_bytes,
    )
    .unwrap();
    let mut misbound_adapter_result = adapter_result.clone();
    misbound_adapter_result["manifestDigest"] = serde_json::json!("0".repeat(64));
    fs::write(
        isolated
            .join("release/runs/run-fixture/evidence/adapter-results/adapter-result-fixture.json"),
        canonical_json(&misbound_adapter_result),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("a Run adapter result must bind the external Manifest digest");
    fs::write(
        isolated
            .join("release/runs/run-fixture/evidence/adapter-results/adapter-result-fixture.json"),
        &adapter_result_bytes,
    )
    .unwrap();
    let mut unauthorized_adapter_operation = adapter_result.clone();
    unauthorized_adapter_operation["operationId"] = serde_json::json!("unapproved-operation");
    fs::write(
        isolated
            .join("release/runs/run-fixture/evidence/adapter-results/adapter-result-fixture.json"),
        canonical_json(&unauthorized_adapter_operation),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err(
            "Run-scoped evidence must bind an operation allowed by its start authorization",
        );
    fs::write(
        isolated
            .join("release/runs/run-fixture/evidence/adapter-results/adapter-result-fixture.json"),
        &adapter_result_bytes,
    )
    .unwrap();
    let mut invalid_issuer = authorization.clone();
    invalid_issuer["issuer"]["role"] = serde_json::json!("repository-administrator");
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        canonical_json(&invalid_issuer),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("only a Release Steward can issue a start authorization");
    fs::write(
        isolated.join("release/runs/run-fixture/start-authorization.json"),
        &authorization_bytes,
    )
    .unwrap();
    let mut unauthorized_actor = event.clone();
    unauthorized_actor["actor"] = serde_json::json!("other-actor");
    fs::write(&stream, canonical_json(&unauthorized_actor)).unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("Run events must bind the authorized execution-principal actor");
    fs::write(&stream, &canonical).unwrap();
    let mut misbound_event_manifest = event.clone();
    misbound_event_manifest["manifestDigest"] = serde_json::json!("0".repeat(64));
    fs::write(&stream, canonical_json(&misbound_event_manifest)).unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("Run events must bind the Manifest frozen by their start authorization");
    fs::write(&stream, &canonical).unwrap();
    let mut missing_payload_digest = event.clone();
    missing_payload_digest
        .as_object_mut()
        .unwrap()
        .remove("payloadDigest");
    fs::write(&stream, canonical_json(&missing_payload_digest)).unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("Run events require an external payload digest");
    fs::write(&stream, &canonical).unwrap();
    let mut expired_event = event.clone();
    expired_event["observedAt"] = serde_json::json!("2026-07-27T00:00:00Z");
    fs::write(&stream, canonical_json(&expired_event)).unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect("Run eligibility is deferred to the later Run workflow");
    fs::write(&stream, &canonical).unwrap();
    fs::write(
        isolated.join("release/reducers/run-state-1.0.wasm"),
        b"mutated reducer\n",
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("a retained Manifest reducer digest must match exact public bytes");
    fs::write(
        isolated.join("release/reducers/run-state-1.0.wasm"),
        reducer_source,
    )
    .unwrap();
    let mut unreviewed_evidence = evidence_set.clone();
    unreviewed_evidence["steward"]["actor"] = serde_json::json!("unassigned-actor");
    unreviewed_evidence["steward"]["assignment"] = serde_json::json!("unassigned-assignment");
    fs::write(
        isolated.join("release/evidence-sets/evidence-fixture/evidence-set.json"),
        canonical_json(&unreviewed_evidence),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("reviewed evidence sets require an active Release Steward binding");
    fs::write(
        isolated.join("release/evidence-sets/evidence-fixture/evidence-set.json"),
        &evidence_set_bytes,
    )
    .unwrap();
    let mut incomplete_closeout = closeout.clone();
    incomplete_closeout["evidence"] = serde_json::json!([]);
    fs::write(
        isolated.join("release/runs/run-fixture/closeout.json"),
        canonical_json(&incomplete_closeout),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("closeout evidence must cover every retained Run evidence record");
    fs::write(
        isolated.join("release/runs/run-fixture/closeout.json"),
        &closeout_bytes,
    )
    .unwrap();
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(&canonical);
    let carriage_return = String::from_utf8(canonical.clone())
        .unwrap()
        .replace('\n', "\r\n")
        .into_bytes();
    let trailing_line = [canonical.as_slice(), b"\n"].concat();
    for (label, bytes) in [
        ("BOM", bom),
        ("CR", carriage_return),
        ("trailing LF", trailing_line),
    ] {
        fs::write(&stream, bytes).unwrap();
        let error =
            vexil_release_governance_validator::validate_canonical_release_records_repository(
                &isolated,
            )
            .expect_err("raw-byte drift must fail canonical JSONL validation");
        assert!(
            error.contains("invalid raw-byte profile"),
            "{label}: {error}"
        );
    }
    fs::write(&stream, &canonical).unwrap();
    let reordered = String::from_utf8(canonical.clone()).unwrap().replacen(
        "{\"$schema\"",
        "{\"recordKind\":\"release-run-event\",\"$schema\"",
        1,
    );
    fs::write(&stream, reordered).unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("duplicate/non-lexicographic event keys must fail canonical validation");
    fs::write(&stream, &canonical).unwrap();
    let mut private_evidence_set = evidence_set.clone();
    private_evidence_set["entries"][0]["path"] =
        serde_json::json!("C:/Users/private/evidence.json");
    fs::write(
        isolated.join("release/evidence-sets/evidence-fixture/evidence-set.json"),
        canonical_json(&private_evidence_set),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("private absolute evidence paths must fail before reference resolution");
    fs::write(
        isolated.join("release/evidence-sets/evidence-fixture/evidence-set.json"),
        &evidence_set_bytes,
    )
    .unwrap();
    let mut unsupported_manifest = manifest.clone();
    unsupported_manifest["schemaVersion"] = serde_json::json!("1.1");
    fs::write(
        isolated.join("release/manifests/manifest-fixture/manifest.json"),
        canonical_json(&unsupported_manifest),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("an unretained Manifest schema version must fail closed");
    fs::write(
        isolated.join("release/manifests/manifest-fixture/manifest.json"),
        &manifest_bytes,
    )
    .unwrap();
    let mut second_event = event.clone();
    second_event["sequenceNumber"] = serde_json::json!(2);
    second_event["attempt"] = serde_json::json!(2);
    second_event["priorEventDigest"] = serde_json::json!(digest(&canonical));
    let second = canonical_json(&second_event);
    fs::write(&stream, [&canonical[..], &second[..]].concat()).unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect("a contiguous digest-chained stream must validate");
    second_event["priorEventDigest"] = serde_json::json!("0".repeat(64));
    fs::write(
        &stream,
        [&canonical[..], &canonical_json(&second_event)[..]].concat(),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("a Run event must retain the exact preceding-line digest");
    second_event["priorEventDigest"] = serde_json::json!(digest(&canonical));
    second_event["sequenceNumber"] = serde_json::json!(3);
    fs::write(
        &stream,
        [&canonical[..], &canonical_json(&second_event)[..]].concat(),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("Run event sequence numbers must be contiguous");
    second_event["sequenceNumber"] = serde_json::json!(2);
    second_event["attempt"] = serde_json::json!(1);
    fs::write(
        &stream,
        [&canonical[..], &canonical_json(&second_event)[..]].concat(),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("Run operation/attempt identities must not repeat");
    fs::write(&stream, &canonical).unwrap();
    fs::write(
        &stream,
        String::from_utf8(canonical.clone())
            .unwrap()
            .replace(",\"actor\"", ", \"actor\""),
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("parse-equivalent whitespace drift must fail canonical validation");
    fs::write(&stream, canonical).unwrap();
    fs::write(
        isolated.join("release/runs/run-fixture/manifest.json"),
        "{}\n",
    )
    .unwrap();
    vexil_release_governance_validator::validate_canonical_release_records_repository(&isolated)
        .expect_err("a Manifest-shaped path inside a Run cannot acquire authority");
    fs::remove_dir_all(isolated).unwrap();
}

#[test]
fn canonical_record_initial_contract_uses_external_manifest_identity_and_full_start_preflight() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-manifest-1.0.schema.json",
        "evidenceSetDigest":"a".repeat(64),"evidenceSetId":"evidence-fixture","manifestId":"manifest-fixture",
        "recordKind":"release-manifest","reducer":{"digest":"b".repeat(64),"id":"release/reducers/run-state-1.0.wasm","version":"1.0"},
        "schemaVersion":"1.0","stateSchema":{"digest":"c".repeat(64),"id":"release/schemas/run-state-1.0.schema.json","version":"1.0"}
    });
    vexil_release_governance_validator::validate_canonical_release_record_schema(&root, &manifest)
        .expect(
        "a Manifest identity is the external digest of its exact bytes, never a self-digest field",
    );
    let mut legacy_self_digest = manifest.clone();
    legacy_self_digest["payloadDigest"] = serde_json::json!("d".repeat(64));
    vexil_release_governance_validator::validate_canonical_release_record_schema(
        &root,
        &legacy_self_digest,
    )
    .expect_err("a Manifest must reject a self-digest field");

    let authorization = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/privileged-run-start-authorization-1.0.schema.json",
        "allowedOperations":["publish-runtime"],"allowedPermissions":["packages:write"],"allowedTargets":["registry:crates-io"],"allowedUnits":["vexil-runtime"],
        "authorizationId":"authorization-fixture","candidate":{"attestationDigest":"e".repeat(64),"bundleDigest":"f".repeat(64),"bundleId":"candidate-bundle","subjectDigest":"0".repeat(64)},
        "evidenceSetDigest":"a".repeat(64),"evidenceSetId":"evidence-fixture",
        "executionPrincipal":{"actor":"coordinator","assignment":"run-coordinator","role":"release-run-coordinator"},"expiresAt":"2026-07-26T00:00:00Z",
        "governanceRevision":{"digest":"2".repeat(64),"id":"governance-revision-v1"},
        "historicalTagBaseline":{"digest":"3".repeat(64),"id":"baseline-tags"},"historicalTagSnapshot":{"digest":"4".repeat(64),"id":"tag-snapshot"},
        "issuedAt":"2026-07-25T00:00:00Z","issuer":{"actor":"steward","assignment":"release-steward","role":"release-steward"},"manifestDigest":"5".repeat(64),"manifestId":"manifest-fixture",
        "materializationPath":"release/runs/run-fixture/start-authorization.json","notBefore":"2026-07-25T00:00:00Z","protectedEnvironment":"production","recordKind":"privileged-run-start-authorization",
        "runId":"run-fixture","schemaVersion":"1.0","security":{"exceptionsDigest":"6".repeat(64),"findingsDigest":"7".repeat(64)},
        "selectedApprovals":[{"approvalDigest":"8".repeat(64),"approvalId":"approval-fixture","disposition":null}],
        "stateSchema":{"digest":"c".repeat(64),"id":"release/schemas/run-state-1.0.schema.json","version":"1.0"},"reducer":{"digest":"b".repeat(64),"id":"release/reducers/run-state-1.0.wasm","version":"1.0"},
        "targetControlEvidence":[{"permissionDigest":"9".repeat(64),"target":"registry:crates-io","targetControlDigest":"a".repeat(64)}],"workload":"release-workload"
    });
    vexil_release_governance_validator::validate_canonical_release_record_schema(
        &root,
        &authorization,
    )
    .expect("start authorization retains every immutable preflight binding");
    for field in [
        "selectedApprovals",
        "governanceRevision",
        "historicalTagBaseline",
        "historicalTagSnapshot",
        "security",
        "candidate",
        "targetControlEvidence",
        "stateSchema",
        "reducer",
        "workload",
        "protectedEnvironment",
        "allowedOperations",
        "allowedUnits",
        "allowedTargets",
        "allowedPermissions",
        "materializationPath",
    ] {
        let mut incomplete = authorization.clone();
        incomplete.as_object_mut().unwrap().remove(field);
        vexil_release_governance_validator::validate_canonical_release_record_schema(
            &root,
            &incomplete,
        )
        .expect_err(&format!("start authorization must retain {field}"));
    }
}

fn canonical_json(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).unwrap();
    bytes.push(b'\n');
    bytes
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn clean_manifest_generation_root(source_root: &Path) -> PathBuf {
    let isolated = std::env::temp_dir().join(format!(
        "vexil-manifest-generation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    let output = Command::new("git")
        .env("GIT_CLONE_PROTECTION_ACTIVE", "false")
        .args(["clone", "--quiet", "--no-local"])
        .arg(source_root)
        .arg(&isolated)
        .output()
        .expect("clone clean public fixture repository");
    assert!(
        output.status.success(),
        "git clone failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::copy(
        source_root.join("release/schemas/release-manifest-1.0.schema.json"),
        isolated.join("release/schemas/release-manifest-1.0.schema.json"),
    )
    .expect("copy current Manifest schema into clean fixture repository");
    fs::copy(
        source_root.join("release/schemas/release-manifest-1.1.schema.json"),
        isolated.join("release/schemas/release-manifest-1.1.schema.json"),
    )
    .expect("copy current generated Manifest schema into clean fixture repository");
    fs::copy(
        source_root.join("release/schemas/history-observation.schema.json"),
        isolated.join("release/schemas/history-observation.schema.json"),
    )
    .expect("copy current history observation schema into clean fixture repository");
    fs::copy(
        source_root.join("release/schemas/checkpoint-change-unit-1.0.schema.json"),
        isolated.join("release/schemas/checkpoint-change-unit-1.0.schema.json"),
    )
    .expect("copy current checkpoint Change Unit schema into clean fixture repository");
    fs::copy(
        source_root.join("release/schemas/security-scan-1.0.schema.json"),
        isolated.join("release/schemas/security-scan-1.0.schema.json"),
    )
    .expect("copy current security scan schema into clean fixture repository");
    let output = Command::new("git")
        .current_dir(&isolated)
        .args([
            "add",
            "release/schemas/release-manifest-1.0.schema.json",
            "release/schemas/release-manifest-1.1.schema.json",
            "release/schemas/history-observation.schema.json",
            "release/schemas/checkpoint-change-unit-1.0.schema.json",
            "release/schemas/security-scan-1.0.schema.json",
        ])
        .output()
        .expect("stage fixture Manifest schema");
    assert!(
        output.status.success(),
        "git add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .current_dir(&isolated)
        .args([
            "-c",
            "user.name=Vexil fixture",
            "-c",
            "user.email=fixture@invalid.example",
            "commit",
            "--allow-empty",
            "--quiet",
            "-m",
            "test: update Manifest fixture schema",
        ])
        .output()
        .expect("commit fixture Manifest schema");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    isolated
}

fn commit_fixture_path(root: &Path, path: &str, message: &str) {
    let output = Command::new("git")
        .current_dir(root)
        .args(["add", path])
        .output()
        .expect("stage fixture path");
    assert!(
        output.status.success(),
        "git add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.name=Vexil fixture",
            "-c",
            "user.email=fixture@invalid.example",
            "commit",
            "--quiet",
            "-m",
            message,
        ])
        .output()
        .expect("commit fixture path");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture_head_commit(root: &Path) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("resolve fixture commit");
    assert!(output.status.success(), "fixture commit resolution failed");
    String::from_utf8(output.stdout)
        .expect("fixture commit output is UTF-8")
        .trim()
        .to_owned()
}

#[test]
fn manifest_generation_is_deterministic_external_digest_bound_and_side_effect_free() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = clean_manifest_generation_root(&source_root);
    let change_unit_path = "release/checkpoint-change-units/checkpoint-python-generator-fix.json";
    let change_unit_destination = root.join(change_unit_path);
    fs::create_dir_all(
        change_unit_destination
            .parent()
            .expect("Change Unit parent"),
    )
    .expect("create Change Unit parent");
    fs::copy(source_root.join(change_unit_path), &change_unit_destination)
        .expect("copy fixture Change Unit record");
    let catalog_bytes = fs::read(root.join("release/catalog.json")).expect("read catalog");
    let rationale_bytes = fs::read(root.join("release/rationales/vexil-runtime-ts-0-4-1.json"))
        .expect("read version rationale");
    for (path, contents) in [
        ("release/policies/approval-1.0.json", "approval\n"),
        ("release/policies/approval-1.1.json", "updated approval\n"),
        ("release/policies/failure-1.0.json", "failure\n"),
        ("release/policies/recovery-1.0.json", "recovery\n"),
        ("release/policies/closeout-1.0.json", "closeout\n"),
        ("release/candidates/candidate-1.0.json", "candidate\n"),
        ("release/rehearsals/rehearsal-1.0.json", "rehearsal\n"),
        ("release/identities/registry-custody-1.0.json", "custody\n"),
        ("release/evidence/compatibility-1.0.json", "compatibility\n"),
        (
            "release/schemas/run-state-1.0.schema.json",
            "state schema\n",
        ),
        ("release/reducers/run-state-1.0.wasm", "reducer\n"),
    ] {
        let destination = root.join(path);
        fs::create_dir_all(destination.parent().expect("fixture source parent"))
            .expect("create fixture source parent");
        fs::write(destination, contents).expect("write fixture source");
    }
    let history_snapshot_path = "release/history/observations/tag-snapshot-1.0.json";
    let history_snapshot = serde_json::json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "$id":"https://vexil.dev/release/history/observations/tag-snapshot-1.0.json",
        "claim":{"tags":[]},"collectorVersion":"fixture-1.0","contentId":format!("sha256:{}", "a".repeat(64)),
        "observationId":"tag-snapshot-1.0","observedAt":"2026-07-24T00:00:00Z","query":"git ls-remote --tags origin",
        "recordKind":"release-history-observation","sourceId":"git-remote-tags","state":"observed","validUntil":"2026-07-26T00:00:00Z","version":"1.0"
    });
    let history_snapshot_destination = root.join(history_snapshot_path);
    fs::create_dir_all(
        history_snapshot_destination
            .parent()
            .expect("history observation parent"),
    )
    .expect("create history observation parent");
    fs::write(
        &history_snapshot_destination,
        canonical_json(&history_snapshot),
    )
    .expect("write fixture history observation");
    let history_ledger_path = root.join("release/history/ledger.md");
    let mut history_ledger =
        fs::read_to_string(&history_ledger_path).expect("read fixture history ledger");
    history_ledger.push_str("- `tag-snapshot-1.0` — observed from `git-remote-tags` (`sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`)\n");
    fs::write(&history_ledger_path, history_ledger).expect("render fixture history ledger");
    let security_scan_path = "release/security/scans/security-fixture-1.0.json";
    let security_scan_destination = root.join(security_scan_path);
    fs::create_dir_all(
        security_scan_destination
            .parent()
            .expect("security scan parent"),
    )
    .expect("create security scan parent");
    let security_scan = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/security-scan-1.0.schema.json","recordKind":"release-security-scan","schemaVersion":"1.0","scanId":"security-fixture-1.0","observedAt":"2026-07-25T00:00:00Z","scanner":"npm-audit@2",
        "scope":{"manifest":"packages/runtime-ts/package.json","lockfile":"packages/runtime-ts/package-lock.json","exposure":"build-test-release-chain"},"lockfileDigest":format!("sha256:{}",digest(&fs::read(root.join("packages/runtime-ts/package-lock.json")).expect("read fixture lockfile"))),
        "findings":[{"id":"fixture-remediated","package":"fixture","severity":"low","status":"remediated","releaseRelevant":true}]
    });
    fs::write(&security_scan_destination, canonical_json(&security_scan))
        .expect("write fixture security scan");
    commit_fixture_path(
        &root,
        "release",
        "test: add Manifest generation fixture inputs",
    );
    let artifact = |path: &str| serde_json::json!({"digest":digest(&fs::read(root.join(path)).expect("read fixture artifact")),"id":path,"version":"1.0"});
    let evidence_entry = |id: &str, path: &str| serde_json::json!({"contentDigest":digest(&fs::read(root.join(path)).expect("read fixture evidence")),"id":id,"kind":"fixture-input","path":path});
    let evidence_set = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-evidence-set-1.0.schema.json",
        "entries":[
            evidence_entry("approval-policy","release/policies/approval-1.0.json"),
            evidence_entry("candidate","release/candidates/candidate-1.0.json"),
            evidence_entry("change-unit-checkpoint-python-generator-fix",change_unit_path),
            evidence_entry("closeout","release/policies/closeout-1.0.json"),
            evidence_entry("compatibility","release/evidence/compatibility-1.0.json"),
            evidence_entry("failure-policy","release/policies/failure-1.0.json"),
            evidence_entry("historical-tag-snapshot",history_snapshot_path),
            evidence_entry("recovery-policy","release/policies/recovery-1.0.json"),
            evidence_entry("reducer","release/reducers/run-state-1.0.wasm"),
            evidence_entry("registry-custody","release/identities/registry-custody-1.0.json"),
            evidence_entry("rehearsal","release/rehearsals/rehearsal-1.0.json"),
            evidence_entry("security",security_scan_path),
            evidence_entry("state-schema","release/schemas/run-state-1.0.schema.json"),
            evidence_entry("version-rationale","release/rationales/vexil-runtime-ts-0-4-1.json"),
            {"contentDigest":digest(&catalog_bytes),"id":"catalog","kind":"release-catalog","path":"release/catalog.json"}
        ],
        "evidenceSetId":"manifest-generation-evidence","recordKind":"release-evidence-set","reviewedAt":"2026-07-25T00:00:00Z",
        "schemaVersion":"1.0","steward":{"actor":"github:furkanmamuk","assignment":"assignment-release-steward-2026-07-14","role":"release-steward"}
    });
    let evidence_set_bytes = canonical_json(&evidence_set);
    let evidence_set_path =
        root.join("release/evidence-sets/manifest-generation-evidence/evidence-set.json");
    fs::create_dir_all(evidence_set_path.parent().expect("evidence-set parent"))
        .expect("create canonical evidence-set fixture parent");
    fs::write(&evidence_set_path, &evidence_set_bytes)
        .expect("write canonical reviewed evidence-set fixture");
    commit_fixture_path(
        &root,
        "release/evidence-sets/manifest-generation-evidence/evidence-set.json",
        "test: add canonical Manifest evidence-set fixture",
    );
    let manifest = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-manifest-1.1.schema.json",
        "approvalPolicy":artifact("release/policies/approval-1.0.json"),
        "baseCommit":fixture_head_commit(&root),"closeoutRequirements":artifact("release/policies/closeout-1.0.json"),
        "candidate":artifact("release/candidates/candidate-1.0.json"),"compatibilityEvidence":artifact("release/evidence/compatibility-1.0.json"),
        "evidenceSetDigest":digest(&evidence_set_bytes),"evidenceSetId":"manifest-generation-evidence","failurePolicy":artifact("release/policies/failure-1.0.json"),
        "historicalTagSnapshot":artifact(history_snapshot_path),
        "manifestId":"manifest-generation-fixture","publicationOrder":["vexil-runtime-ts"],"recordKind":"release-manifest","recoveryPolicy":artifact("release/policies/recovery-1.0.json"),
        "registryCustody":artifact("release/identities/registry-custody-1.0.json"),"rehearsal":artifact("release/rehearsals/rehearsal-1.0.json"),
        "reducer":artifact("release/reducers/run-state-1.0.wasm"),
        "releaseUnits":[{"canonicalTag":"vexil-runtime-ts-v0.4.1","changeUnits":[{"digest":digest(&fs::read(root.join(change_unit_path)).expect("read checkpoint fixture")),"id":"checkpoint-python-generator-fix"}],"previousVersion":null,"proposedVersion":"0.4.1","sourceCommit":fixture_head_commit(&root),"targets":[{"kind":"npm-package","mandatory":true,"name":"@vexil-lang/runtime"}],"unitId":"vexil-runtime-ts","versionRationale":{"digest":digest(&rationale_bytes),"id":"vexil-runtime-ts-0-4-1"},"versionSource":{"observedDeclaration":"0.4.1","path":"packages/runtime-ts/package.json"}}],
        "schemaVersion":"1.1","security":artifact(security_scan_path),"stateSchema":artifact("release/schemas/run-state-1.0.schema.json"),"supersedes":null
    });

    let first = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &manifest,
            evidence_set_bytes: &evidence_set_bytes,
        },
    )
    .expect("complete synthetic input must generate a Manifest");
    let second = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &manifest,
            evidence_set_bytes: &evidence_set_bytes,
        },
    )
    .expect("same reviewed input must be repeatable");
    let candidate_plan =
        vexil_release_governance_validator::prepare_isolated_candidate_build(&root, &first.bytes)
            .expect("a clean exact Manifest commit may form an isolated candidate-build plan");
    assert_eq!(candidate_plan.manifest_id, "manifest-generation-fixture");
    assert_eq!(candidate_plan.base_commit, fixture_head_commit(&root));
    assert_eq!(
        candidate_plan.source_commits.get("vexil-runtime-ts"),
        Some(&fixture_head_commit(&root))
    );
    assert_eq!(
        candidate_plan.toolchains.get("node"),
        Some(&"22.0.0".to_owned())
    );
    assert_eq!(
        candidate_plan.toolchains.get("python"),
        Some(&"3.10.0".to_owned())
    );
    let forbidden_workspace_root = root.join("candidate-workspace-must-not-be-here");
    assert!(
        vexil_release_governance_validator::materialize_isolated_candidate_workspaces(
            &root,
            &first.bytes,
            &forbidden_workspace_root,
        )
        .is_err(),
        "candidate workspaces must not be created inside the reviewed checkout"
    );
    assert!(
        !forbidden_workspace_root.exists(),
        "a rejected in-repository candidate destination must have no filesystem effect"
    );
    let candidate_workspace_root = std::env::temp_dir().join(format!(
        "vexil-candidate-workspaces-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    let candidate_workspaces =
        vexil_release_governance_validator::materialize_isolated_candidate_workspaces(
            &root,
            &first.bytes,
            &candidate_workspace_root,
        )
        .expect("a clean exact Manifest must materialize detached candidate workspaces");
    assert_eq!(candidate_workspaces.len(), 1);
    let workspace = &candidate_workspaces[0];
    assert_eq!(workspace.unit_id, "vexil-runtime-ts");
    assert_eq!(workspace.source_commit, fixture_head_commit(&root));
    assert!(workspace.path.is_dir());
    assert!(!workspace.path.join("_bmad").exists());
    assert!(!workspace.path.join(".agents").exists());
    assert!(!workspace.path.join("_bmad-output").exists());
    let output = Command::new("git")
        .current_dir(&root)
        .args(["worktree", "remove", "--force"])
        .arg(&workspace.path)
        .output()
        .expect("remove candidate fixture workspace");
    assert!(
        output.status.success(),
        "candidate fixture workspace removal failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(&candidate_workspace_root).expect("remove candidate fixture workspace root");
    fs::create_dir(&candidate_workspace_root).expect("create occupied candidate workspace root");
    assert!(
        vexil_release_governance_validator::materialize_isolated_candidate_workspaces(
            &root,
            &first.bytes,
            &candidate_workspace_root,
        )
        .is_err(),
        "candidate setup must reject an occupied workspace root"
    );
    fs::remove_dir_all(&candidate_workspace_root)
        .expect("remove occupied candidate workspace root");
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.external_digest, second.external_digest);
    assert_eq!(
        first.external_digest,
        format!("sha256:{}", digest(&first.bytes))
    );
    assert!(!root
        .join("release/manifests/manifest-generation-fixture")
        .exists());

    let mut changed_policy = manifest.clone();
    let mut changed_evidence_set = evidence_set.clone();
    changed_evidence_set["evidenceSetId"] = serde_json::json!("manifest-generation-evidence-2");
    changed_evidence_set["entries"][0] =
        evidence_entry("approval-policy", "release/policies/approval-1.1.json");
    let changed_evidence_set_bytes = canonical_json(&changed_evidence_set);
    let changed_evidence_set_path =
        root.join("release/evidence-sets/manifest-generation-evidence-2/evidence-set.json");
    fs::create_dir_all(
        changed_evidence_set_path
            .parent()
            .expect("changed evidence-set parent"),
    )
    .expect("create changed canonical evidence-set fixture parent");
    fs::write(&changed_evidence_set_path, &changed_evidence_set_bytes)
        .expect("write changed canonical reviewed evidence-set fixture");
    commit_fixture_path(
        &root,
        "release/evidence-sets/manifest-generation-evidence-2/evidence-set.json",
        "test: add changed canonical Manifest evidence-set fixture",
    );
    changed_policy["approvalPolicy"] = artifact("release/policies/approval-1.1.json");
    changed_policy["evidenceSetId"] = serde_json::json!("manifest-generation-evidence-2");
    changed_policy["evidenceSetDigest"] = serde_json::json!(digest(&changed_evidence_set_bytes));
    changed_policy["baseCommit"] = serde_json::json!(fixture_head_commit(&root));
    changed_policy["releaseUnits"][0]["sourceCommit"] =
        serde_json::json!(fixture_head_commit(&root));
    let changed = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &changed_policy,
            evidence_set_bytes: &changed_evidence_set_bytes,
        },
    )
    .expect("a material policy change must form a distinct Manifest candidate");
    assert_ne!(first.external_digest, changed.external_digest);

    let expired_history_path = "release/history/observations/tag-snapshot-expired-1.0.json";
    let mut expired_history = history_snapshot.clone();
    expired_history["$id"] = serde_json::json!(
        "https://vexil.dev/release/history/observations/tag-snapshot-expired-1.0.json"
    );
    expired_history["observationId"] = serde_json::json!("tag-snapshot-expired-1.0");
    expired_history["validUntil"] = serde_json::json!("2026-07-24T23:59:59Z");
    let expired_history_destination = root.join(expired_history_path);
    fs::write(
        &expired_history_destination,
        canonical_json(&expired_history),
    )
    .expect("write expired fixture history observation");
    let history_ledger =
        fs::read_to_string(&history_ledger_path).expect("read fixture history ledger");
    let history_ledger = history_ledger.replace(
        "- `tag-snapshot-1.0` — observed from `git-remote-tags` (`sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`)\n",
        "- `tag-snapshot-1.0` — observed from `git-remote-tags` (`sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`)\n- `tag-snapshot-expired-1.0` — observed from `git-remote-tags` (`sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`)\n",
    );
    fs::write(&history_ledger_path, history_ledger).expect("render expired fixture history ledger");
    let mut expired_evidence_set = evidence_set.clone();
    expired_evidence_set["evidenceSetId"] = serde_json::json!("manifest-generation-expired");
    let expired_history_entry =
        evidence_entry("historical-tag-snapshot-expired", expired_history_path);
    let entries = expired_evidence_set["entries"]
        .as_array_mut()
        .expect("expired evidence entries");
    let entry = entries
        .iter_mut()
        .find(|entry| entry["id"] == "historical-tag-snapshot")
        .expect("historical snapshot evidence entry");
    *entry = expired_history_entry;
    let expired_evidence_set_bytes = canonical_json(&expired_evidence_set);
    let expired_evidence_set_path =
        root.join("release/evidence-sets/manifest-generation-expired/evidence-set.json");
    fs::create_dir_all(
        expired_evidence_set_path
            .parent()
            .expect("expired evidence-set parent"),
    )
    .expect("create expired evidence-set fixture parent");
    fs::write(&expired_evidence_set_path, &expired_evidence_set_bytes)
        .expect("write expired canonical reviewed evidence-set fixture");
    commit_fixture_path(
        &root,
        "release",
        "test: add expired Manifest tag observation fixture",
    );
    let mut expired_manifest = manifest.clone();
    expired_manifest["manifestId"] = serde_json::json!("expired-observation-manifest");
    expired_manifest["historicalTagSnapshot"] = artifact(expired_history_path);
    expired_manifest["evidenceSetId"] = serde_json::json!("manifest-generation-expired");
    expired_manifest["evidenceSetDigest"] = serde_json::json!(digest(&expired_evidence_set_bytes));
    expired_manifest["baseCommit"] = serde_json::json!(fixture_head_commit(&root));
    expired_manifest["releaseUnits"][0]["sourceCommit"] =
        serde_json::json!(fixture_head_commit(&root));
    let error = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &expired_manifest,
            evidence_set_bytes: &expired_evidence_set_bytes,
        },
    )
    .expect_err("an expired live-tag observation must block Manifest emission");
    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.requirement == "fresh-live-tag-observation"));

    let mut prior = manifest.clone();
    prior["manifestId"] = serde_json::json!("prior-manifest");
    let prior_bytes = canonical_json(&prior);
    let prior_path = root.join("release/manifests/prior-manifest/manifest.json");
    fs::create_dir_all(prior_path.parent().expect("prior Manifest parent"))
        .expect("create prior Manifest fixture directory");
    fs::write(&prior_path, &prior_bytes).expect("write prior immutable Manifest fixture");
    commit_fixture_path(
        &root,
        "release/manifests/prior-manifest/manifest.json",
        "test: add prior Manifest fixture",
    );
    let mut successor = manifest.clone();
    successor["manifestId"] = serde_json::json!("successor-manifest");
    successor["baseCommit"] = serde_json::json!(fixture_head_commit(&root));
    successor["releaseUnits"][0]["sourceCommit"] = serde_json::json!(fixture_head_commit(&root));
    successor["supersedes"] = serde_json::json!({
        "manifestId":"prior-manifest",
        "manifestDigest":digest(&prior_bytes)
    });
    let successor = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &successor,
            evidence_set_bytes: &evidence_set_bytes,
        },
    )
    .expect("a distinct Manifest may reference immutable predecessor bytes");
    assert_ne!(first.external_digest, successor.external_digest);

    let mut self_digest = manifest.clone();
    self_digest["payloadDigest"] = serde_json::json!("0".repeat(64));
    assert!(
        vexil_release_governance_validator::generate_release_manifest(
            &root,
            &vexil_release_governance_validator::ReleaseManifestRequest {
                manifest: &self_digest,
                evidence_set_bytes: &evidence_set_bytes,
            },
        )
        .is_err()
    );

    let mut wrong_evidence = manifest.clone();
    wrong_evidence["evidenceSetDigest"] = serde_json::json!("0".repeat(64));
    let error = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &wrong_evidence,
            evidence_set_bytes: &evidence_set_bytes,
        },
    )
    .expect_err("a mismatched reviewed evidence set must not emit a Manifest");
    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.requirement == "reviewed-evidence-binding"));

    let mut nonexistent_commit = manifest.clone();
    nonexistent_commit["baseCommit"] = serde_json::json!("0".repeat(40));
    let error = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &nonexistent_commit,
            evidence_set_bytes: &evidence_set_bytes,
        },
    )
    .expect_err("a nonexistent reviewed commit must block Manifest emission");
    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.requirement == "source-led-release-set"));

    let mut missing_reducer = manifest.clone();
    missing_reducer["reducer"]["digest"] = serde_json::json!("0".repeat(64));
    let error = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &missing_reducer,
            evidence_set_bytes: &evidence_set_bytes,
        },
    )
    .expect_err("a stale retained reducer digest must block Manifest emission");
    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.requirement == "retained-state-artifacts"));

    let mut missing_security = manifest.clone();
    missing_security
        .as_object_mut()
        .expect("Manifest object")
        .remove("security");
    let error = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &missing_security,
            evidence_set_bytes: &evidence_set_bytes,
        },
    )
    .expect_err("missing security evidence must block Manifest emission");
    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.requirement == "source-led-release-set"));

    let mut private_input = manifest.clone();
    private_input["candidate"]["id"] = serde_json::json!("_bmad/private-candidate-1.0.json");
    assert!(
        vexil_release_governance_validator::generate_release_manifest(
            &root,
            &vexil_release_governance_validator::ReleaseManifestRequest {
                manifest: &private_input,
                evidence_set_bytes: &evidence_set_bytes,
            },
        )
        .is_err()
    );
    assert!(!root
        .join("release/manifests/manifest-generation-fixture")
        .exists());
    fs::write(
        root.join("release/manifest-generation-dirty-fixture.txt"),
        "dirty\n",
    )
    .expect("create dirty public fixture file");
    let error = vexil_release_governance_validator::generate_release_manifest(
        &root,
        &vexil_release_governance_validator::ReleaseManifestRequest {
            manifest: &manifest,
            evidence_set_bytes: &evidence_set_bytes,
        },
    )
    .expect_err("a dirty public worktree must block Manifest emission");
    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.requirement == "clean-reviewed-source"));
    assert!(
        vexil_release_governance_validator::prepare_isolated_candidate_build(&root, &first.bytes)
            .is_err(),
        "a dirty public checkout must not form an isolated candidate-build plan"
    );
    fs::remove_file(root.join("release/manifest-generation-dirty-fixture.txt"))
        .expect("remove dirty public fixture file");
    fs::remove_dir_all(root).expect("remove clean Manifest generation fixture repository");
}

#[test]
fn detached_approval_constructor_binds_exact_inputs_and_revalidates_governance() {
    use vexil_release_governance_validator::{
        assess_detached_approval, construct_approval_disposition, construct_detached_approval,
        governance_revision_v1, ApprovalDispositionRequest, ApprovalMergeEvidence,
        DetachedApprovalOutcome, DetachedApprovalRequest,
    };

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let evidence_source = fs::read(root.join("release/catalog.json")).unwrap();
    let evidence_set = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-evidence-set-1.0.schema.json",
        "entries":[{"contentDigest":digest(&evidence_source),"id":"catalog","kind":"release-catalog","path":"release/catalog.json"}],
        "reviewedAt":"2026-07-25T00:00:00Z",
        "steward":{"actor":"github:furkanmamuk","assignment":"assignment-release-steward-2026-07-14","role":"release-steward"},
        "evidenceSetId":"evidence-fixture","recordKind":"release-evidence-set","schemaVersion":"1.0"
    });
    let evidence_set_bytes = canonical_json(&evidence_set);
    let evidence_digest = digest(&evidence_set_bytes);
    let manifest = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-manifest-1.0.schema.json",
        "evidenceSetDigest":evidence_digest,"evidenceSetId":"evidence-fixture",
        "manifestId":"manifest-fixture",
        "recordKind":"release-manifest","reducer":{"digest":"c".repeat(64),"id":"release/reducers/run-state-1.0.wasm","version":"1.0"},
        "schemaVersion":"1.0","stateSchema":{"digest":"d".repeat(64),"id":"release/schemas/run-state-1.0.schema.json","version":"1.0"}
    });
    let manifest_bytes = canonical_json(&manifest);
    let governance_digest = governance_revision_v1(&root).unwrap();
    let request = DetachedApprovalRequest {
        approval_id: "approval-fixture",
        manifest_bytes: &manifest_bytes,
        evidence_set_id: "evidence-fixture",
        evidence_set_digest: &evidence_digest,
        governance_digest: &governance_digest,
        actor: "github:furkanmamuk",
        role: "release-steward",
        assignment_id: "assignment-release-steward-2026-07-14",
        manifest_approver_actor: None,
        approved_at: "2026-07-25T00:00:00Z",
        expires_at: "2026-07-26T00:00:00Z",
        provider_audit_reference: None,
    };
    let approval_bytes = construct_detached_approval(&root, &request).unwrap();
    let provider_only = DetachedApprovalRequest {
        provider_audit_reference: Some("https://provider.example/audit/fixture"),
        ..request
    };
    let provider_only_bytes = construct_detached_approval(&root, &provider_only).unwrap();
    assert_eq!(
        assess_detached_approval(&root, &provider_only_bytes, "2026-07-25T12:00:00Z", None)
            .unwrap(),
        DetachedApprovalOutcome::ValidButNotCanonical
    );
    let wrong_role = DetachedApprovalRequest {
        role: "repository-administrator",
        ..request
    };
    assert!(construct_detached_approval(&root, &wrong_role).is_err());
    let bad_request = DetachedApprovalRequest {
        governance_digest: &"0".repeat(64),
        ..request
    };
    assert!(construct_detached_approval(&root, &bad_request).is_err());
    let mut noncanonical = approval_bytes.clone();
    noncanonical.insert(1, b' ');
    assert_eq!(
        assess_detached_approval(&root, &noncanonical, "2026-07-25T12:00:00Z", None).unwrap(),
        DetachedApprovalOutcome::InvalidStructure
    );
    assert_eq!(
        assess_detached_approval(&root, &approval_bytes, "2026-07-25T12:00:00Z", None).unwrap(),
        DetachedApprovalOutcome::ValidButNotCanonical
    );
    let approval_digest = digest(&approval_bytes);
    let merge = ApprovalMergeEvidence {
        repository: "vexil-lang/vexil",
        reference: "refs/heads/main",
        commit_id: &"e".repeat(40),
        tree_id: &"f".repeat(40),
        approval_path: "release/manifests/manifest-fixture/approvals/approval-fixture.json",
        blob_id: &"1".repeat(40),
        observed_blob_bytes: &approval_bytes,
        approval_digest: &approval_digest,
        manifest_bytes: &manifest_bytes,
        evidence_set_bytes: &evidence_set_bytes,
        merge_or_pr_id: "PR-1",
        observed_at: "2026-07-25T12:00:00Z",
        collector: "fixture",
        manifest_approver_actor: None,
        dispositions: &[],
        dispositions_complete: true,
    };
    assert_eq!(
        assess_detached_approval(&root, &approval_bytes, "2026-07-25T12:00:00Z", Some(&merge))
            .unwrap(),
        DetachedApprovalOutcome::Eligible
    );
    let wrong_blob = ApprovalMergeEvidence {
        observed_blob_bytes: b"not the approval bytes\n",
        ..merge
    };
    assert_eq!(
        assess_detached_approval(
            &root,
            &approval_bytes,
            "2026-07-25T12:00:00Z",
            Some(&wrong_blob)
        )
        .unwrap(),
        DetachedApprovalOutcome::ValidButNotCanonical
    );
    let invalid_dependencies = ApprovalMergeEvidence {
        manifest_bytes: b"{}\n",
        ..merge
    };
    assert_eq!(
        assess_detached_approval(
            &root,
            &approval_bytes,
            "2026-07-25T12:00:00Z",
            Some(&invalid_dependencies)
        )
        .unwrap(),
        DetachedApprovalOutcome::InvalidStructure
    );
    let mut successor_manifest = manifest.clone();
    successor_manifest["manifestId"] = Value::String("manifest-fixture-successor".into());
    let successor_manifest_bytes = canonical_json(&successor_manifest);
    assert_ne!(
        digest(&manifest_bytes),
        digest(&successor_manifest_bytes),
        "any canonical Manifest-byte change must have a new external identity"
    );
    let successor_manifest_merge = ApprovalMergeEvidence {
        manifest_bytes: &successor_manifest_bytes,
        ..merge
    };
    assert_eq!(
        assess_detached_approval(
            &root,
            &approval_bytes,
            "2026-07-25T12:00:00Z",
            Some(&successor_manifest_merge),
        )
        .unwrap(),
        DetachedApprovalOutcome::InvalidStructure,
        "an approval cannot be reused for different canonical Manifest bytes"
    );
    let incomplete_dispositions = ApprovalMergeEvidence {
        dispositions_complete: false,
        ..merge
    };
    assert_eq!(
        assess_detached_approval(
            &root,
            &approval_bytes,
            "2026-07-25T12:00:00Z",
            Some(&incomplete_dispositions)
        )
        .unwrap(),
        DetachedApprovalOutcome::CanonicalButCurrentlyIneligible
    );
    let non_main = ApprovalMergeEvidence {
        reference: "refs/heads/feature",
        ..merge
    };
    assert_eq!(
        assess_detached_approval(
            &root,
            &approval_bytes,
            "2026-07-25T12:00:00Z",
            Some(&non_main)
        )
        .unwrap(),
        DetachedApprovalOutcome::ValidButNotCanonical
    );
    let expiry_merge = ApprovalMergeEvidence {
        observed_at: "2026-07-26T00:00:00Z",
        ..merge
    };
    assert_eq!(
        assess_detached_approval(
            &root,
            &approval_bytes,
            "2026-07-26T00:00:00Z",
            Some(&expiry_merge),
        )
        .unwrap(),
        DetachedApprovalOutcome::CanonicalButCurrentlyIneligible
    );
    let revocation = construct_approval_disposition(
        &root,
        &approval_bytes,
        &ApprovalDispositionRequest {
            disposition_id: "revocation-fixture",
            disposition: "revoked",
            effective_at: "2026-07-25T12:00:00Z",
            actor: "github:furkanmamuk",
            role: "repository-administrator",
            assignment_id: "assignment-repository-administrator-2026-07-14",
            reason: "fixture containment",
            source: "https://github.com/vexil-lang/vexil/issues/75",
            observed_at: "2026-07-25T12:00:00Z",
            collector: "fixture",
        },
    )
    .unwrap();
    let dispositions = [revocation.as_slice()];
    let revoked = ApprovalMergeEvidence {
        dispositions: &dispositions,
        ..merge
    };
    assert_eq!(
        assess_detached_approval(
            &root,
            &approval_bytes,
            "2026-07-25T12:00:00Z",
            Some(&revoked)
        )
        .unwrap(),
        DetachedApprovalOutcome::CanonicalButCurrentlyIneligible
    );
}

#[test]
fn privileged_run_start_preflight_is_pure_and_exactly_bound() {
    use vexil_release_governance_validator::{
        authorize_privileged_run_start, construct_detached_approval, governance_revision_v1,
        validate_privileged_job_preflight, ApprovalMergeEvidence, DetachedApprovalPreflight,
        DetachedApprovalRequest, PrivilegedJobPreflight, PrivilegedRunStartRequest,
    };

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let evidence_source = fs::read(root.join("release/catalog.json")).unwrap();
    let evidence_set = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-evidence-set-1.0.schema.json",
        "entries":[{"contentDigest":digest(&evidence_source),"id":"catalog","kind":"release-catalog","path":"release/catalog.json"}],
        "reviewedAt":"2026-07-25T00:00:00Z",
        "steward":{"actor":"github:furkanmamuk","assignment":"assignment-release-steward-2026-07-14","role":"release-steward"},
        "evidenceSetId":"authorization-evidence","recordKind":"release-evidence-set","schemaVersion":"1.0"
    });
    let evidence_set_bytes = canonical_json(&evidence_set);
    let evidence_digest = digest(&evidence_set_bytes);
    let security_scan_path = "release/security/scans/npm-runtime-ts-2026-07-25.json";
    let security_scan_digest = digest(&fs::read(root.join(security_scan_path)).unwrap());
    let artifact =
        |id: &str, digest: String| serde_json::json!({"digest":digest,"id":id,"version":"1.0"});
    let manifest = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/release-manifest-1.1.schema.json",
        "approvalPolicy":artifact("release/policies/approval.json","a".repeat(64)),"baseCommit":"b".repeat(40),"candidate":artifact("release/candidates/candidate.json","1".repeat(64)),"closeoutRequirements":artifact("release/policies/closeout.json","c".repeat(64)),"compatibilityEvidence":artifact("release/evidence/compatibility.json","d".repeat(64)),
        "evidenceSetDigest":evidence_digest,"evidenceSetId":"authorization-evidence","failurePolicy":artifact("release/policies/failure.json","e".repeat(64)),"historicalTagSnapshot":artifact("release/history/observations/snapshot.json","f".repeat(64)),"manifestId":"authorization-manifest","publicationOrder":["vexil-runtime"],"recordKind":"release-manifest","recoveryPolicy":artifact("release/policies/recovery.json","0".repeat(64)),"registryCustody":artifact("release/identities/custody.json","2".repeat(64)),"rehearsal":artifact("release/rehearsals/rehearsal.json","3".repeat(64)),
        "reducer":{"digest":"c".repeat(64),"id":"release/reducers/run-state-1.0.wasm","version":"1.0"},"releaseUnits":[{"canonicalTag":"vexil-runtime-v0.5.1","changeUnits":[{"digest":"4".repeat(64),"id":"checkpoint-python-generator-fix"}],"previousVersion":null,"proposedVersion":"0.5.1","sourceCommit":"5".repeat(40),"targets":[{"kind":"npm-package","mandatory":true,"name":"@vexil-lang/runtime"}],"unitId":"vexil-runtime","versionRationale":{"digest":"6".repeat(64),"id":"runtime-rationale"},"versionSource":{"observedDeclaration":"0.5.1","path":"packages/runtime-ts/package.json"}}],
        "schemaVersion":"1.1","security":artifact(security_scan_path,security_scan_digest.clone()),"stateSchema":{"digest":"d".repeat(64),"id":"release/schemas/run-state-1.0.schema.json","version":"1.0"},"supersedes":null
    });
    let manifest_bytes = canonical_json(&manifest);
    let governance_digest = governance_revision_v1(&root).unwrap();
    let approval_bytes = construct_detached_approval(
        &root,
        &DetachedApprovalRequest {
            approval_id: "authorization-approval",
            manifest_bytes: &manifest_bytes,
            evidence_set_id: "authorization-evidence",
            evidence_set_digest: &evidence_digest,
            governance_digest: &governance_digest,
            actor: "github:furkanmamuk",
            role: "release-steward",
            assignment_id: "assignment-release-steward-2026-07-14",
            manifest_approver_actor: None,
            approved_at: "2026-07-25T00:00:00Z",
            expires_at: "2026-07-26T00:00:00Z",
            provider_audit_reference: None,
        },
    )
    .unwrap();
    let approval_digest = digest(&approval_bytes);
    let baseline: Value =
        serde_json::from_slice(&fs::read(root.join("release/history/baseline-tags.json")).unwrap())
            .unwrap();
    let snapshot = serde_json::json!({"tags":baseline["tags"].clone()});
    let baseline_digest = baseline["baselineDigest"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    let authorization = serde_json::json!({
        "$schema":"https://vexil.dev/release/schemas/privileged-run-start-authorization-1.0.schema.json",
        "authorizationId":"authorization-preflight","candidate":{"attestationDigest":"0".repeat(64),"bundleDigest":"1".repeat(64),"bundleId":"candidate-fixture","subjectDigest":"2".repeat(64)},
        "evidenceSetDigest":evidence_digest,"evidenceSetId":"authorization-evidence",
        "executionPrincipal":{"actor":"github:furkanmamuk","assignment":"assignment-release-run-coordinator-2026-07-14","role":"release-run-coordinator"},
        "expiresAt":"2026-07-26T00:00:00Z","governanceRevision":{"digest":governance_digest,"id":"governance-revision-v1"},
        "historicalTagBaseline":{"digest":baseline_digest,"id":"baseline-tags"},"historicalTagSnapshot":{"digest":digest(&canonical_json(&snapshot)),"id":"snapshot-fixture"},
        "issuedAt":"2026-07-25T00:00:00Z","issuer":{"actor":"github:furkanmamuk","assignment":"assignment-release-steward-2026-07-14","role":"release-steward"},
        "allowedOperations":["privileged-operation-rbr-004"],"allowedPermissions":["publish:exact-approved-release-unit"],"allowedTargets":["npm-package:@vexil-lang/runtime"],"allowedUnits":["vexil-runtime"],
        "manifestDigest":digest(&manifest_bytes),"manifestId":"authorization-manifest","materializationPath":"release/runs/run-preflight/start-authorization.json","notBefore":"2026-07-25T00:00:00Z",
        "protectedEnvironment":"production","recordKind":"privileged-run-start-authorization","reducer":manifest["reducer"].clone(),"runId":"run-preflight","schemaVersion":"1.0",
        "security":{"exceptionsDigest":"3".repeat(64),"findingsDigest":security_scan_digest},"selectedApprovals":[{"approvalDigest":approval_digest,"approvalId":"authorization-approval","disposition":null}],
        "stateSchema":manifest["stateSchema"].clone(),"targetControlEvidence":[{"permissionDigest":"5".repeat(64),"target":"npm-package:@vexil-lang/runtime","targetControlDigest":"6".repeat(64)}],"workload":"production-release"
    });
    let merge = ApprovalMergeEvidence {
        repository: "vexil-lang/vexil",
        reference: "refs/heads/main",
        commit_id: &"a".repeat(40),
        tree_id: &"b".repeat(40),
        approval_path:
            "release/manifests/authorization-manifest/approvals/authorization-approval.json",
        blob_id: &"c".repeat(40),
        observed_blob_bytes: &approval_bytes,
        approval_digest: &approval_digest,
        manifest_bytes: &manifest_bytes,
        evidence_set_bytes: &evidence_set_bytes,
        merge_or_pr_id: "PR-authorization-fixture",
        observed_at: "2026-07-25T12:00:00Z",
        collector: "fixture",
        manifest_approver_actor: None,
        dispositions: &[],
        dispositions_complete: true,
    };
    let approvals = [DetachedApprovalPreflight {
        approval_bytes: &approval_bytes,
        merge,
    }];
    let request = PrivilegedRunStartRequest {
        authorization: &authorization,
        manifest_bytes: &manifest_bytes,
        evidence_set_bytes: &evidence_set_bytes,
        approvals: &approvals,
        historical_tag_baseline: &baseline,
        historical_tag_snapshot: &snapshot,
        evaluation_time: "2026-07-25T12:00:00Z",
    };
    let run_path = root.join("release/runs/run-preflight");
    assert!(
        !run_path.exists(),
        "the fixture must start without Run state"
    );
    let generated = authorize_privileged_run_start(&root, &request).unwrap();
    assert_eq!(generated.bytes, canonical_json(&authorization));
    assert_eq!(generated.external_digest, digest(&generated.bytes));
    assert!(
        !run_path.exists(),
        "authorization preflight must not materialize a Run, lease, or event"
    );
    let mut tag_authorization = authorization.clone();
    tag_authorization["allowedOperations"] = serde_json::json!(["privileged-operation-rbr-003"]);
    tag_authorization["allowedPermissions"] =
        serde_json::json!(["contents:write:refs/tags/exact-approved-manifest-tag"]);
    tag_authorization["allowedTargets"] = serde_json::json!(["release-manifest-bound-tag"]);
    tag_authorization["targetControlEvidence"] = serde_json::json!([{
        "permissionDigest":"5".repeat(64),
        "target":"release-manifest-bound-tag",
        "targetControlDigest":"6".repeat(64)
    }]);
    let tag_request = PrivilegedRunStartRequest {
        authorization: &tag_authorization,
        ..request
    };
    assert_eq!(
        authorize_privileged_run_start(&root, &tag_request)
            .unwrap()
            .bytes,
        canonical_json(&tag_authorization),
        "the reviewed tag operation is bound to the selected units' canonical tags"
    );
    let job = PrivilegedJobPreflight {
        authorization_bytes: &generated.bytes,
        run_id: "run-preflight",
        actor: "github:furkanmamuk",
        role: "release-run-coordinator",
        assignment_id: "assignment-release-run-coordinator-2026-07-14",
        evaluation_time: "2026-07-25T12:00:00Z",
        coordinator_lease_active: true,
        sequenced_run_context_active: true,
    };
    validate_privileged_job_preflight(&root, &job)
        .expect("only exact authorization plus both future Coordinator gates can dispatch");
    let missing_lease = PrivilegedJobPreflight {
        coordinator_lease_active: false,
        ..job
    };
    assert!(validate_privileged_job_preflight(&root, &missing_lease).is_err());
    let missing_context = PrivilegedJobPreflight {
        sequenced_run_context_active: false,
        ..job
    };
    assert!(validate_privileged_job_preflight(&root, &missing_context).is_err());
    let mut altered_authorization = authorization.clone();
    altered_authorization["allowedTargets"] = serde_json::json!(["registry:other"]);
    let altered_request = PrivilegedRunStartRequest {
        authorization: &altered_authorization,
        ..request
    };
    let error = authorize_privileged_run_start(&root, &altered_request).unwrap_err();
    assert!(error
        .blockers
        .iter()
        .any(|blocker| blocker.requirement == "exact-authorized-scope"));
    let mut unknown_operation = authorization.clone();
    unknown_operation["allowedOperations"] = serde_json::json!(["unreviewed-operation"]);
    let unknown_operation_request = PrivilegedRunStartRequest {
        authorization: &unknown_operation,
        ..request
    };
    let error = authorize_privileged_run_start(&root, &unknown_operation_request).unwrap_err();
    assert!(error
        .blockers
        .iter()
        .any(|blocker| blocker.requirement == "exact-authorized-scope"));
    let mut expanded_permissions = authorization.clone();
    expanded_permissions["allowedPermissions"] =
        serde_json::json!(["publish:exact-approved-release-unit", "packages:write"]);
    let expanded_permissions_request = PrivilegedRunStartRequest {
        authorization: &expanded_permissions,
        ..request
    };
    let error = authorize_privileged_run_start(&root, &expanded_permissions_request).unwrap_err();
    assert!(error
        .blockers
        .iter()
        .any(|blocker| blocker.requirement == "exact-authorized-scope"));
    let mut policy_operation = authorization.clone();
    policy_operation["allowedOperations"] = serde_json::json!(["privileged-operation-rbr-008"]);
    policy_operation["allowedPermissions"] = serde_json::json!(["repository-metadata:read"]);
    let policy_operation_request = PrivilegedRunStartRequest {
        authorization: &policy_operation,
        ..request
    };
    let error = authorize_privileged_run_start(&root, &policy_operation_request).unwrap_err();
    assert!(error
        .blockers
        .iter()
        .any(|blocker| blocker.requirement == "exact-authorized-scope"));
    let mut candidate_mismatch = authorization.clone();
    candidate_mismatch["candidate"]["bundleDigest"] = Value::String("9".repeat(64));
    let candidate_mismatch_request = PrivilegedRunStartRequest {
        authorization: &candidate_mismatch,
        ..request
    };
    let error = authorize_privileged_run_start(&root, &candidate_mismatch_request).unwrap_err();
    assert!(error
        .blockers
        .iter()
        .any(|blocker| blocker.requirement == "manifest-and-evidence-binding"));
    let stale_snapshot = serde_json::json!({"tags":[]});
    let stale_snapshot_request = PrivilegedRunStartRequest {
        historical_tag_snapshot: &stale_snapshot,
        ..request
    };
    let error = authorize_privileged_run_start(&root, &stale_snapshot_request).unwrap_err();
    assert!(error
        .blockers
        .iter()
        .any(|blocker| blocker.requirement == "fresh-historical-tag-observation"));
    let mut private_input = authorization.clone();
    private_input["candidate"]["bundleId"] = Value::String("_bmad/private-candidate".into());
    let private_input_request = PrivilegedRunStartRequest {
        authorization: &private_input,
        ..request
    };
    let error = authorize_privileged_run_start(&root, &private_input_request).unwrap_err();
    assert!(error
        .blockers
        .iter()
        .any(|blocker| blocker.requirement == "public-inputs-only"));
    let expired_request = PrivilegedRunStartRequest {
        evaluation_time: "2026-07-26T00:00:00Z",
        ..request
    };
    let error = authorize_privileged_run_start(&root, &expired_request).unwrap_err();
    assert!(error
        .blockers
        .iter()
        .any(|blocker| blocker.requirement == "authorization-window"));
    let mut changed_manifest = manifest.clone();
    changed_manifest["manifestId"] = Value::String("authorization-manifest-successor".into());
    let changed_manifest_bytes = canonical_json(&changed_manifest);
    let changed_manifest_request = PrivilegedRunStartRequest {
        manifest_bytes: &changed_manifest_bytes,
        ..request
    };
    assert!(authorize_privileged_run_start(&root, &changed_manifest_request).is_err());
}

#[test]
fn npm_candidate_artifact_inspection_binds_archive_bytes_and_metadata() {
    use vexil_release_governance_validator::{
        inspect_npm_tgz_candidate, NpmCandidateArtifactInspectionRequest,
    };

    let fixture = std::env::temp_dir().join(format!(
        "vexil-npm-candidate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    let package = fixture.join("package");
    fs::create_dir_all(package.join("dist")).expect("create npm candidate fixture");
    fs::write(
        package.join("package.json"),
        "{\"name\":\"@vexil-lang/runtime\",\"version\":\"0.4.1\"}\n",
    )
    .expect("write npm package metadata");
    fs::write(package.join("dist/index.js"), "export const value = 1;\n")
        .expect("write npm package entry");
    let artifact = fixture.join("vexil-lang-runtime-0.4.1.tgz");
    let output = Command::new("tar")
        .args(["-czf"])
        .arg(&artifact)
        .args(["-C"])
        .arg(&fixture)
        .arg("package")
        .output()
        .expect("create npm candidate archive");
    assert!(
        output.status.success(),
        "tar fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let source_commit = "a".repeat(40);
    let declared_entries = BTreeSet::from([
        "package/package.json".to_owned(),
        "package/dist/index.js".to_owned(),
    ]);
    let request = NpmCandidateArtifactInspectionRequest {
        unit_id: "vexil-runtime-ts",
        source_commit: &source_commit,
        expected_package_name: "@vexil-lang/runtime",
        expected_version: "0.4.1",
        declared_entries: &declared_entries,
        artifact_path: &artifact,
    };
    let inspected = inspect_npm_tgz_candidate(&request).expect("inspect exact npm archive");
    assert_eq!(inspected.entries.len(), 2);
    assert_eq!(inspected.version, "0.4.1");
    let wrong_version = NpmCandidateArtifactInspectionRequest {
        expected_version: "0.4.2",
        ..request
    };
    assert!(inspect_npm_tgz_candidate(&wrong_version).is_err());
    fs::write(
        package.join(".npmrc"),
        "//registry.npmjs.org/:_authToken=fixture\n",
    )
    .expect("write credential fixture");
    let output = Command::new("tar")
        .args(["-czf"])
        .arg(&artifact)
        .args(["-C"])
        .arg(&fixture)
        .arg("package")
        .output()
        .expect("recreate credential fixture archive");
    assert!(output.status.success());
    assert!(inspect_npm_tgz_candidate(&request).is_err());
    fs::remove_dir_all(fixture).expect("remove npm candidate fixture");
}

#[test]
fn python_wheel_candidate_inspection_binds_wheel_bytes_and_metadata() {
    use vexil_release_governance_validator::{
        inspect_python_wheel_candidate, PythonWheelCandidateArtifactInspectionRequest,
    };

    let fixture = std::env::temp_dir().join(format!(
        "vexil-wheel-candidate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    fn urlsafe_base64(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let digest = Sha256::digest(bytes);
        let mut encoded = String::with_capacity(43);
        let mut chunks = digest.chunks_exact(3);
        for chunk in &mut chunks {
            encoded.push(ALPHABET[(chunk[0] >> 2) as usize] as char);
            encoded.push(ALPHABET[((chunk[0] & 0x03) << 4 | (chunk[1] >> 4)) as usize] as char);
            encoded.push(ALPHABET[((chunk[1] & 0x0f) << 2 | (chunk[2] >> 6)) as usize] as char);
            encoded.push(ALPHABET[(chunk[2] & 0x3f) as usize] as char);
        }
        match chunks.remainder() {
            [first] => {
                encoded.push(ALPHABET[(first >> 2) as usize] as char);
                encoded.push(ALPHABET[((first & 0x03) << 4) as usize] as char);
            }
            [first, second] => {
                encoded.push(ALPHABET[(first >> 2) as usize] as char);
                encoded.push(ALPHABET[((first & 0x03) << 4 | (second >> 4)) as usize] as char);
                encoded.push(ALPHABET[((second & 0x0f) << 2) as usize] as char);
            }
            [] => {}
            _ => unreachable!("SHA-256 remainder has at most two bytes"),
        }
        encoded
    }

    let metadata = fixture.join("vexil_runtime-0.1.0.dist-info");
    fs::create_dir_all(&metadata).expect("create Python wheel metadata fixture");
    let metadata_contents = "Metadata-Version: 2.4\nName: vexil_runtime\nVersion: 0.1.0\n";
    fs::write(
        metadata.join("METADATA"),
        metadata_contents,
    )
    .expect("write Python wheel metadata");
    let wheel_contents = "Wheel-Version: 1.0\nRoot-Is-Purelib: true\nTag: py3-none-any\n";
    fs::write(
        metadata.join("WHEEL"),
        wheel_contents,
    )
    .expect("write Python wheel metadata");
    let module_contents = "VALUE = 1\n";
    fs::write(fixture.join("vexil_runtime.py"), module_contents)
        .expect("write Python wheel content");
    let wheel_record_entry = |path: &str, contents: &str| {
        format!(
            "{path},sha256={},{}\n",
            urlsafe_base64(contents.as_bytes()),
            contents.len()
        )
    };
    let record_contents = format!(
        "{}{}{}vexil_runtime-0.1.0.dist-info/RECORD,,\n",
        wheel_record_entry("vexil_runtime.py", module_contents),
        wheel_record_entry(
            "vexil_runtime-0.1.0.dist-info/METADATA",
            metadata_contents
        ),
        wheel_record_entry("vexil_runtime-0.1.0.dist-info/WHEEL", wheel_contents),
    );
    fs::write(
        metadata.join("RECORD"),
        &record_contents,
    )
    .expect("write Python wheel record");
    let artifact = fixture.join("vexil_runtime-0.1.0-py3-none-any.whl");
    let output = Command::new("tar")
        .args(["-a", "-cf"])
        .arg(&artifact)
        .args(["-C"])
        .arg(&fixture)
        .args(["vexil_runtime.py", "vexil_runtime-0.1.0.dist-info"])
        .output()
        .expect("create Python wheel fixture");
    assert!(
        output.status.success(),
        "tar fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let source_commit = "b".repeat(40);
    let declared_entries = BTreeSet::from([
        "vexil_runtime.py".to_owned(),
        "vexil_runtime-0.1.0.dist-info/METADATA".to_owned(),
        "vexil_runtime-0.1.0.dist-info/WHEEL".to_owned(),
        "vexil_runtime-0.1.0.dist-info/RECORD".to_owned(),
    ]);
    let request = PythonWheelCandidateArtifactInspectionRequest {
        unit_id: "vexil-runtime-py",
        source_commit: &source_commit,
        expected_project_name: "vexil_runtime",
        expected_version: "0.1.0",
        declared_entries: &declared_entries,
        artifact_path: &artifact,
    };
    let inspected = inspect_python_wheel_candidate(&request).expect("inspect exact Python wheel");
    assert_eq!(inspected.entries.len(), 4);
    assert_eq!(inspected.project_name, "vexil_runtime");
    assert_eq!(inspected.version, "0.1.0");
    let wrong_project = PythonWheelCandidateArtifactInspectionRequest {
        expected_project_name: "another_project",
        ..request
    };
    assert!(inspect_python_wheel_candidate(&wrong_project).is_err());
    fs::write(
        metadata.join("RECORD"),
        record_contents.replacen("sha256=", "sha256=invalid", 1),
    )
    .expect("write tampered Python wheel record");
    let output = Command::new("tar")
        .args(["-a", "-cf"])
        .arg(&artifact)
        .args(["-C"])
        .arg(&fixture)
        .args(["vexil_runtime.py", "vexil_runtime-0.1.0.dist-info"])
        .output()
        .expect("recreate tampered Python wheel fixture");
    assert!(output.status.success());
    assert!(inspect_python_wheel_candidate(&request).is_err());
    fs::write(metadata.join("RECORD"), &record_contents)
        .expect("restore Python wheel record");
    fs::write(fixture.join("backdoor.py"), "VALUE = 2\n")
        .expect("write undeclared wheel content");
    let output = Command::new("tar")
        .args(["-a", "-cf"])
        .arg(&artifact)
        .args(["-C"])
        .arg(&fixture)
        .args([
            "vexil_runtime.py",
            "backdoor.py",
            "vexil_runtime-0.1.0.dist-info",
        ])
        .output()
        .expect("recreate undeclared Python wheel fixture");
    assert!(output.status.success());
    assert!(inspect_python_wheel_candidate(&request).is_err());
    fs::remove_file(fixture.join("backdoor.py")).expect("remove undeclared wheel content");
    fs::create_dir_all(fixture.join("_bmad")).expect("create prohibited wheel path");
    fs::write(fixture.join("_bmad/private.txt"), "private\n")
        .expect("write prohibited wheel path");
    let output = Command::new("tar")
        .args(["-a", "-cf"])
        .arg(&artifact)
        .args(["-C"])
        .arg(&fixture)
        .args([
            "vexil_runtime.py",
            "vexil_runtime-0.1.0.dist-info",
            "_bmad",
        ])
        .output()
        .expect("recreate prohibited Python wheel fixture");
    assert!(output.status.success());
    assert!(inspect_python_wheel_candidate(&request).is_err());
    fs::remove_dir_all(fixture).expect("remove Python wheel candidate fixture");
}

#[test]
fn cargo_crate_candidate_inspection_binds_archive_bytes_and_metadata() {
    use vexil_release_governance_validator::{
        inspect_cargo_crate_candidate, CargoCrateCandidateArtifactInspectionRequest,
    };

    let fixture = std::env::temp_dir().join(format!(
        "vexil-crate-candidate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    let crate_root = fixture.join("vexil-example-0.4.3");
    fs::create_dir_all(crate_root.join("src")).expect("create Cargo crate fixture");
    fs::write(
        crate_root.join("Cargo.toml"),
        "[package]\nname = \"vexil-example\"\nversion = \"0.4.3\"\n",
    )
    .expect("write Cargo crate metadata");
    fs::write(crate_root.join("src/lib.rs"), "pub const VALUE: u8 = 1;\n")
        .expect("write Cargo crate content");
    let artifact = fixture.join("vexil-example-0.4.3.crate");
    let output = Command::new("tar")
        .args(["-czf"])
        .arg(&artifact)
        .args(["-C"])
        .arg(&fixture)
        .arg("vexil-example-0.4.3")
        .output()
        .expect("create Cargo crate fixture");
    assert!(
        output.status.success(),
        "tar fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let source_commit = "c".repeat(40);
    let declared_entries = BTreeSet::from([
        "vexil-example-0.4.3/Cargo.toml".to_owned(),
        "vexil-example-0.4.3/src/lib.rs".to_owned(),
    ]);
    let request = CargoCrateCandidateArtifactInspectionRequest {
        unit_id: "vexil-example",
        source_commit: &source_commit,
        expected_package_name: "vexil-example",
        expected_version: "0.4.3",
        declared_entries: &declared_entries,
        artifact_path: &artifact,
    };
    let inspected = inspect_cargo_crate_candidate(&request).expect("inspect exact Cargo crate");
    assert_eq!(inspected.entries.len(), 2);
    assert_eq!(inspected.package_name, "vexil-example");
    assert_eq!(inspected.version, "0.4.3");
    let wrong_version = CargoCrateCandidateArtifactInspectionRequest {
        expected_version: "0.4.4",
        ..request
    };
    assert!(inspect_cargo_crate_candidate(&wrong_version).is_err());
    fs::create_dir_all(crate_root.join(".agents")).expect("create prohibited crate path");
    fs::write(crate_root.join(".agents/private.txt"), "private\n")
        .expect("write prohibited crate path");
    let output = Command::new("tar")
        .args(["-czf"])
        .arg(&artifact)
        .args(["-C"])
        .arg(&fixture)
        .arg("vexil-example-0.4.3")
        .output()
        .expect("recreate prohibited Cargo crate fixture");
    assert!(output.status.success());
    assert!(inspect_cargo_crate_candidate(&request).is_err());
    fs::remove_dir_all(fixture).expect("remove Cargo crate candidate fixture");
}

#[test]
fn go_module_candidate_inspection_binds_proxy_zip_bytes_and_metadata() {
    use vexil_release_governance_validator::{
        inspect_go_module_zip_candidate, GoModuleCandidateArtifactInspectionRequest,
    };
    let fixture = std::env::temp_dir().join(format!("vexil-go-candidate-{}", std::process::id()));
    let root = fixture.join("github.com/vexil-lang/vexil/packages/runtime-go@v0.1.0");
    fs::create_dir_all(&root).expect("create Go module fixture");
    fs::write(root.join("go.mod"), "module github.com/vexil-lang/vexil/packages/runtime-go\n\ngo 1.22\n").expect("write go.mod");
    fs::write(root.join("VERSION"), "0.1.0\n").expect("write VERSION");
    fs::write(root.join("runtime.go"), "package runtime\n").expect("write Go content");
    let artifact = fixture.join("runtime-go@v0.1.0.zip");
    let output = Command::new("tar").args(["-a", "-cf"]).arg(&artifact).args(["-C"]).arg(&fixture).arg("github.com").output().expect("create Go proxy zip");
    assert!(output.status.success());
    let prefix = "github.com/vexil-lang/vexil/packages/runtime-go@v0.1.0";
    let declared_entries = BTreeSet::from([
        format!("{prefix}/go.mod"), format!("{prefix}/VERSION"), format!("{prefix}/runtime.go"),
    ]);
    let commit = "d".repeat(40);
    let request = GoModuleCandidateArtifactInspectionRequest { unit_id: "vexil-runtime-go", source_commit: &commit, expected_module_path: "github.com/vexil-lang/vexil/packages/runtime-go", expected_version: "0.1.0", declared_entries: &declared_entries, artifact_path: &artifact };
    let inspected = inspect_go_module_zip_candidate(&request).expect("inspect Go module proxy zip");
    assert_eq!(inspected.entries.len(), 3);
    fs::create_dir_all(root.join("_bmad")).expect("create private fixture path");
    fs::write(root.join("_bmad/private.txt"), "private\n").expect("write private fixture path");
    let output = Command::new("tar").args(["-a", "-cf"]).arg(&artifact).args(["-C"]).arg(&fixture).arg("github.com").output().expect("recreate Go proxy zip");
    assert!(output.status.success());
    assert!(inspect_go_module_zip_candidate(&request).is_err());
    fs::remove_dir_all(fixture).expect("remove Go module fixture");
}

#[test]
fn selective_promotion_rejects_the_wholesale_checkpoint_identity() {
    use vexil_release_governance_validator::{
        validate_selective_promotion, SelectivePromotionRequest,
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let approved = BTreeSet::from(["checkpoint-python-generator-fix".to_owned()]);
    let request = SelectivePromotionRequest {
        current_base: &"a".repeat(40),
        proposed_commit: "d4099e8188f40603ebf52473d6543ce4a6054201",
        approved_change_units: &approved,
    };
    assert!(validate_selective_promotion(&root, &request).is_err());
}

#[test]
fn canonical_contract_and_all_fixtures_have_the_expected_result() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    vexil_release_governance_validator::validate_repository(&root)
        .expect("the canonical stewardship contract must validate");
}

#[test]
fn historical_tag_baseline_and_additive_repair_guards_fail_closed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut baseline = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://vexil.dev/release/history/baseline-tags.json",
        "version": "1.0",
        "recordKind": "historical-tag-baseline",
        "status": "ratified",
        "remote": {"name": "origin", "url": "https://github.com/vexil-lang/vexil.git", "query": "git ls-remote --tags"},
        "observedAt": "2026-07-23T00:00:00Z",
        "baselineDigest": null,
        "tags": [{"name": "v0.2.0", "kind": "annotated", "refTarget": "1111111111111111111111111111111111111111", "annotatedTag": "2222222222222222222222222222222222222222", "peeledCommit": "3333333333333333333333333333333333333333"}],
        "ratificationIds": ["history-ratification-steward", "history-ratification-admin"]
    });
    let digest = vexil_release_governance_validator::history_baseline_digest(
        baseline.as_object().expect("fixture baseline object"),
    )
    .expect("fixture baseline digest");
    baseline["baselineDigest"] = Value::String(digest);
    vexil_release_governance_validator::validate_history_baseline_schema(&root, &baseline)
        .expect("ratified fixture baseline schema must validate");
    vexil_release_governance_validator::validate_history_baseline(&baseline)
        .expect("ratified fixture baseline must retain complete identities");
    let snapshot = serde_json::json!({"tags": baseline["tags"].clone()});
    vexil_release_governance_validator::validate_history_tag_snapshot(&baseline, &snapshot)
        .expect("matching snapshot must preserve every baseline identity");
    baseline["tags"][0]["peeledCommit"] =
        Value::String("4444444444444444444444444444444444444444".into());
    vexil_release_governance_validator::validate_history_tag_snapshot(&baseline, &snapshot)
        .expect_err("moved tag identity must fail closed before a repair");

    let policy: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/history/additive-repair-policy.json")).unwrap(),
    )
    .unwrap();
    let proposal = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://vexil.dev/release/history/repair-proposals/forbidden-tag-move.json",
        "version": "1.0", "recordKind": "additive-repair-proposal", "proposalId": "forbidden-tag-move", "status": "proposed",
        "anomaly": "fixture", "affectedConsumers": ["fixture consumer"], "correctionSurface": "documentation", "newIdentifier": null,
        "approval": null, "evidenceReferences": ["fixture"], "proposedActions": ["move-tag"]
    });
    vexil_release_governance_validator::validate_additive_repair_proposal_schema(&root, &proposal)
        .expect("destructive proposal remains schema-visible for preflight rejection");
    vexil_release_governance_validator::validate_additive_repair_preflight(
        &snapshot, &policy, &proposal,
    )
    .expect_err("tag move must be rejected before any remote operation");
}

#[test]
fn history_schemas_preserve_unknown_evidence_and_root_tag_prohibition() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let unknown = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://vexil.dev/release/history/observations/registry-unavailable.json",
        "version": "1.0", "recordKind": "release-history-observation", "observationId": "registry-unavailable",
        "contentId": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "sourceId": "pypi", "query": "GET PyPI project metadata", "state": "unavailable",
        "observedAt": "2026-07-23T00:00:00Z", "collectorVersion": "fixture", "claim": {}, "failureEvidence": "network unavailable"
    });
    vexil_release_governance_validator::validate_history_observation_schema(&root, &unknown)
        .expect("unavailable evidence must remain a valid, distinct observation");
    let mut decision: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/history/reconciliation-decision.json")).unwrap(),
    )
    .unwrap();
    vexil_release_governance_validator::validate_history_reconciliation_decision_schema(
        &root, &decision,
    )
    .expect("pending reconciliation decision schema must validate");
    decision["rootTagPolicy"] = Value::String("allowed".into());
    vexil_release_governance_validator::validate_history_reconciliation_decision_schema(
        &root, &decision,
    )
    .expect_err("a project-wide root tag policy must be rejected");
}

#[test]
fn read_only_history_collector_preserves_lightweight_and_annotated_identities() {
    let output = concat!(
        "1111111111111111111111111111111111111111\trefs/tags/lightweight\n",
        "2222222222222222222222222222222222222222\trefs/tags/annotated\n",
        "3333333333333333333333333333333333333333\trefs/tags/annotated^{}\n"
    );
    let collection = vexil_release_governance_validator::parse_history_tag_collection(
        "https://example.invalid/vexil.git",
        output,
        "2026-07-23T00:00:00Z",
    )
    .expect("read-only collector fixture must parse");
    assert_eq!(collection["tags"][0]["kind"], "annotated");
    assert_eq!(
        collection["tags"][0]["peeledCommit"],
        "3333333333333333333333333333333333333333"
    );
    assert_eq!(collection["tags"][1]["kind"], "lightweight");
}

#[test]
fn catalog_rejects_detached_owners_stale_source_declarations_and_invalid_publication_states() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/catalog.json")).expect("read canonical catalog"),
    )
    .expect("parse canonical catalog");
    vexil_release_governance_validator::validate_catalog(&root, &catalog)
        .expect("canonical catalog must be source-grounded");

    let mut detached_owner = catalog.clone();
    detached_owner["units"][0]["owner"]["assignmentId"] =
        Value::String("assignment-package-steward-vexil-lang-2026-07-14".into());
    vexil_release_governance_validator::validate_catalog(&root, &detached_owner)
        .expect_err("an owner assignment must bind the declared role and source root");

    let mut escaped_version_path = catalog.clone();
    escaped_version_path["units"][0]["versionSource"]["path"] = Value::String("Cargo.toml".into());
    vexil_release_governance_validator::validate_catalog(&root, &escaped_version_path)
        .expect_err("a version path must remain under the unit source root");

    let mut stale_target = catalog.clone();
    stale_target["units"][6]["targets"][0]["name"] = Value::String("forged-package-name".into());
    vexil_release_governance_validator::validate_catalog(&root, &stale_target)
        .expect_err("a catalog target must match its source manifest");

    let mut stale_binary = catalog.clone();
    stale_binary["units"][17]["targets"][1]["name"] = Value::String("forged-binary-name".into());
    vexil_release_governance_validator::validate_catalog(&root, &stale_binary)
        .expect_err("a catalog binary target must match a declared or default Cargo binary");

    let mut mismatched_kind = catalog.clone();
    mismatched_kind["units"][6]["kind"] = Value::String("typescript-runtime".into());
    vexil_release_governance_validator::validate_catalog(&root, &mismatched_kind)
        .expect_err("a catalog kind must match its source declaration");

    let mut stale_changelog = catalog.clone();
    stale_changelog["units"][6]["changelog"]["status"] = Value::String("absent".into());
    stale_changelog["units"][6]["changelog"]["path"] = Value::Null;
    vexil_release_governance_validator::validate_catalog(&root, &stale_changelog)
        .expect_err("an existing unit changelog must not be cataloged as absent");

    let mut unordered = catalog.clone();
    unordered["units"].as_array_mut().unwrap().swap(0, 1);
    vexil_release_governance_validator::validate_catalog(&root, &unordered)
        .expect_err("catalog units must remain stable-ID ascending");

    let mut no_edges = catalog.clone();
    no_edges["units"][6]["dependencyEdges"] = Value::Null;
    vexil_release_governance_validator::validate_catalog_schema(&root, &no_edges)
        .expect_err("each catalog unit must retain a typed dependency edge array");

    let mut invalid_publication = catalog.clone();
    invalid_publication["units"][6]["publication"]["status"] =
        Value::String("candidate-unreleased".into());
    vexil_release_governance_validator::validate_catalog(&root, &invalid_publication)
        .expect_err("classification, target category, and status must be a valid combination");

    let mut escaped_changelog = catalog.clone();
    escaped_changelog["units"][6]["changelog"]["path"] =
        Value::String("crates/vexil-runtime/CHANGELOG.md".into());
    vexil_release_governance_validator::validate_catalog(&root, &escaped_changelog)
        .expect_err("a changelog path must remain under the unit source root");

    vexil_release_governance_validator::render_catalog_markdown(&root, &stale_target)
        .expect_err("catalog rendering must reject a semantically invalid catalog");
}

#[test]
fn version_rationales_are_per_unit_and_fail_closed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    vexil_release_governance_validator::validate_version_rationale_repository(&root)
        .expect("canonical version rationales must bind checked-in catalog versions only");

    let catalog: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/catalog.json")).expect("read canonical catalog"),
    )
    .expect("parse canonical catalog");
    let rationale: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/rationales/vexil-runtime-ts-0-4-1.json"))
            .expect("read canonical TypeScript runtime rationale"),
    )
    .expect("parse canonical TypeScript runtime rationale");
    vexil_release_governance_validator::validate_version_rationale(&root, &catalog, &rationale)
        .expect("the canonical rationale must bind one current publishable catalog unit");
    vexil_release_governance_validator::validate_version_rationale(&root, &catalog, &rationale)
        .expect("rationale validation must be repeatable");

    let mut shared_evidence = rationale.clone();
    shared_evidence["$id"] =
        Value::String("https://vexil.dev/release/rationales/vexil-runtime-py-0-1-0.json".into());
    shared_evidence["rationaleId"] = Value::String("vexil-runtime-py-0-1-0".into());
    shared_evidence["unitId"] = Value::String("vexil-runtime-py".into());
    shared_evidence["proposedPackageVersion"] = Value::String("0.1.0".into());
    shared_evidence["affectedSurfaces"][1]["surface"] =
        Value::String("vexil-runtime Python package API".into());
    shared_evidence["affectedSurfaces"][1]["authorityPath"] =
        Value::String("packages/runtime-py/pyproject.toml".into());
    shared_evidence["packageStewardReview"]["assignmentId"] =
        Value::String("assignment-package-steward-runtime-py-2026-07-14".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &shared_evidence,
    )
    .expect(
        "multiple units may reference one opaque evidence identity without selecting a release set",
    );

    let mut malformed_evidence = rationale.clone();
    malformed_evidence["compatibilityEvidenceIdentity"] = Value::String("sha256:ABC".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &malformed_evidence,
    )
    .expect_err("malformed compatibility evidence identities must fail closed");

    let mut unknown_unit = rationale.clone();
    unknown_unit["unitId"] = Value::String("missing-release-unit".into());
    vexil_release_governance_validator::validate_version_rationale(&root, &catalog, &unknown_unit)
        .expect_err("unknown catalog units must fail closed");

    let mut non_publishable_unit = rationale.clone();
    non_publishable_unit["unitId"] = Value::String("command-protocol-example".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &non_publishable_unit,
    )
    .expect_err("non-publishable catalog units cannot receive a release rationale");

    let mut mismatched_version = rationale.clone();
    mismatched_version["proposedPackageVersion"] = Value::String("99.99.99".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &mismatched_version,
    )
    .expect_err("rationales must not override checked-in version authority");

    let mut prior_published = rationale.clone();
    prior_published["previousPackageVersion"]["kind"] =
        Value::String("prior-published-package-version".into());
    prior_published["previousPackageVersion"]["version"] = Value::String("0.4.0".into());
    prior_published["changeClass"] = Value::String("patch-compatible".into());
    vexil_release_governance_validator::validate_version_rationale(&root, &catalog, &prior_published)
        .expect_err("prior published versions remain blocked until an explicit public provenance contract exists");

    let mut unordered_assessments = rationale.clone();
    unordered_assessments["affectedSurfaces"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &unordered_assessments,
    )
    .expect_err("affected-surface assessments must remain stable and independent");

    let mut duplicate_namespace = rationale.clone();
    duplicate_namespace["affectedSurfaces"][1]["namespace"] = Value::String("language-spec".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &duplicate_namespace,
    )
    .expect_err("each applicable namespace must have an independent assessment");

    let mut missing_namespace = rationale.clone();
    missing_namespace["affectedSurfaces"]
        .as_array_mut()
        .unwrap()
        .pop();
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &missing_namespace,
    )
    .expect_err("all three compatibility namespaces require independent assessment");

    let mut missing_review = rationale.clone();
    missing_review["packageStewardReview"] = Value::Null;
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &missing_review,
    )
    .expect_err("a Package Steward review is mandatory");

    let mut misattributed_review = rationale.clone();
    misattributed_review["packageStewardReview"]["actorId"] =
        Value::String("github:not-the-package-steward".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &misattributed_review,
    )
    .expect_err("a rationale review must be attributed to the unit Package Steward");

    let mut private_path = rationale.clone();
    private_path["affectedSurfaces"][0]["authorityPath"] =
        Value::String("C:\\Users\\private\\evidence.md".into());
    vexil_release_governance_validator::validate_version_rationale(&root, &catalog, &private_path)
        .expect_err("private or local authority paths must fail closed");

    let mut stale_authority_revision = rationale.clone();
    stale_authority_revision["affectedSurfaces"][0]["authorityRevision"] =
        Value::String("0000000000000000000000000000000000000000".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &stale_authority_revision,
    )
    .expect_err(
        "authority revisions must remain bound to the canonical catalog lifecycle revision",
    );

    let mut traversing_path = rationale.clone();
    traversing_path["affectedSurfaces"][0]["authorityPath"] =
        Value::String("spec/../spec/vexil-spec.md".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &traversing_path,
    )
    .expect_err("authority paths cannot traverse within or beyond a public authority root");

    let mut unsupported_matrix = rationale.clone();
    unsupported_matrix["supportMatrix"]["claims"][0]["evidenceIdentity"] = Value::Null;
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &unsupported_matrix,
    )
    .expect_err("support claims without evidence must fail closed");

    let mut contradictory_support = rationale.clone();
    let duplicate_claim = contradictory_support["supportMatrix"]["claims"][0].clone();
    contradictory_support["supportMatrix"]["claims"]
        .as_array_mut()
        .unwrap()
        .push(duplicate_claim);
    contradictory_support["supportMatrix"]["claims"][1]["compatibility"] =
        Value::String("unsupported".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &contradictory_support,
    )
    .expect_err("support matrices cannot contain conflicting duplicate platform claims");

    let mut draft_conformance = rationale.clone();
    draft_conformance["affectedSurfaces"][0]["assertion"] =
        Value::String("formal-conformance".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &draft_conformance,
    )
    .expect_err("draft language status cannot be elevated to formal conformance");

    let mut missing_decision = rationale.clone();
    missing_decision["affectedSurfaces"][0]["compatibility"] =
        Value::String("behavior-changed".into());
    missing_decision["publicCompatibilityDecision"] = Value::Null;
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &missing_decision,
    )
    .expect_err("behavior and public API changes require an approved public decision");

    let mut declared_change_without_surface_change = rationale.clone();
    declared_change_without_surface_change["changeClass"] = Value::String("behavior-change".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &declared_change_without_surface_change,
    )
    .expect_err("declared behavior changes cannot bypass the public decision requirement");

    let mut mismatched_public_id = rationale;
    mismatched_public_id["$id"] =
        Value::String("https://vexil.dev/release/rationales/some-other-rationale.json".into());
    vexil_release_governance_validator::validate_version_rationale(
        &root,
        &catalog,
        &mismatched_public_id,
    )
    .expect_err("public rationale IDs must match their rationale IDs");
}

#[test]
fn catalog_lifecycle_is_complete_and_fail_closed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    vexil_release_governance_validator::validate_catalog_lifecycle_repository(&root)
        .expect("the seeded active lifecycle ledger must agree with the canonical catalog");

    let catalog: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/catalog.json")).expect("read canonical catalog"),
    )
    .expect("parse canonical catalog");
    let lifecycle: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/catalog-lifecycle.json"))
            .expect("read canonical lifecycle ledger"),
    )
    .expect("parse canonical lifecycle ledger");
    vexil_release_governance_validator::validate_catalog_lifecycle(&root, &catalog, &lifecycle)
        .expect("lifecycle validation must be deterministic");

    let mut rename = lifecycle.clone();
    let mut legacy = rename["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["unitId"] == "vexil-runtime-ts")
        .unwrap()
        .clone();
    legacy["lifecycleRecordId"] = Value::String("lifecycle-vexil-runtime-ts-legacy".into());
    legacy["unitId"] = Value::String("vexil-runtime-ts-legacy".into());
    legacy["state"] = Value::String("renamed".into());
    legacy["sourceRoot"] = Value::String("packages/runtime-ts-legacy".into());
    legacy["canonicalTagNamespace"] = Value::String("vexil-runtime-ts-legacy-v<semver>".into());
    legacy["targetIdentities"][0]["name"] = Value::String("@vexil-lang/runtime-legacy".into());
    legacy["lifecycleDecision"]["decisionId"] =
        Value::String("catalog-lifecycle-vexil-runtime-ts-rename".into());
    legacy["lifecycleDecision"]["state"] = Value::String("approved-rename".into());
    legacy["successorUnitId"] = Value::String("vexil-runtime-ts".into());
    legacy["compatibilityImpact"]["state"] = Value::String("requires-rationale".into());
    legacy["compatibilityImpact"]["rationaleReference"] =
        Value::String("vexil-runtime-ts-0-4-1".into());
    legacy["compatibilityImpact"]["decisionState"] = Value::String("accepted".into());
    rename["records"].as_array_mut().unwrap().push(legacy);
    rename["records"]
        .as_array_mut()
        .unwrap()
        .sort_by(|left, right| left["unitId"].as_str().cmp(&right["unitId"].as_str()));
    for record in rename["records"].as_array_mut().unwrap() {
        if record["unitId"] == "vexil-runtime-ts" {
            record["predecessorUnitId"] = Value::String("vexil-runtime-ts-legacy".into());
            record["lifecycleDecision"]["decisionId"] =
                Value::String("catalog-lifecycle-vexil-runtime-ts-rename".into());
            record["lifecycleDecision"]["state"] = Value::String("approved-rename".into());
            record["compatibilityImpact"]["state"] = Value::String("requires-rationale".into());
            record["compatibilityImpact"]["rationaleReference"] =
                Value::String("vexil-runtime-ts-0-4-1".into());
            record["compatibilityImpact"]["decisionState"] = Value::String("accepted".into());
        }
    }
    vexil_release_governance_validator::validate_catalog_lifecycle(&root, &catalog, &rename)
        .expect("a reviewed rename must retain the old identity and explicit chain");

    let mut retirement = lifecycle.clone();
    let mut retired = retirement["records"][0].clone();
    retired["lifecycleRecordId"] = Value::String("lifecycle-zzz-retired".into());
    retired["unitId"] = Value::String("zzz-retired".into());
    retired["state"] = Value::String("retired".into());
    retired["sourceRoot"] = Value::String("crates/zzz-retired".into());
    retired["canonicalTagNamespace"] = Value::String("zzz-retired-v<semver>".into());
    retired["targetIdentities"][0]["kind"] = Value::String("cargo-package".into());
    retired["targetIdentities"][0]["name"] = Value::String("zzz-retired".into());
    retired["owner"] = serde_json::json!({"roleId":"package-steward","assignmentId":"assignment-package-steward-vexil-runtime-2026-07-14"});
    retired["publication"] = serde_json::json!({"classification":"non-publishable","targetCategory":"non-release","status":"non-publishable"});
    retired["lifecycleDecision"] = serde_json::json!({"decisionId":"catalog-lifecycle-zzz-retirement","state":"approved-retirement","effectiveRevision":"24a4d906d36e75c86e7d556ff07a093aa807b96c"});
    retired["stewardProposal"] = serde_json::json!({"actorId":"github:furkanmamuk","roleId":"package-steward","assignmentId":"assignment-package-steward-vexil-runtime-2026-07-14","proposedAt":"2026-07-24T00:00:00Z"});
    retired["releaseStewardAcceptance"]["roleAssertions"] =
        serde_json::json!(["package-steward", "release-steward"]);
    retired["compatibilityImpact"] = serde_json::json!({"state":"not-applicable","rationaleReference":null,"decisionState":"not-applicable"});
    retirement["records"].as_array_mut().unwrap().push(retired);
    vexil_release_governance_validator::validate_catalog_lifecycle(&root, &catalog, &retirement)
        .expect(
            "a reviewed retirement must preserve its former identity outside the active catalog",
        );

    let mut mismatched_decision = lifecycle.clone();
    mismatched_decision["records"][6]["lifecycleDecision"]["state"] =
        Value::String("approved-retirement".into());
    vexil_release_governance_validator::validate_catalog_lifecycle(
        &root,
        &catalog,
        &mismatched_decision,
    )
    .expect_err("lifecycle decisions must match their lifecycle state");

    let mut mismatched_decision_revision = lifecycle.clone();
    mismatched_decision_revision["records"][6]["lifecycleDecision"]["effectiveRevision"] =
        Value::String("0000000000000000000000000000000000000000".into());
    vexil_release_governance_validator::validate_catalog_lifecycle(
        &root,
        &catalog,
        &mismatched_decision_revision,
    )
    .expect_err("lifecycle decisions must retain their own effective revision");

    let mut unrelated_rationale = rename.clone();
    for record in unrelated_rationale["records"].as_array_mut().unwrap() {
        if record["unitId"] == "vexil-runtime-ts-legacy" {
            record["successorUnitId"] = Value::String("vexil-runtime".into());
        }
    }
    vexil_release_governance_validator::validate_catalog_lifecycle(
        &root,
        &catalog,
        &unrelated_rationale,
    )
    .expect_err("lifecycle transitions cannot use another unit's rationale");

    let mut missing_active = lifecycle.clone();
    missing_active["records"].as_array_mut().unwrap().pop();
    vexil_release_governance_validator::validate_catalog_lifecycle(
        &root,
        &catalog,
        &missing_active,
    )
    .expect_err("unrecorded active catalog units must require add/propose review");

    let mut ownerless = lifecycle.clone();
    ownerless["records"][0]["owner"]["assignmentId"] = Value::String("missing-owner".into());
    vexil_release_governance_validator::validate_catalog_lifecycle(&root, &catalog, &ownerless)
        .expect_err("ownerless lifecycle units must fail closed");

    let mut reused_target = lifecycle.clone();
    reused_target["records"][1]["targetIdentities"] =
        reused_target["records"][0]["targetIdentities"].clone();
    vexil_release_governance_validator::validate_catalog_lifecycle(&root, &catalog, &reused_target)
        .expect_err("target identities cannot be reused across lifecycle records");

    let mut misattributed = lifecycle.clone();
    misattributed["records"][6]["stewardProposal"]["actorId"] =
        Value::String("github:not-the-owner".into());
    vexil_release_governance_validator::validate_catalog_lifecycle(&root, &catalog, &misattributed)
        .expect_err("proposals and acceptance must be attributable to active role assertions");

    let mut stale_revision = lifecycle.clone();
    stale_revision["records"][6]["effectiveRevision"] =
        Value::String("0000000000000000000000000000000000000000".into());
    vexil_release_governance_validator::validate_catalog_lifecycle(
        &root,
        &catalog,
        &stale_revision,
    )
    .expect_err("stale effective revisions must fail closed");

    let mut publishability_change = lifecycle.clone();
    publishability_change["records"][6]["publication"]["status"] =
        Value::String("candidate-unreleased".into());
    vexil_release_governance_validator::validate_catalog_lifecycle(
        &root,
        &catalog,
        &publishability_change,
    )
    .expect_err("publishability changes require explicit reviewed lifecycle transitions");

    let mut missing_rationale = rename;
    for record in missing_rationale["records"].as_array_mut().unwrap() {
        if record["unitId"] == "vexil-runtime-ts" {
            record["compatibilityImpact"]["rationaleReference"] =
                Value::String("missing-rationale".into());
        }
    }
    vexil_release_governance_validator::validate_catalog_lifecycle(
        &root,
        &catalog,
        &missing_rationale,
    )
    .expect_err("compatibility transitions must reference a current rationale contract");

    let mut private_path = lifecycle;
    private_path["authorityBoundary"] = Value::String("C:\\Users\\private\\catalog.md".into());
    vexil_release_governance_validator::validate_catalog_lifecycle(&root, &catalog, &private_path)
        .expect_err("private local paths must never enter public lifecycle governance");
}

#[test]
fn typed_release_dependency_graph_is_manifest_led_and_deterministic() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/catalog.json")).expect("read canonical catalog"),
    )
    .expect("parse canonical catalog");

    let order = vexil_release_governance_validator::derive_release_order(&root, &catalog)
        .expect("the current manifest-backed graph must be valid");
    assert_eq!(
        order,
        vec![
            "vexil-lang",
            "vexil-codegen-go",
            "vexil-codegen-py",
            "vexil-codegen-rust",
            "vexil-codegen-ts",
            "vexil-runtime",
            "vexil-runtime-go",
            "vexil-runtime-ts",
            "vexil-store",
            "vexilc",
        ]
    );

    let unit = |id: &str| {
        catalog["units"]
            .as_array()
            .expect("catalog units")
            .iter()
            .find(|unit| unit["id"] == id)
            .expect("catalog unit")
    };
    let publish_before = catalog["units"]
        .as_array()
        .expect("catalog units")
        .iter()
        .flat_map(|unit| {
            unit["dependencyEdges"]
                .as_array()
                .expect("typed dependency edge array")
                .iter()
                .filter(move |edge| edge["edgeType"] == "publish_before")
                .map(move |edge| {
                    format!(
                        "{} -> {} @ {}#{}",
                        edge["relatedUnitId"].as_str().expect("related unit"),
                        unit["id"].as_str().expect("dependent unit"),
                        edge["sourceEvidence"]["path"]
                            .as_str()
                            .expect("evidence path"),
                        edge["sourceEvidence"]["location"]
                            .as_str()
                            .expect("evidence location"),
                    )
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        publish_before,
        vec![
            "vexil-lang -> vexil-codegen-go @ crates/vexil-codegen-go/Cargo.toml#dependencies.vexil-lang",
            "vexil-lang -> vexil-codegen-py @ crates/vexil-codegen-py/Cargo.toml#dependencies.vexil-lang",
            "vexil-lang -> vexil-codegen-rust @ crates/vexil-codegen-rust/Cargo.toml#dependencies.vexil-lang",
            "vexil-lang -> vexil-codegen-ts @ crates/vexil-codegen-ts/Cargo.toml#dependencies.vexil-lang",
            "vexil-lang -> vexil-store @ crates/vexil-store/Cargo.toml#dependencies.vexil-lang",
            "vexil-runtime -> vexil-store @ crates/vexil-store/Cargo.toml#dependencies.vexil-runtime",
            "vexil-codegen-go -> vexilc @ crates/vexilc/Cargo.toml#dependencies.vexil-codegen-go",
            "vexil-codegen-py -> vexilc @ crates/vexilc/Cargo.toml#dependencies.vexil-codegen-py",
            "vexil-codegen-rust -> vexilc @ crates/vexilc/Cargo.toml#dependencies.vexil-codegen-rust",
            "vexil-codegen-ts -> vexilc @ crates/vexilc/Cargo.toml#dependencies.vexil-codegen-ts",
            "vexil-lang -> vexilc @ crates/vexilc/Cargo.toml#dependencies.vexil-lang",
            "vexil-store -> vexilc @ crates/vexilc/Cargo.toml#dependencies.vexil-store",
        ]
    );
    assert!(unit("vexil-bench")["dependencyEdges"]
        .as_array()
        .expect("bench edges")
        .is_empty());
    assert!(unit("vexil-runtime-go")["dependencyEdges"]
        .as_array()
        .expect("Go edges")
        .is_empty());
    assert!(unit("vexil-runtime-py")["dependencyEdges"]
        .as_array()
        .expect("Python edges")
        .is_empty());
    assert!(unit("vexil-runtime-ts")["dependencyEdges"]
        .as_array()
        .expect("TypeScript edges")
        .is_empty());
    assert_eq!(
        unit("vexilc")["targets"]
            .as_array()
            .expect("vexilc targets")
            .len(),
        2,
        "the compiler remains one release unit with package and binary targets"
    );
    let retired_bot = fs::read_to_string(root.join(".vexilbot.toml"))
        .expect("read retained retired-bot evidence");
    assert!(
        !retired_bot.contains("vexil-codegen-py"),
        "the retained bot configuration deliberately omits the Python generator"
    );
    assert!(order.contains(&"vexil-codegen-py".to_owned()));
    assert_eq!(
        vexil_release_governance_validator::derive_release_order(&root, &catalog)
            .expect("the graph must be repeatable"),
        order
    );

    let mut missing_related_unit = catalog.clone();
    missing_related_unit["units"][6]["dependencyEdges"][0]["relatedUnitId"] =
        Value::String("missing-release-unit".into());
    assert!(
        vexil_release_governance_validator::validate_catalog(&root, &missing_related_unit)
            .expect_err("missing related units must block graph validation")
            .contains("missing related unit")
    );

    let mut unknown_type = catalog.clone();
    unknown_type["units"][6]["dependencyEdges"][0]["edgeType"] = Value::String("unknown".into());
    vexil_release_governance_validator::validate_catalog_schema(&root, &unknown_type)
        .expect_err("the typed-edge contract must reject unknown edge types");

    let mut duplicate = catalog.clone();
    let duplicated_edge = duplicate["units"][6]["dependencyEdges"][0].clone();
    duplicate["units"][6]["dependencyEdges"]
        .as_array_mut()
        .expect("typed edge array")
        .push(duplicated_edge);
    assert!(
        vexil_release_governance_validator::validate_catalog(&root, &duplicate)
            .expect_err("duplicate typed edges must be rejected")
            .contains("duplicate dependency edge")
    );

    let mut unordered = catalog.clone();
    unordered["units"][16]["dependencyEdges"]
        .as_array_mut()
        .expect("store typed edge array")
        .swap(0, 1);
    assert!(
        vexil_release_governance_validator::validate_catalog(&root, &unordered)
            .expect_err("typed edges must have stable order")
            .contains("stable sort order")
    );

    let mut stale_evidence = catalog.clone();
    stale_evidence["units"][6]["dependencyEdges"][0]["sourceEvidence"]["location"] =
        Value::String("dependencies.forged".into());
    assert!(
        vexil_release_governance_validator::validate_catalog(&root, &stale_evidence)
            .expect_err("catalog and manifest edge evidence must agree")
            .contains("missing manifest-derived")
    );

    let mut non_ordering_edges = catalog.clone();
    non_ordering_edges["units"][10]["dependencyEdges"] = serde_json::json!([
        {"edgeType":"bundle","relatedUnitId":"vexil-runtime","direction":"related-before-unit","sourceEvidence":{"sourceKind":"release-dependency-edge-decision","path":"release/decisions/missing.json","location":"bundle-1"}},
        {"edgeType":"compatibility","relatedUnitId":"vexil-runtime","direction":"related-before-unit","sourceEvidence":{"sourceKind":"release-dependency-edge-decision","path":"release/decisions/missing.json","location":"compatibility-1"}}
    ]);
    vexil_release_governance_validator::derive_release_order(&root, &non_ordering_edges)
        .expect_err("non-ordering edges must cite an approved public decision record");
}

#[test]
fn go_runtime_version_decision_establishes_one_checked_in_source() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let decision: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/decisions/runtime-go-version-2026-07-23.json"))
            .expect("the approved Go version decision must be public and checked in"),
    )
    .expect("the approved Go version decision must be JSON");
    assert_eq!(decision["status"], "approved");
    assert_eq!(decision["selectedVersion"], "0.1.0");
    assert_eq!(
        fs::read_to_string(root.join("packages/runtime-go/VERSION"))
            .expect("the approved Go version must have one checked-in source"),
        "0.1.0\n"
    );

    let catalog: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/catalog.json")).expect("read canonical catalog"),
    )
    .expect("parse canonical catalog");
    let go = catalog["units"]
        .as_array()
        .expect("catalog units")
        .iter()
        .find(|unit| unit["id"] == "vexil-runtime-go")
        .expect("Go runtime catalog unit");
    assert_eq!(go["publication"]["status"], "source-inventory-only");
    assert_eq!(go["versionSource"]["format"], "go-version-file");
    assert_eq!(go["versionSource"]["observedDeclaration"], "0.1.0");

    vexil_release_governance_validator::validate_candidate_tag(
        &root,
        &catalog,
        "packages/runtime-go/v0.1.0",
    )
    .expect("a canonical, new Go candidate tag must remain a pure structural check");
    vexil_release_governance_validator::validate_candidate_tag(
        &root,
        &catalog,
        "vexil-runtime-ts-v0.4.1",
    )
    .expect("a TypeScript candidate must match its checked-in source version");
    for candidate in [
        "v0.1.0",
        "vexil-codegen-go-v0.4.3",
        "vexil-runtime-ts-v0.4.2",
    ] {
        vexil_release_governance_validator::validate_candidate_tag(&root, &catalog, candidate)
            .expect_err("root, legacy, and historical candidate tags must fail closed");
    }
    let rust_runtime_collision = vexil_release_governance_validator::validate_candidate_tag(
        &root,
        &catalog,
        "vexil-runtime-v0.5.1",
    )
    .expect_err("the Rust runtime's canonical namespace must still reject Historical Tag reuse");
    assert!(
        rust_runtime_collision.contains("collides"),
        "the Rust runtime namespace must reach Historical Tag collision validation: {rust_runtime_collision}"
    );

    let mut duplicate_namespace = catalog.clone();
    let units = duplicate_namespace["units"]
        .as_array_mut()
        .expect("catalog units");
    let ts_namespace = units
        .iter()
        .find(|unit| unit["id"] == "vexil-runtime-ts")
        .expect("TypeScript runtime")["canonicalTagNamespace"]
        .clone();
    units
        .iter_mut()
        .find(|unit| unit["id"] == "vexil-runtime-go")
        .expect("Go runtime")["canonicalTagNamespace"] = ts_namespace;
    vexil_release_governance_validator::validate_catalog(&root, &duplicate_namespace)
        .expect_err("future canonical tag namespaces must remain unique");

    for invalid_version in [
        "",
        "\n",
        "0.1.0\n\n",
        " 0.1.0\n",
        "0.1.0 \n",
        "0.1\n",
        "01.1.0\n",
        "0.1.0\r\n",
    ] {
        vexil_release_governance_validator::strict_go_version_declaration(invalid_version)
            .expect_err("Go VERSION must be exactly one strict SemVer token followed by LF");
    }

    let mut stale_go_version = catalog.clone();
    let go_version_source = stale_go_version["units"]
        .as_array_mut()
        .expect("catalog units")
        .iter_mut()
        .find(|unit| unit["id"] == "vexil-runtime-go")
        .expect("Go runtime")
        .get_mut("versionSource")
        .expect("Go version source");
    go_version_source["observedDeclaration"] = Value::String("0.1.1".into());
    vexil_release_governance_validator::validate_catalog(&root, &stale_go_version)
        .expect_err("a stale Go VERSION observation must fail closed");

    for forbidden_authority in [
        "packages/runtime-go/go.mod",
        "packages/runtime-ts/package-lock.json",
        ".github/workflows/npm-publish.yml",
        "release/history/baseline-tags.json",
        "CHANGELOG.md",
        "C:/Users/private/VERSION",
    ] {
        let mut forged_authority = catalog.clone();
        let version_source = forged_authority["units"]
            .as_array_mut()
            .expect("catalog units")
            .iter_mut()
            .find(|unit| unit["id"] == "vexil-runtime-go")
            .expect("Go runtime")
            .get_mut("versionSource")
            .expect("Go version source");
        version_source["path"] = Value::String(forbidden_authority.into());
        vexil_release_governance_validator::validate_catalog(&root, &forged_authority)
            .expect_err("Go version authority must remain its checked-in VERSION file");
    }

    let lockfile = fs::read_to_string(root.join("packages/runtime-ts/package-lock.json"))
        .expect("read TypeScript lockfile");
    vexil_release_governance_validator::validate_typescript_lockfile_agreement(&lockfile, "0.4.1")
        .expect("TypeScript lockfile root declaration must agree with package.json");
    vexil_release_governance_validator::validate_typescript_lockfile_agreement(
        &lockfile.replacen("\"version\": \"0.4.1\"", "\"version\": \"0.4.2\"", 2),
        "0.4.1",
    )
    .expect_err("TypeScript lockfile disagreement must not become version authority");
    let mut rootless_lockfile: Value =
        serde_json::from_str(&lockfile).expect("parse TypeScript lockfile");
    rootless_lockfile["packages"]
        .as_object_mut()
        .expect("lockfile packages")
        .remove("");
    vexil_release_governance_validator::validate_typescript_lockfile_agreement(
        &serde_json::to_string(&rootless_lockfile).expect("serialize rootless lockfile"),
        "0.4.1",
    )
    .expect_err("TypeScript lockfiles without a root package declaration must fail closed");

    let vexilc_main = fs::read_to_string(root.join("crates/vexilc/src/main.rs"))
        .expect("read vexilc main source");
    vexil_release_governance_validator::validate_vexilc_version_display(&vexilc_main, "0.5.1")
        .expect("vexilc display must derive from its package version");
    vexil_release_governance_validator::validate_vexilc_version_display(
        &vexilc_main.replace("env!(\"CARGO_PKG_VERSION\")", "\"0.0.0\""),
        "0.5.1",
    )
    .expect_err("vexilc must not hard-code a displayed version");
    vexil_release_governance_validator::validate_vexilc_version_display(
        &vexilc_main.replace(
            "println!(\"vexilc {}\", env!(\"CARGO_PKG_VERSION\"));",
            "println!(\"vexilc 0.0.0\"); // println!(\"vexilc {}\", env!(\"CARGO_PKG_VERSION\"));",
        ),
        "0.5.1",
    )
    .expect_err("a comment must not satisfy the vexilc version display control");

    let npm_workflow = fs::read_to_string(root.join(".github/workflows/npm-publish.yml"))
        .expect("read publication-disabled npm workflow");
    vexil_release_governance_validator::validate_npm_publish_workflow_source(&npm_workflow)
        .expect("the canonical npm workflow must preserve its advisory boundary");
    vexil_release_governance_validator::validate_npm_publish_workflow_source(
        &npm_workflow.replace(
            "- \"vexil-runtime-ts-v*\"",
            "- \"vexil-runtime-v*\"\n      # - \"vexil-runtime-ts-v*\"",
        ),
    )
    .expect_err("a comment must not satisfy the canonical npm tag trigger control");
    vexil_release_governance_validator::validate_npm_publish_workflow_source(
        &npm_workflow.replace("- run: npm test", "- run: npm publish"),
    )
    .expect_err("the publication-disabled npm workflow must reject active publication commands");
}

#[test]
fn external_control_records_and_workflows_fail_closed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    vexil_release_governance_validator::validate_external_controls_repository(&root)
        .expect("canonical external-control records must validate");

    let current_path = root
        .join("release/controls/observations/current-github-controls-2026-07-22-sha-pinning.json");
    let current: Value = serde_json::from_str(&fs::read_to_string(current_path).unwrap()).unwrap();
    vexil_release_governance_validator::validate_current_observation_record(&root, &current)
        .expect("current observations must bind to exact expected controls");

    let mut mismatched_query = current.clone();
    mismatched_query["results"][0]["query"]["path"] = Value::String(
        "/repos/vexil-lang/vexil/branches/main/protection/required_pull_request_reviews".into(),
    );
    vexil_release_governance_validator::validate_current_observation_record(
        &root,
        &mismatched_query,
    )
    .expect_err("a current observation must not substitute a partial endpoint");

    let (provider, path) =
        vexil_release_governance_validator::expected_observation_query(&root, "EC-001")
            .expect("direct GitHub observation must resolve");
    assert_eq!(provider, "github");
    assert_eq!(path, "/repos/vexil-lang/vexil/branches/main/protection");
    vexil_release_governance_validator::expected_observation_query(&root, "EC-004")
        .expect_err("templated observation must require explicit target expansion");

    let authorized_path =
        root.join("release/controls/observations/owner-authorized-github-audit-2026-07-17.json");
    let authorized: Value =
        serde_json::from_str(&fs::read_to_string(authorized_path).unwrap()).unwrap();
    vexil_release_governance_validator::validate_external_observation_schema(&root, &authorized)
        .expect("the owner-authorized GET-only observation must validate");

    let mut missing_authorization = authorized.clone();
    missing_authorization["collection"]
        .as_object_mut()
        .unwrap()
        .remove("credentialAuthorization");
    vexil_release_governance_validator::validate_external_observation_schema(
        &root,
        &missing_authorization,
    )
    .expect_err("a write-capable observation credential must retain explicit owner authorization");

    let mut write_operation = authorized;
    write_operation["results"][0]["query"]["method"] = Value::String("POST".into());
    vexil_release_governance_validator::validate_external_observation_schema(
        &root,
        &write_operation,
    )
    .expect_err("an owner authorization cannot permit a provider write");

    let fixture_root =
        std::env::temp_dir().join(format!("vexil-workflow-isolation-{}", std::process::id()));
    let workflow_dir = fixture_root.join(".github/workflows");
    fs::create_dir_all(&workflow_dir).unwrap();
    fs::write(
        workflow_dir.join("privileged.yaml"),
        "name: privileged\npermissions:\n  issues: write\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n",
    )
    .unwrap();
    vexil_release_governance_validator::validate_workflow_static_isolation(&fixture_root)
        .expect_err("write-capable .yaml workflows must require immutable Action pins");
    fs::remove_dir_all(fixture_root).unwrap();

    let plan: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/exercises/revocation-exercise-plan.json")).unwrap(),
    )
    .unwrap();
    let result: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/exercises/revocation-exercise-result.json"))
            .unwrap(),
    )
    .unwrap();
    vexil_release_governance_validator::validate_revocation_exercise_pair(&plan, &result)
        .expect("the retained executed exercise must remain valid");

    let mut incomplete_success = result.clone();
    incomplete_success["evidence"]
        .as_object_mut()
        .unwrap()
        .remove("stableEvidenceDigest");
    vexil_release_governance_validator::validate_revocation_exercise_schema(
        &root,
        &incomplete_success,
    )
    .expect_err("an executed result must retain complete event evidence and a digest");
    vexil_release_governance_validator::validate_revocation_exercise_pair(
        &plan,
        &incomplete_success,
    )
    .expect_err("an executed result without a digest must fail pair validation");
}

#[test]
fn authority_schema_and_semantic_boundaries_fail_closed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let canonical: Value =
        serde_json::from_str(&fs::read_to_string(root.join("release/stewardship.json")).unwrap())
            .unwrap();

    let mut unknown_boundary = canonical.clone();
    unknown_boundary["privilegedAuthorization"]["typoedAuthority"] = Value::Bool(true);
    vexil_release_governance_validator::validate_contract_schema(&root, &unknown_boundary)
        .expect_err("authority schema must reject unknown authority-bearing fields");

    let mut empty_scope = canonical.clone();
    empty_scope["roles"][0]["decisionScope"] = Value::String(String::new());
    vexil_release_governance_validator::validate_contract(&empty_scope)
        .expect_err("roles must retain non-empty decision scopes");

    let mut privileged_advice = canonical;
    privileged_advice["advisoryAutomation"]["allowedActions"]
        .as_array_mut()
        .unwrap()
        .push(Value::String("authorize-privileged-release".into()));
    vexil_release_governance_validator::validate_contract(&privileged_advice)
        .expect_err("advisory automation must not gain privileged authority");
}

#[test]
fn exercise_schema_and_runbook_boundaries_fail_closed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut exercise: Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("release/exercises/tabletop-stewardship-continuity-2026-07-14.json"),
        )
        .unwrap(),
    )
    .unwrap();
    exercise["unexpected"] = Value::Bool(true);
    vexil_release_governance_validator::validate_stewardship_exercise_schema(&root, &exercise)
        .expect_err("exercise schema must reject unknown fields");

    let canonical = fs::read_to_string(root.join("release/runbooks/emergency-stop.md")).unwrap();
    let over_broad = canonical.replace(
        "| stop, revoke, contain |",
        "| stop, revoke, contain, approve-publication |",
    );
    vexil_release_governance_validator::validate_exercise_runbook_boundary(
        &over_broad,
        &["repository-administrator"],
        &["stop", "revoke", "contain", "activate-succession"],
        "fixture",
    )
    .expect_err("runbook must not gain publication authority");
}

#[test]
fn canonical_assignment_record_accepts_the_reviewed_sole_maintainer_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    vexil_release_governance_validator::validate_assignments_repository(&root)
        .expect("the canonical sole-maintainer decision must validate");
}

#[test]
fn unresolved_continuity_requires_a_public_recovery_contact_route() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut assignments: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/stewardship/assignments.json")).unwrap(),
    )
    .unwrap();
    assignments["continuity"]
        .as_object_mut()
        .unwrap()
        .remove("recoveryContact");
    vexil_release_governance_validator::validate_assignments(&assignments)
        .expect_err("unresolved continuity must expose the public recovery contact route");
}

#[test]
fn assignment_review_evidence_is_decision_bound_and_public() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let canonical: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/stewardship/assignments.json")).unwrap(),
    )
    .unwrap();

    let mut unrelated_decision = canonical.clone();
    unrelated_decision["assignments"][0]["reviewEvidence"]["decisionId"] =
        Value::String("unrelated-decision".into());
    vexil_release_governance_validator::validate_assignments(&unrelated_decision)
        .expect_err("assignment evidence must bind to the canonical decision");

    let mut private_evidence = canonical;
    private_evidence["assignments"][0]["reviewEvidence"]["source"] =
        Value::String("restricted-workspace-reference/private.md".into());
    vexil_release_governance_validator::validate_assignments(&private_evidence)
        .expect_err("assignment evidence must reject private planning sources");
}

#[test]
fn tabletop_exercise_fixtures_fail_closed_for_live_effects_and_missing_evidence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let assignments: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/stewardship/assignments.json")).unwrap(),
    )
    .unwrap();
    let canonical: Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("release/exercises/tabletop-stewardship-continuity-2026-07-14.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for fixture_path in fs::read_dir(root.join("release/validator/fixtures/exercises")).unwrap() {
        let fixture_path = fixture_path.unwrap().path();
        let fixture: Value =
            serde_json::from_str(&fs::read_to_string(&fixture_path).unwrap()).unwrap();
        let mut record = canonical.clone();
        apply_exercise_mutation(&mut record, fixture["mutation"].as_str().unwrap());
        let error = vexil_release_governance_validator::validate_stewardship_exercise(
            &record,
            &assignments,
        )
        .expect_err(&format!("fixture {} must fail", fixture_path.display()));
        assert!(
            error.contains(fixture["expectedReason"].as_str().unwrap()),
            "fixture {} failed with {error:?}",
            fixture_path.display()
        );
    }
}

#[test]
fn tabletop_runbooks_reject_live_commands_private_paths_and_missing_stop_conditions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let canonical = fs::read_to_string(root.join("release/runbooks/emergency-stop.md")).unwrap();
    for (mutation, expected) in [
        ("\n```sh\ngit tag v9.9.9\n```\n", "forbidden executable"),
        ("\ngh secret set RELEASE_TOKEN\n", "forbidden executable"),
        (
            "\nC:\\Users\\example\\private-note\n",
            "public/private boundary",
        ),
        ("", ""),
    ] {
        if mutation.is_empty() {
            let missing = canonical.replace("Stop condition", "Pause point");
            let error = vexil_release_governance_validator::validate_exercise_runbook_content(
                &missing,
                "emergency-stop-runbook",
                "fixture",
            )
            .expect_err("a runbook without a stop condition must fail");
            assert!(error.contains("missing a required decision point"));
        } else {
            let error = vexil_release_governance_validator::validate_exercise_runbook_content(
                &(canonical.clone() + mutation),
                "emergency-stop-runbook",
                "fixture",
            )
            .expect_err("unsafe runbook mutation must fail");
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }
}

#[test]
fn assignment_fixtures_cover_continuity_and_publication_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let canonical: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/stewardship/assignments.json")).unwrap(),
    )
    .unwrap();
    for fixture_path in fs::read_dir(root.join("release/validator/fixtures/assignments")).unwrap() {
        let fixture_path = fixture_path.unwrap().path();
        let fixture: Value =
            serde_json::from_str(&fs::read_to_string(&fixture_path).unwrap()).unwrap();
        let expected_valid = fixture["valid"].as_bool().unwrap();
        let mut record = canonical.clone();
        if let Some(mutation) = fixture.get("mutation").and_then(Value::as_str) {
            apply_assignment_mutation(&mut record, mutation);
        }
        let outcome = vexil_release_governance_validator::validate_assignments(&record);
        if expected_valid {
            outcome.expect("positive assignment fixture must validate");
        } else {
            let error = outcome.expect_err(&format!(
                "negative assignment fixture must fail: {}",
                fixture_path.display()
            ));
            assert!(
                error.contains(fixture["expectedReason"].as_str().unwrap()),
                "fixture {} failed with {error:?}",
                fixture_path.display()
            );
        }
    }
}

#[test]
fn negative_fixtures_fail_for_their_intended_boundary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let canonical: Value =
        serde_json::from_str(&fs::read_to_string(root.join("release/stewardship.json")).unwrap())
            .unwrap();
    for fixture_path in fs::read_dir(root.join("release/validator/fixtures/negative")).unwrap() {
        let fixture_path = fixture_path.unwrap().path();
        let fixture: Value =
            serde_json::from_str(&fs::read_to_string(&fixture_path).unwrap()).unwrap();
        let mutation = fixture["mutation"].as_str().unwrap();
        let expected = fixture["expectedReason"].as_str().unwrap();
        let outcome = match mutation {
            "stale-markdown" => vexil_release_governance_validator::validate_documentation_parity(
                &canonical,
                "stale documentation",
            ),
            "private-absolute-path" => {
                let leaked_path = ["C:", "Users", "example", "workspace-temp"].join("\\");
                vexil_release_governance_validator::ensure_no_private_leakage(&leaked_path)
            }
            _ => {
                let mut record = canonical.clone();
                apply_mutation(&mut record, mutation);
                vexil_release_governance_validator::validate_contract(&record)
            }
        };
        let error = outcome.expect_err(&format!("fixture {} must fail", fixture_path.display()));
        assert!(
            error.contains(expected),
            "fixture {} failed with {error:?}, expected {expected:?}",
            fixture_path.display()
        );
    }
}

#[test]
fn responsibility_fixtures_fail_closed_for_inventory_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let canonical: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/stewardship/responsibilities.json")).unwrap(),
    )
    .unwrap();
    for fixture_path in
        fs::read_dir(root.join("release/validator/fixtures/responsibilities")).unwrap()
    {
        let fixture_path = fixture_path.unwrap().path();
        let fixture: Value =
            serde_json::from_str(&fs::read_to_string(&fixture_path).unwrap()).unwrap();
        let mut record = canonical.clone();
        apply_responsibility_mutation(&mut record, fixture["mutation"].as_str().unwrap());
        let outcome = vexil_release_governance_validator::validate_responsibilities(&record);
        if fixture["valid"].as_bool() == Some(true) {
            outcome.unwrap_or_else(|error| {
                panic!("fixture {} must validate: {error}", fixture_path.display())
            });
        } else {
            let error =
                outcome.expect_err(&format!("fixture {} must fail", fixture_path.display()));
            assert!(
                error.contains(fixture["expectedReason"].as_str().unwrap()),
                "fixture {} failed with {error:?}",
                fixture_path.display()
            );
        }
    }
}

#[test]
fn responsibility_inventory_normalization_is_deterministic_and_non_duplicating() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let canonical: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/stewardship/responsibilities.json")).unwrap(),
    )
    .unwrap();
    let mut unordered = canonical.clone();
    unordered["responsibilities"]
        .as_array_mut()
        .unwrap()
        .reverse();
    unordered["manifestComparison"]["mismatches"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let first = vexil_release_governance_validator::normalize_responsibility_inventory(&unordered)
        .expect("normalization must accept collectable inventory input");
    let second = vexil_release_governance_validator::normalize_responsibility_inventory(&unordered)
        .expect("repeated normalization must accept unchanged input");
    assert_eq!(first, second, "unchanged input must normalize identically");
    assert_eq!(
        first, canonical,
        "normalization must restore canonical ordering without duplicates"
    );
    vexil_release_governance_validator::validate_responsibilities(&first)
        .expect("normalized inventory must validate");
}

#[test]
fn privileged_operations_fail_closed_for_all_required_gates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let responsibilities: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/stewardship/responsibilities.json")).unwrap(),
    )
    .unwrap();
    let assignments: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/stewardship/assignments.json")).unwrap(),
    )
    .unwrap();
    let canonical: Value = serde_json::from_str(
        &fs::read_to_string(root.join("release/privileged/operations-contract.json")).unwrap(),
    )
    .unwrap();
    for fixture_path in fs::read_dir(root.join("release/validator/fixtures/privileged")).unwrap() {
        let fixture_path = fixture_path.unwrap().path();
        let fixture: Value =
            serde_json::from_str(&fs::read_to_string(&fixture_path).unwrap()).unwrap();
        let mut operations = canonical.clone();
        apply_privileged_mutation(&mut operations, fixture["mutation"].as_str().unwrap());
        let outcome = vexil_release_governance_validator::validate_privileged_operations(
            &operations,
            &responsibilities,
            &assignments,
        );
        if fixture["valid"].as_bool() == Some(true) {
            outcome.unwrap_or_else(|error| {
                panic!("fixture {} must validate: {error}", fixture_path.display())
            });
        } else {
            let error =
                outcome.expect_err(&format!("fixture {} must fail", fixture_path.display()));
            assert!(
                error.contains(fixture["expectedReason"].as_str().unwrap()),
                "fixture {} failed with {error:?}",
                fixture_path.display()
            );
        }
    }
}

#[test]
fn isolated_public_copy_needs_no_non_public_workspace_directory() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let isolated = std::env::temp_dir().join(format!("vexil-stewardship-{}", std::process::id()));
    let _ = fs::remove_dir_all(&isolated);
    fs::create_dir_all(isolated.join("release/schemas")).unwrap();
    fs::create_dir_all(isolated.join("release/stewardship")).unwrap();
    fs::create_dir_all(isolated.join("release/runbooks")).unwrap();
    fs::create_dir_all(isolated.join("release/advisory")).unwrap();
    fs::create_dir_all(isolated.join("release/privileged")).unwrap();
    fs::create_dir_all(isolated.join("release/exercises")).unwrap();
    fs::create_dir_all(isolated.join("release/controls/observations")).unwrap();
    fs::create_dir_all(isolated.join("release/identities")).unwrap();
    fs::create_dir_all(isolated.join("release/history/ratifications")).unwrap();
    fs::create_dir_all(isolated.join("release/history/observations")).unwrap();
    fs::create_dir_all(isolated.join("release/history/entries")).unwrap();
    fs::create_dir_all(isolated.join("release/history/repair-proposals")).unwrap();
    fs::create_dir_all(isolated.join("release/decisions")).unwrap();
    fs::create_dir_all(isolated.join("release/rationales")).unwrap();
    fs::create_dir_all(isolated.join("docs/book/src/release")).unwrap();
    fs::create_dir_all(isolated.join(".github/workflows")).unwrap();
    fs::create_dir_all(isolated.join("packages/runtime-go")).unwrap();
    fs::create_dir_all(isolated.join("schemas/vexil")).unwrap();
    fs::create_dir_all(isolated.join("spec")).unwrap();
    for relative in [
        "Cargo.toml",
        "Cargo.lock",
        "release/stewardship.json",
        "release/schemas/stewardship.schema.json",
        "release/schemas/stewardship-assignment.schema.json",
        "release/schemas/retired-bot-responsibility.schema.json",
        "release/schemas/privileged-operation.schema.json",
        "release/schemas/stewardship-exercise.schema.json",
        "release/schemas/external-control.schema.json",
        "release/schemas/external-observation.schema.json",
        "release/schemas/external-remediation.schema.json",
        "release/schemas/identity-custody.schema.json",
        "release/schemas/revocation-exercise.schema.json",
        "release/schemas/history-baseline.schema.json",
        "release/schemas/history-ratification.schema.json",
        "release/schemas/history-observation-sources.schema.json",
        "release/schemas/history-observation.schema.json",
        "release/schemas/history-ledger-entry.schema.json",
        "release/schemas/additive-repair-proposal.schema.json",
        "release/schemas/history-reconciliation-decision.schema.json",
        "release/schemas/catalog.schema.json",
        "release/schemas/catalog-lifecycle.schema.json",
        "release/schemas/version-rationale.schema.json",
        "release/schemas/release-manifest-1.0.schema.json",
        "release/schemas/release-manifest-1.1.schema.json",
        "release/schemas/security-scan-1.0.schema.json",
        "release/schemas/release-evidence-set-1.0.schema.json",
        "release/schemas/release-detached-approval-1.0.schema.json",
        "release/schemas/release-approval-disposition-1.0.schema.json",
        "release/schemas/privileged-run-start-authorization-1.0.schema.json",
        "release/schemas/release-adapter-result-envelope-1.0.schema.json",
        "release/schemas/release-run-event-1.0.schema.json",
        "release/schemas/release-run-evidence-1.0.schema.json",
        "release/schemas/release-closeout-1.0.schema.json",
        "release/catalog.json",
        "release/catalog-lifecycle.json",
        "release/rationales/vexil-runtime-ts-0-4-1.json",
        "release/decisions/runtime-go-version-2026-07-23.json",
        "release/stewardship/assignments.json",
        "release/stewardship/responsibilities.json",
        "release/advisory/automation-contract.json",
        "release/privileged/operations-contract.json",
        "release/exercises/tabletop-stewardship-continuity-2026-07-14.json",
        "release/exercises/revocation-exercise-plan.json",
        "release/exercises/revocation-exercise-result.json",
        "release/controls/expected-controls.json",
        "release/controls/observations/baseline-2026-07-13.json",
        "release/controls/remediation-plan-github-protections.json",
        "release/identities/custody.json",
        "release/security/inventory.toml",
        "release/security/scan-coverage.md",
        "release/security/scans/npm-runtime-ts-2026-07-25.json",
        "release/history/baseline-tags.json",
        "release/history/ratifications/history-ratification-release-steward-2026-07-23.json",
        "release/history/ratifications/history-ratification-repository-administrator-2026-07-23.json",
        "release/history/observations/observation-root-changelog-v1-0-0-2026-07-23.json",
        "release/history/observations/observation-github-releases-2026-07-23.json",
        "release/history/observations/observation-release-artifacts-2026-07-23.json",
        "release/history/observations/observation-crates-io-2026-07-23.json",
        "release/history/observations/observation-npm-runtime-2026-07-23.json",
        "release/history/observations/observation-package-manifests-2026-07-23.json",
        "release/history/observations/observation-pypi-vexil-runtime-2026-07-23.json",
        "release/history/observations/observation-go-proxy-runtime-2026-07-23.json",
        "release/history/entries/entry-root-v1-0-0-changelog-anomaly-2026-07-23.json",
        "release/history/entries/entry-release-surface-observations-2026-07-23.json",
        "release/history/observation-sources.json",
        "release/history/additive-repair-policy.json",
        "release/history/reconciliation-decision.json",
        "release/history/ledger.md",
        "docs/book/src/release/stewardship.md",
        "docs/book/src/release/stewardship-continuity.md",
        "docs/book/src/release/retired-bot-responsibilities.md",
        "docs/book/src/release/advisory-automation.md",
        "docs/book/src/release/privileged-operations.md",
        "docs/book/src/release/stewardship-exercises.md",
        "docs/book/src/release/catalog.md",
        "docs/book/src/release/canonical-records.md",
        "docs/book/src/SUMMARY.md",
        "release/runbooks/advisory-automation.md",
        "release/runbooks/privileged-readiness-and-fail-closed.md",
        "release/runbooks/stewardship-succession.md",
        "release/runbooks/unavailable-owner.md",
        "release/runbooks/emergency-stop.md",
        "release/runbooks/trust-revocation.md",
        "release/runbooks/advisory-manual-fallback.md",
        "GOVERNANCE.md",
        ".github/workflows/release.yml",
        ".github/workflows/npm-publish.yml",
        "crates/vexil-lang/Cargo.toml",
        "crates/vexilc/Cargo.toml",
        "crates/vexilc/src/main.rs",
        "crates/vexil-runtime/Cargo.toml",
        "crates/vexil-codegen-rust/Cargo.toml",
        "crates/vexil-codegen-ts/Cargo.toml",
        "crates/vexil-codegen-go/Cargo.toml",
        "crates/vexil-codegen-py/Cargo.toml",
        "crates/vexil-store/Cargo.toml",
        "crates/vexil-bench/Cargo.toml",
        "packages/runtime-ts/package.json",
        "packages/runtime-ts/package-lock.json",
        "packages/runtime-py/pyproject.toml",
        "packages/runtime-go/go.mod",
        "packages/runtime-go/VERSION",
        "schemas/vexil/schema.vexil",
        "spec/vexil-spec.md",
        "examples/command-protocol/Cargo.toml",
        "examples/cross-language/rust-device/Cargo.toml",
        "examples/multi-file-project/Cargo.toml",
        "examples/sensor-packet/Cargo.toml",
        "examples/system-monitor/Cargo.toml",
        "release/validator/Cargo.toml",
        "crates/vexil-codegen-go/CHANGELOG.md",
        "crates/vexil-codegen-rust/CHANGELOG.md",
        "crates/vexil-codegen-ts/CHANGELOG.md",
        "crates/vexil-lang/CHANGELOG.md",
        "crates/vexil-runtime/CHANGELOG.md",
        "crates/vexil-store/CHANGELOG.md",
        "crates/vexilc/CHANGELOG.md",
    ] {
        let destination = isolated.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(root.join(relative), destination).unwrap();
    }
    vexil_release_governance_validator::validate_repository(&isolated)
        .expect("isolated public copy must validate");
    let isolated_catalog: Value = serde_json::from_str(
        &fs::read_to_string(isolated.join("release/catalog.json"))
            .expect("read isolated canonical catalog"),
    )
    .expect("parse isolated canonical catalog");
    vexil_release_governance_validator::validate_candidate_tag(
        &isolated,
        &isolated_catalog,
        "vexil-runtime-ts-v0.4.1",
    )
    .expect("candidate validation must accept a source-matching nonhistorical tag");
    let baseline_path = isolated.join("release/history/baseline-tags.json");
    let invalid_baseline = fs::read_to_string(&baseline_path)
        .expect("read isolated Historical Tag baseline")
        .replacen(
            "\"status\": \"ratified\"",
            "\"status\": \"awaiting-ratification\"",
            1,
        );
    fs::write(&baseline_path, invalid_baseline)
        .expect("write invalid isolated Historical Tag baseline");
    vexil_release_governance_validator::validate_candidate_tag(
        &isolated,
        &isolated_catalog,
        "vexil-runtime-ts-v0.4.1",
    )
    .expect_err("candidate validation must reject an unratified Historical Tag baseline");
    fs::remove_dir_all(isolated).unwrap();
}

fn apply_privileged_mutation(record: &mut Value, mutation: &str) {
    let operations = record["operations"].as_array_mut().unwrap();
    match mutation {
        "valid" => {}
        "duplicate-disposition" => {
            let mut duplicate = operations[0].clone();
            duplicate["id"] = Value::String("privileged-operation-rbr-003-duplicate".into());
            operations.push(duplicate);
        }
        "missing-manifest" => {
            operations[0]["requiredInputs"]
                .as_object_mut()
                .unwrap()
                .remove("manifestDigest");
        }
        "missing-protected-identity" => {
            operations[0]["target"]
                .as_object_mut()
                .unwrap()
                .remove("protectedAuthority");
        }
        "missing-external-control" => {
            operations[0]["requiredInputs"]["futureControls"] =
                serde_json::json!(["later evidence"]);
        }
        "shared-advisory-credential" => {
            operations[0]["hybridBoundary"] =
                Value::String("Advisory stages may use the privileged credential.".into());
        }
        "broad-pat" => {
            operations[0]["authentication"]["personalAccessTokens"] =
                Value::String("allowed".into());
        }
        "expired-bootstrap" => {
            operations[0]["authentication"]["bootstrapException"] = serde_json::json!({"status":"approved","targetScope":"one target","custodian":"github:furkanmamuk","expiresOn":"2025-01-01","revocationPath":"public runbook","auditSurface":"public audit"});
        }
        "effect-after-failed-readiness" => {
            operations[0]["effectPolicy"] =
                Value::String("An effect is permitted while blocked.".into());
        }
        "broad-administration-permission" => {
            operations[0]["minimumPermissions"] =
                Value::Array(vec![Value::String("administration:write".into())]);
        }
        "wrong-owner-role" => {
            operations[0]["owner"]["roleId"] = Value::String("repository-administrator".into());
            operations[0]["owner"]["assignmentId"] =
                Value::String("assignment-repository-administrator-2026-07-14".into());
        }
        "mismatched-target-input" => {
            operations[0]["requiredInputs"]["targetIdentity"] =
                Value::String("repository:vexil-lang/other-target".into());
        }
        other => panic!("unknown privileged mutation: {other}"),
    }
}

fn apply_exercise_mutation(record: &mut Value, mutation: &str) {
    match mutation {
        "missing-follow-up-owner" => {
            record["scenarios"][0]
                .as_object_mut()
                .unwrap()
                .remove("followUpOwner");
        }
        "over-broad-emergency-action" => {
            record["scenarios"][0]["allowedActions"]
                .as_array_mut()
                .unwrap()
                .push(Value::String("approve-publication".into()));
        }
        "false-provider-compliance" => {
            record["scenarios"][0]["providerBlockers"][0]["status"] =
                Value::String("tested-compliant".into());
        }
        "secret-in-evidence" => {
            record["evidence"]["secretsIncluded"] = Value::Bool(true);
        }
        "ephemeral-evidence" => {
            record["evidence"]["persistence"] = Value::String("ephemeral-chat-log".into());
        }
        "stale-assignment-link" => {
            record["participants"][0]["assignmentId"] = Value::String("assignment-stale".into());
        }
        "empty-participants" => {
            record["participants"] = Value::Array(vec![]);
        }
        "duplicate-scenario-id" => {
            record["scenarios"][1]["id"] = record["scenarios"][0]["id"].clone();
        }
        "swapped-procedure" => {
            record["scenarios"][0]["procedureId"] = Value::String("emergency-stop-runbook".into());
        }
        "missing-prohibited-boundary" => {
            record["scenarios"][0]["prohibitedActions"]
                .as_array_mut()
                .unwrap()
                .retain(|value| value != "approve-publication");
        }
        other => panic!("unknown exercise mutation: {other}"),
    }
}

fn apply_responsibility_mutation(record: &mut Value, mutation: &str) {
    let root = record.as_object_mut().unwrap();
    match mutation {
        "missing-required-class" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .retain(|entry| entry["responsibilityClass"] != "manual-fallback-knowledge");
        }
        "missing-codegen-py-discrepancy" => {
            root.get_mut("manifestComparison").unwrap()["mismatches"]
                .as_array_mut()
                .unwrap()
                .retain(|entry| entry["unit"] != "crates/vexil-codegen-py");
        }
        "private-evidence" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[0]["historicalEvidence"][0]["source"] =
                Value::String("restricted-workspace-reference/private-note.md".into());
        }
        "duplicate-id" => {
            let responsibilities = root
                .get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap();
            responsibilities[1]["id"] = responsibilities[0]["id"].clone();
        }
        "falsely-retired" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[0]["dispositionStatus"] = Value::String("retired".into());
        }
        "valid-maintained-replacement" | "valid-manual-procedure" | "valid-approved-retirement" => {
        }
        "duplicate-disposition" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[4]["advisoryDisposition"]["retirement"] = serde_json::json!({});
        }
        "unknown-disposition" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[4]["dispositionStatus"] = Value::String("unknown-disposition".into());
        }
        "privileged-permission" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[4]["advisoryDisposition"]["minimumPermissions"]
                .as_array_mut()
                .unwrap()
                .push(Value::String("contents:write".into()));
        }
        "missing-fallback" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[4]["advisoryDisposition"]
                .as_object_mut()
                .unwrap()
                .remove("fallback");
        }
        "missing-owner" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[4]["advisoryDisposition"]
                .as_object_mut()
                .unwrap()
                .remove("owner");
        }
        "missing-audit-evidence" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[4]["advisoryDisposition"]
                .as_object_mut()
                .unwrap()
                .remove("auditEvidence");
        }
        "retirement-without-accepted-decision" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[6]["advisoryDisposition"]["retirement"]["publicDecision"]["status"] =
                Value::String("proposed".into());
        }
        "fallback-reaches-privileged-effects" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[4]["advisoryDisposition"]["fallback"]["noPrivilegeBoundary"] =
                Value::String("Fallback can publish after a job failure.".into());
        }
        "private-advisory-evidence" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[4]["advisoryDisposition"]["auditEvidence"] =
                Value::String("C:\\Users\\example\\private-evidence.md".into());
        }
        "advisory-undispositioned" => {
            root.get_mut("responsibilities")
                .unwrap()
                .as_array_mut()
                .unwrap()[4]["dispositionStatus"] = Value::String("undispositioned".into());
        }
        other => panic!("unknown responsibility mutation: {other}"),
    }
}

fn apply_assignment_mutation(record: &mut Value, mutation: &str) {
    let root = record.as_object_mut().unwrap();
    match mutation {
        "missing-required-role" => {
            root.get_mut("assignments")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .retain(|assignment| assignment["roleId"] != "release-run-coordinator");
        }
        "invented-primary" => {
            root.get_mut("assignments").unwrap().as_array_mut().unwrap()[0]["primaryActorId"] =
                Value::String("github:unresolved".into());
        }
        "scope-less" => {
            root.get_mut("assignments").unwrap().as_array_mut().unwrap()[0]
                .as_object_mut()
                .unwrap()
                .remove("scope");
        }
        "combined-role-escalation" => {
            root.get_mut("assignments").unwrap().as_array_mut().unwrap()[0]["permittedActions"] =
                Value::Array(vec![Value::String("approve-publication".into())]);
        }
        "unresolved-continuity" => {
            root.get_mut("decision").unwrap()["status"] =
                Value::String("unresolved-continuity".into());
            root.get_mut("continuity").unwrap()["recoveryContact"]["status"] =
                Value::String("unresolved-no-distinct-custodian".into());
            root.get_mut("publicationReadiness").unwrap()["reason"] =
                Value::String("The unresolved continuity gate blocks Manifest approval and privileged publication.".into());
        }
        "invented-custodian-under-sole-maintainer" => {
            root.get_mut("continuity").unwrap()["custodian"] = serde_json::json!({
                "actorId":"github:furkanmamuk",
                "nonPublishingCapabilities":["recover-administration","stop-automation","revoke-trust","initiate-succession"],
                "hasNormalPublicationCredential":false
            });
        }
        "publishing-custodian" | "valid-single-steward-custodian" => {
            root.get_mut("decision").unwrap()["status"] =
                Value::String("single-steward-custodian".into());
            root.get_mut("continuity").unwrap()["recoveryContact"]["status"] =
                Value::String("unresolved-no-distinct-custodian".into());
            root.get_mut("identities").unwrap().as_array_mut().unwrap().push(serde_json::json!({
                "id":"github:recovery-custodian", "name":"Recovery Custodian", "email":"recovery@example.test", "github":"recovery-custodian"
            }));
            root.get_mut("continuity").unwrap()["custodian"] = serde_json::json!({
                "actorId":"github:recovery-custodian",
                "nonPublishingCapabilities":["recover-administration","stop-automation","revoke-trust","initiate-succession"],
                "hasNormalPublicationCredential": mutation == "publishing-custodian"
            });
        }
        "self-approved-detached-approval" => {
            root.get_mut("decision").unwrap()["status"] =
                Value::String("multi-steward-detached-approval".into());
            root.get_mut("continuity").unwrap()["recoveryContact"]["status"] =
                Value::String("unresolved-no-distinct-custodian".into());
            root.get_mut("identities").unwrap().as_array_mut().unwrap().push(serde_json::json!({
                "id":"github:second-steward", "name":"Second Steward", "email":"second@example.test", "github":"second-steward"
            }));
            root.get_mut("continuity").unwrap()["qualifiedReleaseStewardActorIds"] =
                serde_json::json!(["github:furkanmamuk", "github:second-steward"]);
            root.get_mut("assignments")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "assignmentId":"assignment-second-release-steward",
                    "roleId":"release-steward",
                    "primaryActorId":"github:second-steward",
                    "scope":{"kind":"release-manifest-lifecycle","root":"release-manifests"},
                    "effectiveFrom":"2026-07-14",
                    "reviewEvidence":{"decisionId":"sole-maintainer-governance-2026-07-23","source":"https://github.com/vexil-lang/vexil/issues/75","reviewedBy":"github:furkanmamuk","reviewedAt":"2026-07-23"},
                    "continuityProcedure":"release-continuity-runbook",
                    "status":"active"
                }));
            root.get_mut("continuity").unwrap()["detachedApproval"] = serde_json::json!({
                "status":"mandatory", "manifestApproverActorId":"github:furkanmamuk", "detachedApproverActorId":"github:furkanmamuk", "rule":"Identity distinction is mandatory."
            });
        }
        "private-evidence" => {
            root.get_mut("decision").unwrap()["reviewEvidence"]["source"] =
                Value::String("C:\\Users\\example\\workspace-temp".into());
        }
        "qualified-non-steward" => {
            root.get_mut("identities")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "id":"github:governed-observer", "name":"Governed Observer", "email":"observer@example.test", "github":"governed-observer"
                }));
            root.get_mut("continuity").unwrap()["qualifiedReleaseStewardActorIds"] =
                serde_json::json!(["github:furkanmamuk", "github:governed-observer"]);
        }
        "unavailable-owner-authorizes-release" => {
            root.get_mut("continuity").unwrap()["unavailableOwnerRoute"]["allowedActions"]
                .as_array_mut()
                .unwrap()
                .push(Value::String("authorize-privileged-release".into()));
        }
        "missing-maintained-root" => {
            root.get_mut("assignments")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .retain(|assignment| assignment["scope"]["root"] != "packages/runtime-go");
        }
        "vague-package-scope" => {
            let package = root
                .get_mut("assignments")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|assignment| assignment["roleId"] == "package-steward")
                .unwrap();
            package["scope"]["root"] = Value::String("*".into());
        }
        other => panic!("unknown assignment mutation: {other}"),
    }
}

fn apply_mutation(record: &mut Value, mutation: &str) {
    let root = record.as_object_mut().unwrap();
    match mutation {
        "missing-role" => {
            root.get_mut("roles").unwrap().as_array_mut().unwrap().pop();
        }
        "missing-boundary-field" => {
            root.get_mut("roles").unwrap().as_array_mut().unwrap()[0]
                .as_object_mut()
                .unwrap()
                .remove("auditSurface");
        }
        "unknown-action" => {
            root.get_mut("roles").unwrap().as_array_mut().unwrap()[0]["permittedActions"]
                .as_array_mut()
                .unwrap()
                .push(Value::String("approve-relase-manifest".into()));
        }
        "non-authority-release-authority" => {
            root.get_mut("privilegedAuthorization").unwrap()["requiredRole"] =
                Value::String("bot".into());
        }
        "advisory-automation-privileged-action" => {
            root.get_mut("advisoryAutomation").unwrap()["prohibitedActions"]
                .as_array_mut()
                .unwrap()
                .retain(|value| value != "deploy");
        }
        "over-broad-emergency-authority" => {
            root.get_mut("roles").unwrap().as_array_mut().unwrap()[1]["permittedActions"]
                .as_array_mut()
                .unwrap()
                .push(Value::String("execute-authorized-release-action".into()));
        }
        "combined-role-without-assertion" => {
            root.get_mut("privilegedAuthorization").unwrap()["requiredRoleAssertion"] =
                Value::String("implicit combined role".into());
        }
        "embedded-role-assignment" => {
            root.insert("assignments".into(), Value::Array(vec![]));
        }
        "governance-bypass" => {
            root.get_mut("governanceRoute").unwrap()["nonBypassStatement"] =
                Value::String("This record may bypass existing governance.".into());
        }
        other => panic!("unknown fixture mutation: {other}"),
    }
}
