use jameskills_core::domain::{
    GuidanceAction, OfficialGuidanceSource, PortablePath, ToolId, ToolOperation,
    skill::validate_bundle,
};
use std::collections::BTreeMap;

const MANIFEST: &str = include_str!("../../../docs/examples/repository-foundation/jameskills.toml");
const SKILL: &str = include_str!("../../../docs/examples/repository-foundation/SKILL.md");
const POLICY: &str =
    include_str!("../../../docs/examples/repository-foundation/policies/repository.toml");
const GUIDANCE: &str =
    include_str!("../../../docs/examples/repository-foundation/guidance/repository.toml");

fn bundle_files(guidance: &str) -> BTreeMap<PortablePath, Vec<u8>> {
    [
        ("jameskills.toml", MANIFEST),
        ("SKILL.md", SKILL),
        ("policies/repository.toml", POLICY),
        ("guidance/repository.toml", guidance),
    ]
    .into_iter()
    .map(|(path, source)| {
        (
            PortablePath::new(path.to_owned()).unwrap(),
            source.as_bytes().to_vec(),
        )
    })
    .collect()
}

#[test]
fn guidance_step_without_verification_requirements_is_rejected() {
    let guidance = GUIDANCE.replacen(
        "verification_requirement_ids = [\"git-ready\"]",
        "verification_requirement_ids = []",
        1,
    );
    assert_ne!(guidance, GUIDANCE, "fixture mutation must target a step");

    assert!(validate_bundle(&bundle_files(&guidance)).is_err());
}

#[test]
fn guidance_step_cannot_claim_verification_outside_its_plan() {
    let guidance = GUIDANCE.replacen(
        "verification_requirement_ids = [\"readme-structure\"]",
        "verification_requirement_ids = [\"git-ready\"]",
        1,
    );
    assert_ne!(guidance, GUIDANCE, "fixture mutation must target a step");

    assert!(validate_bundle(&bundle_files(&guidance)).is_err());
}

#[test]
fn guidance_fact_values_are_checked_against_the_registered_fact() {
    let guidance = GUIDANCE.replacen(
        "verification_requirement_ids = [\"git-ready\"]",
        "verification_requirement_ids = [\"git-ready\"]\napplies_when = { fact = \"os\", equals = \"plan9\" }",
        1,
    );
    assert_ne!(guidance, GUIDANCE, "fixture mutation must target a step");

    assert!(validate_bundle(&bundle_files(&guidance)).is_err());
}

#[test]
fn validated_bundle_retains_parsed_policies_and_guidance_steps() {
    let bundle = validate_bundle(&bundle_files(GUIDANCE)).unwrap();

    assert_eq!(bundle.policies().len(), 1);
    let plan = bundle
        .guidance_plans()
        .iter()
        .find(|plan| plan.id() == "release-setup")
        .expect("validated bundle must retain the compiled release guide");
    assert_eq!(plan.requirement_ids(), ["release-ready"]);
    assert_eq!(plan.steps().len(), 1);
    let step = &plan.steps()[0];
    assert_eq!(step.id(), "release");
    assert_eq!(
        step.prompt_es(),
        "Prepara versión SemVer, changelog y checksums de artefactos tras pasar gates. Publicar requiere la autorización correspondiente."
    );
    assert_eq!(step.verification_requirement_ids(), ["release-ready"]);
    assert!(matches!(step.action(), GuidanceAction::ManualInstruction));

    let github_plan = bundle
        .guidance_plans()
        .iter()
        .find(|plan| plan.id() == "github-protection")
        .expect("validated bundle must retain the registered host guide");
    assert!(matches!(
        github_plan.steps()[0].action(),
        GuidanceAction::OpenOfficialUrl {
            source: OfficialGuidanceSource::GithubRulesets
        }
    ));
}

#[test]
fn compiled_command_action_keeps_only_a_registered_tool_operation() {
    let guidance = GUIDANCE.replacen(
        "kind = \"manual-instruction\"",
        "kind = \"copy-approved-command\"\ntool_id = \"git\"\noperation = \"repository-root\"",
        1,
    );
    assert_ne!(guidance, GUIDANCE, "fixture mutation must target an action");
    let bundle = validate_bundle(&bundle_files(&guidance)).unwrap();
    let verify_git = bundle
        .guidance_plans()
        .iter()
        .find(|plan| plan.id() == "readme-setup")
        .unwrap()
        .steps()
        .iter()
        .find(|step| step.id() == "review-readme")
        .unwrap();
    assert!(matches!(
        verify_git.action(),
        GuidanceAction::CopyApprovedCommand {
            tool_id: ToolId::Git,
            operation: ToolOperation::RepositoryRoot,
        }
    ));
}
