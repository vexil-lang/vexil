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
fn external_control_records_and_workflows_fail_closed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    vexil_release_governance_validator::validate_external_controls_repository(&root)
        .expect("canonical Epic 2 offline records must validate");

    let current_path =
        root.join("release/controls/observations/current-github-controls-2026-07-17.json");
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
fn canonical_assignment_record_fails_closed_for_unresolved_continuity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    vexil_release_governance_validator::validate_assignments_repository(&root)
        .expect("the canonical unresolved-continuity decision must validate");
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
    fs::create_dir_all(isolated.join("docs/book/src/release")).unwrap();
    fs::create_dir_all(isolated.join(".github/workflows")).unwrap();
    for relative in [
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
        "docs/book/src/release/stewardship.md",
        "docs/book/src/release/stewardship-continuity.md",
        "docs/book/src/release/retired-bot-responsibilities.md",
        "docs/book/src/release/advisory-automation.md",
        "docs/book/src/release/privileged-operations.md",
        "docs/book/src/release/stewardship-exercises.md",
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
    ] {
        let destination = isolated.join(relative);
        fs::copy(root.join(relative), destination).unwrap();
    }
    vexil_release_governance_validator::validate_repository(&isolated)
        .expect("isolated public copy must validate");
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
        "missing-epic-2-control" => {
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
        "publishing-custodian" | "valid-single-steward-custodian" => {
            root.get_mut("decision").unwrap()["status"] =
                Value::String("single-steward-custodian".into());
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
                    "reviewEvidence":{"decisionId":"stewardship-continuity-2026-07-14","source":"https://github.com/vexil-lang/vexil/issues/64","reviewedBy":"github:furkanmamuk","reviewedAt":"2026-07-14"},
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
