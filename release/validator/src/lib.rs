use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Component, Path};
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
    validate_catalog_repository(root)?;
    validate_public_boundary(root)?;
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
    validate_catalog_documentation_parity(root, &catalog, &documentation)
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
    if text(
        object(required_value(go, "publication")?, "Go publication")?.get("status"),
        "Go status",
    )? != "blocked-missing-version-source"
        || text(
            object(required_value(go, "versionSource")?, "Go version source")?.get("path"),
            "Go version source path",
        )? != "packages/runtime-go/VERSION"
        || required_value(
            object(required_value(go, "versionSource")?, "Go version source")?,
            "observedDeclaration",
        )? != &Value::Null
    {
        return Err("Go runtime must retain the explicit missing VERSION blocker without an invented version".to_owned());
    }
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
    Ok(())
}

fn expected_catalog_source_roots(root: &Path) -> Result<BTreeSet<String>, String> {
    let workspace = parse_toml(&fs::read_to_string(root.join("Cargo.toml")).map_err(|error| {
        format!("read workspace manifest {}: {error}", root.join("Cargo.toml").display())
    })?)?;
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
                return Err(format!("workspace.{field} source root is missing Cargo.toml: {entry}"));
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
    let workspace = parse_toml(&fs::read_to_string(root.join("Cargo.toml")).map_err(|error| {
        format!("read workspace manifest {}: {error}", root.join("Cargo.toml").display())
    })?)?;
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
        Err(format!("catalog source root has no recognized source declaration: {source_root}"))
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
        if root.join(path).exists() || format != "required-file-absent" || !observed.is_null() {
            return Err(
                "Go runtime version source must remain the absent required VERSION file".to_owned(),
            );
        }
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
                return Err("non-publishable catalog units must mark changelog not-applicable".to_owned());
            }
            let path = changelog
                .get("path")
                .and_then(Value::as_str)
                .ok_or("present catalog changelog requires a path")?;
            require_catalog_path_within_source_root(
                source_root,
                path,
                "changelog",
            )?;
            if !root.join(path).is_file() {
                return Err(format!("catalog changelog is missing: {path}"));
            }
            if Path::new(path).file_name().and_then(|name| name.to_str()) != Some("CHANGELOG.md") {
                return Err("catalog changelog must name the unit CHANGELOG.md file".to_owned());
            }
        }
        "absent" if changelog.get("path") == Some(&Value::Null) => {
            if unit_kind == "non-publishable-package" {
                return Err("non-publishable catalog units must mark changelog not-applicable".to_owned());
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
                return Err("publishable catalog units must state changelog present or absent".to_owned());
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
                    return Err("catalog cargo-binary target must use a Cargo package manifest".to_owned());
                }
                cargo_binary_names(content.as_deref().unwrap(), &root.join(source_root))?.contains(name)
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
                if format != "required-file-absent" || classification != "blocked-version-source" {
                    return Err(
                        "catalog Go target must retain the explicit missing-version blocker"
                            .to_owned(),
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
            return Err(format!("catalog target name {name} does not match its source declaration"));
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
        _ => Err("catalog version source format is invalid for a present source".to_owned()),
    }
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

pub fn render_catalog_markdown(root: &Path, catalog: &Value) -> Result<String, String> {
    validate_catalog_schema(root, catalog)?;
    validate_catalog(root, catalog)?;
    let catalog = object(catalog, "release catalog")?;
    let mut markdown = String::from("# Release Unit Catalog\n\n> Generated view of [`release/catalog.json`](../../../../release/catalog.json). The JSON catalog is canonical; this Markdown is non-authoritative and parity-checked.\n\nThis is a source-led inventory, not a Release Manifest, publication assertion, provider-identity claim, release-order decision, or version-selection decision. Formal specifications and documentation govern semantics; pinned code and tests establish the executable baseline. Historical tags, changelog headings, and registry observations remain evidence in [Release History](./history.md).\n\n## Units\n\n| Unit | Source root | Targets | Status | Source version observation | Canonical tag policy |\n|---|---|---|---|---|---|\n");
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
    markdown.push_str("\n## Boundary and validation\n\n`candidate-unreleased` means the Python source unit is planned work, not a PyPI availability claim. `blocked-missing-version-source` means the Go module has no checked-in `VERSION` source; its `go.mod` module path is not a version. `non-publishable` roots are deliberately cataloged so they cannot be silently mistaken for releases.\n\nDependency-edge entries are provisional until the typed graph is established. A catalog entry or target category never establishes authorization, registry identity, publication eligibility, release ordering, or Release Set membership. Root project-wide `v<semver>` tags remain prohibited during recovery.\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nThe offline command validates source paths, direct declaration observations, unique unit identities, the Go blocker, and byte-exact generated-view parity. It performs no provider query or release effect.\n");
    Ok(markdown)
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
    let mut markdown = String::from("# Stewardship Continuity Tabletop Exercises\n\n> Generated public view of [`release/exercises/tabletop-stewardship-continuity-2026-07-14.json`](../../../../release/exercises/tabletop-stewardship-continuity-2026-07-14.json). The JSON record is canonical; this page is parity-checked and non-authoritative.\n\nThese are tabletop-only, non-mutating exercises, not Release Runs. The unresolved continuity gate still blocks Manifest approval and privileged publication.\n\n## Record\n\n");
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
    if !publication_block.contains("stewardship assignments")
        || !publication_block.contains("external controls")
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
    validate_recovery_contact(required_value(continuity, "recoveryContact")?)?;
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
    if reason.is_empty()
        || (decision_status == "unresolved-continuity" && !reason.contains("continuity"))
    {
        return Err("unresolved continuity must visibly block privileged readiness".to_owned());
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

fn validate_recovery_contact(value: &Value) -> Result<(), String> {
    let contact = object(value, "recovery contact")?;
    require_exact_keys(
        contact,
        &["status", "publicRoute", "outcome"],
        "recovery contact",
    )?;
    require_string(contact, "status", "unresolved-no-distinct-custodian")?;
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
        required_value(object(record, "responsibility inventory")?, "manifestComparison")?,
        "manifest comparison",
    )?;
    let actual: BTreeSet<_> = strings(
        comparison.get("catalogedSourceUnits"),
        "cataloged source units",
    )?
    .into_iter()
    .collect();
    let declared = array(comparison.get("catalogedSourceUnits"), "cataloged source units")?;
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
    markdown.push_str(&format!("\n## Procedure boundary\n\nEach row is an owned fail-closed procedure with exactly one responsibility ID. It requires the current Manifest and typed catalog edges rather than `.vexilbot.toml` or historical behavior. The runbook does not make any procedure operationally ready: external controls, authorization and candidate evidence, and the unresolved continuity gate remain explicit blockers. A green test or workflow cannot complete a blocked operation.\n\nFor compatibility and policy decisions, follow {governance_reference}; this runbook neither changes nor bypasses its BDFL, RFC, or breaking-change commitments.\n\n## Validation\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nThe command validates this public contract offline and fails closed. It does not change a workflow, environment, credential, tag, registry, provider, or release.\n"));
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
    markdown.push_str("\n## Boundaries and continuity\n\nAdvisory automation may validate, triage, label, advise on dependencies, and rehearse only. It has no release, package, deployment, protected-branch, environment, credential, version-selection, Release Set scope-selection, or risk-acceptance authority. A Repository Administrator may only stop, revoke, contain, and activate succession in an emergency; it may not move tags, overwrite artifacts, rewrite evidence, accept security risk, approve publication, or declare completion.\n\nRoles may be combined, but permissions never union implicitly: each action requires an explicit asserted role. Role assignments are deliberately absent from this contract and are recorded separately. Contract validation does not prove live workflow or provider enforcement. Publication remains blocked until stewardship assignments and continuity are resolved and external controls are corrected and verified.\n\n## Offline validation\n\nFrom the repository root, run the repository-local validator without network access:\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nIt validates schema syntax, the canonical record, semantic authority invariants, documentation parity, and the public/private boundary.\n\n## Compatibility governance\n\nThis contract does not replace the BDFL, RFC, public-review, or breaking-change rules in [the governance policy](../../../../GOVERNANCE.md). Language, wire-format, compiler, generator, runtime, corpus/conformance, and public API changes continue through that existing route.\n");
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
    markdown.push_str("\nEach row is an independently auditable role assertion. Combining these assignments does not union permissions: every action remains constrained by the explicit role assertion in the [Stewardship Authority Model](./stewardship.md).\n\n## Unresolved continuity gate\n\n");
    let custodian = continuity.get("custodian").unwrap_or(&Value::Null);
    if custodian.is_null() {
        markdown.push_str("No distinct non-publishing recovery custodian has been approved. The unavailable-owner route is containment or documented succession only: it may stop, revoke, contain, or activate succession, but cannot create release authority, move tags, overwrite artifacts, rewrite evidence, accept risk, or declare completion.\n\n");
    }
    let recovery = object(
        required_value(continuity, "recoveryContact")?,
        "recovery contact",
    )?;
    markdown.push_str(&format!(
        "## Recovery contact route\n\nNo distinct custodian is currently approved. Record containment and request a reviewed successor through [the public decision route]({}); this route grants no recovery, Manifest, or publication authority.\n\n",
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
    markdown.push_str("If a second qualified Release Steward is recorded, detached approval by an identity distinct from the Manifest approver becomes mandatory; provider self-review settings alone are not evidence. A future [release-continuity-runbook](#future-runbook) is reserved for the unavailable-owner and succession procedure.\n\n## Future runbook\n\nThe stable identifier `release-continuity-runbook` is reserved for the public unavailable-owner and succession runbook. It does not create a custodian or authorize a release.\n\n## Validation\n\nFrom a clean public checkout, run:\n\n```sh\ncargo run --manifest-path release/validator/Cargo.toml --offline -- --root .\n```\n\nThe validator checks the authority contract, public role assignments, every currently maintained Package Steward root, documentation parity, and the unresolved fail-closed publication gate. It does not change provider settings or create a release.\n\nThis decision preserves the BDFL, RFC, and breaking-change rules in [GOVERNANCE.md](../../../../GOVERNANCE.md).\n");
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
    use super::{go_module_name, toml_string_in_section};

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
}
