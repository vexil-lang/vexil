use serde_json::Value;
use std::fs;
use std::path::Path;

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
