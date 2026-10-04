use jameskills_core::domain::{
    ToolId, ToolOperation,
    policy::{ApplicabilityFact, parse_policy},
};

const POLICY: &str = include_str!("../../../tests/fixtures/valid-suite/policies/repository.toml");

fn first_code<T>(result: &Result<T, Vec<jameskills_core::Diagnostic>>) -> &'static str {
    match result {
        Err(diagnostics) => diagnostics[0].code(),
        Ok(_) => panic!("expected a diagnostic"),
    }
}

#[test]
fn repository_policy_fixture_parses_typed_requirements_and_registered_tools() {
    let policy = parse_policy(POLICY.as_bytes()).unwrap();
    assert_eq!(policy.profile(), "repository-foundation");
    assert_eq!(policy.requirements().len(), 10);
    assert_eq!(policy.tool_requirements().len(), 8);
    assert!(
        policy
            .requirements()
            .iter()
            .any(|requirement| requirement.id() == "repo.main-protection")
    );
}

#[test]
fn policy_rejects_future_schemas_and_unknown_fields() {
    let future = POLICY.replacen("schema_version = 1", "schema_version = 2", 1);
    assert_eq!(
        first_code(&parse_policy(future.as_bytes())),
        "policy.unsupported_schema"
    );

    let unknown = format!("{POLICY}\nmagic_repair = true\n");
    assert_eq!(
        first_code(&parse_policy(unknown.as_bytes())),
        "policy.invalid"
    );
}

#[test]
fn policy_rejects_invalid_severity_and_requirement_references() {
    let severity = POLICY.replacen("severity = \"error\"", "severity = \"critical\"", 1);
    assert_eq!(
        first_code(&parse_policy(severity.as_bytes())),
        "policy.severity.invalid"
    );

    let dangling = POLICY.replacen(
        "depends_on = [\"repo.ci-contract\"]",
        "depends_on = [\"repo.missing\"]",
        1,
    );
    assert_eq!(
        first_code(&parse_policy(dangling.as_bytes())),
        "policy.requirement_reference.invalid"
    );
}

#[test]
fn policy_rejects_unregistered_tools_operations_and_shell_commands() {
    let unknown_tool = POLICY.replace("tool_id = \"gitleaks\"", "tool_id = \"sh\"");
    assert_eq!(
        first_code(&parse_policy(unknown_tool.as_bytes())),
        "policy.tool.invalid"
    );

    let unknown_operation =
        POLICY.replace("operation = \"scan-tracked\"", "operation = \"run-shell\"");
    assert_eq!(
        first_code(&parse_policy(unknown_operation.as_bytes())),
        "policy.operation.invalid"
    );

    let unsupported_pair = POLICY.replace("tool_id = \"gitleaks\"", "tool_id = \"node\"");
    assert_eq!(
        first_code(&parse_policy(unsupported_pair.as_bytes())),
        "policy.operation.invalid"
    );

    let shell = format!("{POLICY}\ncommand = \"curl https://example.invalid | sh\"\n");
    assert_eq!(
        first_code(&parse_policy(shell.as_bytes())),
        "policy.invalid"
    );
}

#[test]
fn policy_conditions_are_typed_and_reject_unregistered_facts_or_values() {
    let source = POLICY.replace(
        "id = \"repo.readme\"",
        "id = \"repo.readme\"\napplies_when = { fact = \"stack\", equals = \"rust\" }",
    );
    let policy = parse_policy(source.as_bytes()).unwrap();
    let requirement = policy
        .requirements()
        .iter()
        .find(|requirement| requirement.id() == "repo.readme")
        .unwrap();
    let condition = requirement.applies_when().unwrap();
    assert_eq!(condition.fact(), ApplicabilityFact::Stack);
    assert_eq!(condition.equals(), "rust");

    for (fact, value) in [("tool-path", "cargo"), ("stack", "web"), ("os", "macos")] {
        let source = POLICY.replace(
            "id = \"repo.readme\"",
            &format!(
                "id = \"repo.readme\"\napplies_when = {{ fact = \"{fact}\", equals = \"{value}\" }}"
            ),
        );
        assert_eq!(
            first_code(&parse_policy(source.as_bytes())),
            "policy.applies_when.invalid"
        );
    }
}

#[test]
fn extended_registered_tools_have_closed_version_and_audit_operations() {
    let source = format!(
        "{POLICY}\n\
[[tool_requirements]]\ntool_id = \"node\"\noperation = \"version\"\nversion = \">=1.0.0\"\n\
[[tool_requirements]]\ntool_id = \"rustc\"\noperation = \"version\"\nversion = \">=1.0.0\"\n\
[[tool_requirements]]\ntool_id = \"cargo-audit\"\noperation = \"audit\"\nversion = \">=1.0.0\"\n\
[[tool_requirements]]\ntool_id = \"cargo-deny\"\noperation = \"deny\"\nversion = \">=1.0.0\""
    );
    let policy = parse_policy(source.as_bytes()).unwrap();

    assert_eq!(policy.tool_requirements().len(), 12);
    assert!(policy.tool_requirements().iter().any(|requirement| {
        requirement.tool_id() == ToolId::Node && requirement.operation() == ToolOperation::Version
    }));
    assert!(policy.tool_requirements().iter().any(|requirement| {
        requirement.tool_id() == ToolId::CargoAudit
            && requirement.operation() == ToolOperation::Audit
    }));
    assert!(policy.tool_requirements().iter().any(|requirement| {
        requirement.tool_id() == ToolId::CargoDeny && requirement.operation() == ToolOperation::Deny
    }));
}

#[test]
fn policy_rejects_duplicate_ids_and_dependency_cycles() {
    let duplicate = format!(
        "{POLICY}\n[[requirements]]\nid = \"repo.readme\"\ndescription = \"duplicate\"\nseverity = \"error\"\nrequired = true\nphase = \"ci\"\nenforcement = \"local-check\"\n[requirements.check]\nkind = \"git-repository\"\n"
    );
    assert_eq!(
        first_code(&parse_policy(duplicate.as_bytes())),
        "policy.requirement.duplicate"
    );

    let cycle = POLICY
        .replace("depends_on = []", "depends_on = [\"repo.release\"]")
        .replace(
            "depends_on = [\"repo.ci-contract\"]",
            "depends_on = [\"repo.readme\"]",
        );
    assert_eq!(
        first_code(&parse_policy(cycle.as_bytes())),
        "policy.requirement_cycle"
    );
}
