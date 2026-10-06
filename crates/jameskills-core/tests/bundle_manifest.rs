use jameskills_core::PortablePath;
use jameskills_core::domain::policy::parse_policy;
use jameskills_core::domain::skill::{
    parse_frontmatter, parse_manifest, validate_bundle, validate_skill_pair,
};
use std::collections::BTreeMap;

const MANIFEST: &str = include_str!("../../../tests/fixtures/valid-suite/jameskills.toml");
const SKILL: &str = include_str!("../../../tests/fixtures/valid-suite/SKILL.md");
const FOUNDATION_MANIFEST: &str =
    include_str!("../../../docs/examples/repository-foundation/jameskills.toml");
const FOUNDATION_SKILL: &str =
    include_str!("../../../docs/examples/repository-foundation/SKILL.md");
const FOUNDATION_POLICY: &str =
    include_str!("../../../docs/examples/repository-foundation/policies/repository.toml");
const FOUNDATION_GUIDANCE: &str =
    include_str!("../../../docs/examples/repository-foundation/guidance/repository.toml");
const FOUNDATION_STANDARDS: &str =
    include_str!("../../../docs/examples/repository-foundation/references/standards.md");
const FOUNDATION_ENVIRONMENT: &str =
    include_str!("../../../docs/examples/repository-foundation/references/environment.md");
const FOUNDATION_README_TEMPLATE: &str =
    include_str!("../../../docs/examples/repository-foundation/templates/README.md");
const FOUNDATION_GITIGNORE_TEMPLATE: &str =
    include_str!("../../../docs/examples/repository-foundation/templates/gitignore.txt");

fn official_bundle_files() -> BTreeMap<PortablePath, Vec<u8>> {
    [
        ("SKILL.md", FOUNDATION_SKILL),
        ("jameskills.toml", FOUNDATION_MANIFEST),
        ("policies/repository.toml", FOUNDATION_POLICY),
        ("guidance/repository.toml", FOUNDATION_GUIDANCE),
    ]
    .into_iter()
    .map(|(path, content)| {
        (
            PortablePath::new(path.to_owned()).unwrap(),
            content.as_bytes().to_vec(),
        )
    })
    .collect()
}

fn first_code<T>(result: &Result<T, Vec<jameskills_core::Diagnostic>>) -> &'static str {
    match result {
        Err(diagnostics) => diagnostics[0].code(),
        Ok(_) => panic!("expected a diagnostic"),
    }
}

fn first_diagnostic<T>(
    result: &Result<T, Vec<jameskills_core::Diagnostic>>,
) -> &jameskills_core::Diagnostic {
    match result {
        Err(diagnostics) => &diagnostics[0],
        Ok(_) => panic!("expected a diagnostic"),
    }
}

#[test]
fn standard_fixture_parses_and_preserves_skill_markdown_and_metadata() {
    let manifest = parse_manifest(MANIFEST).unwrap();
    let frontmatter = parse_frontmatter(SKILL.as_bytes()).unwrap();
    validate_skill_pair(&manifest, &frontmatter).unwrap();

    assert_eq!(manifest.slug(), "repository-foundation");
    assert_eq!(manifest.version().to_string(), "1.0.0");
    assert_eq!(manifest.capabilities().len(), 1);
    assert_eq!(manifest.capabilities()[0].id(), "git");
    assert!(manifest.capabilities()[0].required());
    assert_eq!(frontmatter.name(), manifest.slug());
    assert_eq!(frontmatter.metadata()["jameskills-version"], "1.0.0");
    assert_eq!(frontmatter.source(), SKILL);
    assert!(frontmatter.body().starts_with("# Repositorio seguro"));
}

#[test]
fn validates_official_bundle_and_returns_canonical_hash() {
    let validated = match validate_bundle(&official_bundle_files()) {
        Ok(validated) => validated,
        Err(diagnostics) => panic!("official bundle rejected: {diagnostics:?}"),
    };

    assert_eq!(validated.manifest().slug(), "repository-foundation");
    assert_eq!(validated.manifest().version().to_string(), "1.0.0");
    assert_eq!(validated.file_count(), 4);
    assert_eq!(validated.content_hash().as_str().len(), 64);
}

#[test]
fn bundle_validation_rejects_a_missing_manifest_resource() {
    let mut files = official_bundle_files();
    files.remove(&PortablePath::new("policies/repository.toml".to_owned()).unwrap());

    let diagnostics = match validate_bundle(&files) {
        Ok(_) => panic!("bundle with a missing manifest resource must fail"),
        Err(diagnostics) => diagnostics,
    };
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "bundle.resource.missing")
    );
}

#[test]
fn bundle_validation_rejects_an_unregistered_guidance_command_field() {
    let mut files = official_bundle_files();
    let guidance = FOUNDATION_GUIDANCE.replacen(
        "kind = \"manual-instruction\"",
        "kind = \"manual-instruction\"\ncommand = \"echo untrusted\"",
        1,
    );
    files.insert(
        PortablePath::new("guidance/repository.toml".to_owned()).unwrap(),
        guidance.into_bytes(),
    );

    let diagnostics = match validate_bundle(&files) {
        Ok(_) => panic!("unregistered guidance command fields must fail"),
        Err(diagnostics) => diagnostics,
    };
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "bundle.guidance.action.invalid")
    );
}

#[test]
fn bundle_validation_combines_guidance_plans_from_multiple_manifest_files() {
    let mut files = official_bundle_files();
    let manifest = FOUNDATION_MANIFEST.replace(
        "guidance_files = [\"guidance/repository.toml\"]",
        "guidance_files = [\"guidance/part-a.toml\", \"guidance/part-b.toml\"]",
    );
    files.insert(
        PortablePath::new("jameskills.toml".to_owned()).unwrap(),
        manifest.into_bytes(),
    );
    files.remove(&PortablePath::new("guidance/repository.toml".to_owned()).unwrap());
    let guidance: toml::Value = toml::from_str(FOUNDATION_GUIDANCE).unwrap();
    let plans = guidance["plans"].as_array().unwrap();
    let midpoint = plans.len() / 2;

    for (path, subset) in [
        ("guidance/part-a.toml", &plans[..midpoint]),
        ("guidance/part-b.toml", &plans[midpoint..]),
    ] {
        let mut table = toml::map::Map::new();
        table.insert("schema_version".to_owned(), toml::Value::Integer(1));
        table.insert("plans".to_owned(), toml::Value::Array(subset.to_vec()));
        let source = toml::to_string(&toml::Value::Table(table)).unwrap();
        files.insert(
            PortablePath::new(path.to_owned()).unwrap(),
            source.into_bytes(),
        );
    }

    let validated = match validate_bundle(&files) {
        Ok(validated) => validated,
        Err(diagnostics) => panic!("split guidance bundle rejected: {diagnostics:?}"),
    };
    assert_eq!(validated.manifest().slug(), "repository-foundation");
}

#[test]
fn official_repository_example_passes_manifest_skill_and_policy_parsers() {
    let manifest = parse_manifest(FOUNDATION_MANIFEST).unwrap();
    let skill = parse_frontmatter(FOUNDATION_SKILL.as_bytes()).unwrap();
    validate_skill_pair(&manifest, &skill).unwrap();
    let policy = parse_policy(FOUNDATION_POLICY.as_bytes()).unwrap();

    assert_eq!(
        manifest.id().as_uuid().to_string(),
        "f9c0199f-c4ce-4b04-85dd-ae12a7db292b"
    );
    assert_eq!(policy.profile(), "repository-foundation");
    assert!(!policy.requirements().is_empty());
}

#[test]
fn portable_repository_example_is_available_at_the_runtime_fixture_path() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/repository-foundation");
    let manifest = std::fs::read_to_string(root.join("jameskills.toml"))
        .expect("canonical runtime example manifest must exist");
    let skill =
        std::fs::read(root.join("SKILL.md")).expect("canonical runtime example skill must exist");
    let policy = std::fs::read(root.join("policies/repository.toml"))
        .expect("canonical runtime example policy must exist");

    let parsed_manifest = parse_manifest(&manifest).unwrap();
    let parsed_skill = parse_frontmatter(&skill).unwrap();
    validate_skill_pair(&parsed_manifest, &parsed_skill).unwrap();
    assert_eq!(
        parse_policy(&policy).unwrap().profile(),
        "repository-foundation"
    );
}

#[test]
fn runtime_guidance_references_known_requirements_and_registered_actions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/repository-foundation");
    let policy_bytes = std::fs::read(root.join("policies/repository.toml")).unwrap();
    let policy = parse_policy(&policy_bytes).unwrap();
    let guidance_source = std::fs::read_to_string(root.join("guidance/repository.toml"))
        .expect("canonical runtime guidance must exist");
    let guidance: toml::Value = toml::from_str(&guidance_source).unwrap();
    let source_guidance: toml::Value = toml::from_str(FOUNDATION_GUIDANCE).unwrap();
    assert_eq!(guidance, source_guidance);
    assert_eq!(guidance["schema_version"].as_integer(), Some(1));

    let known_requirements: std::collections::BTreeSet<&str> = policy
        .requirements()
        .iter()
        .map(|requirement| requirement.id())
        .collect();
    let plans = guidance["plans"].as_array().unwrap();
    let plan_ids: std::collections::BTreeSet<&str> = plans
        .iter()
        .map(|plan| plan["id"].as_str().unwrap())
        .collect();
    assert_eq!(plan_ids.len(), plans.len());
    for requirement in policy.requirements() {
        if let Some(guidance_id) = requirement.guidance_id() {
            assert!(plan_ids.contains(guidance_id));
        }
    }

    for plan in plans {
        let requirements = plan["requirement_ids"].as_array().unwrap();
        assert!(!requirements.is_empty());
        for requirement in requirements {
            assert!(known_requirements.contains(requirement.as_str().unwrap()));
        }
        let steps = plan["steps"].as_array().unwrap();
        let step_ids: std::collections::BTreeSet<&str> = steps
            .iter()
            .map(|step| step["id"].as_str().unwrap())
            .collect();
        assert_eq!(step_ids.len(), steps.len());
        let mut remaining: std::collections::BTreeSet<String> = step_ids
            .iter()
            .map(|step_id| (*step_id).to_owned())
            .collect();
        while !remaining.is_empty() {
            let ready: Vec<String> = steps
                .iter()
                .filter(|step| remaining.contains(step["id"].as_str().unwrap()))
                .filter(|step| {
                    step["requires"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|dependency| !remaining.contains(dependency.as_str().unwrap()))
                })
                .map(|step| step["id"].as_str().unwrap().to_owned())
                .collect();
            assert!(!ready.is_empty(), "guidance plan contains a step cycle");
            for step_id in ready {
                remaining.remove(&step_id);
            }
        }
        for step in steps {
            for dependency in step["requires"].as_array().unwrap() {
                assert!(step_ids.contains(dependency.as_str().unwrap()));
            }
            for requirement in step["verification_requirement_ids"].as_array().unwrap() {
                assert!(known_requirements.contains(requirement.as_str().unwrap()));
            }
            let action = step["action"].as_table().unwrap();
            match action.get("kind").and_then(toml::Value::as_str) {
                Some("manual-instruction" | "recheck") => assert_eq!(action.len(), 1),
                Some("open-official-url") => {
                    assert_eq!(action.len(), 2);
                    assert!(matches!(
                        action.get("source_id").and_then(toml::Value::as_str),
                        Some(
                            "git-install"
                                | "gitleaks"
                                | "conventional-commits"
                                | "github-cli"
                                | "github-rulesets"
                        )
                    ));
                }
                other => panic!("unsupported guidance action {other:?}"),
            }
        }
    }
}

#[test]
fn runtime_reference_material_matches_documented_sources() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/repository-foundation");
    for (path, source) in [
        ("references/standards.md", FOUNDATION_STANDARDS),
        ("references/environment.md", FOUNDATION_ENVIRONMENT),
        ("templates/README.md", FOUNDATION_README_TEMPLATE),
    ] {
        let runtime = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|_| panic!("canonical resource {path} must exist"));
        assert_eq!(runtime.replace("\r\n", "\n"), source.replace("\r\n", "\n"));
    }
}

#[test]
fn runtime_skill_links_to_the_user_safe_gitignore_template() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/repository-foundation");
    let skill = std::fs::read_to_string(root.join("SKILL.md")).unwrap();
    assert!(skill.contains("[gitignore](templates/.gitignore)"));
    let runtime = std::fs::read_to_string(root.join("templates/.gitignore"))
        .expect("the linked gitignore template must exist");
    assert_eq!(
        runtime.replace("\r\n", "\n"),
        FOUNDATION_GITIGNORE_TEMPLATE.replace("\r\n", "\n")
    );
}

#[test]
fn runtime_ci_template_is_pinned_read_only_and_svg_asset_has_no_active_content() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/repository-foundation");
    let ci_template = std::fs::read_to_string(root.join("templates/ci-rust.yml"))
        .expect("canonical CI template must exist");
    assert!(ci_template.contains("on:\n  push:\n  pull_request:"));
    assert!(ci_template.contains("permissions:\n  contents: read"));
    assert!(ci_template.contains("actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803"));
    assert!(
        ci_template
            .contains("dtolnay/rust-toolchain@6bed0761d98439e5a578e2877258200ad565ba87 # v1.98.1")
    );
    assert!(ci_template.contains("toolchain: 1.95.0"));
    assert!(!ci_template.contains("TEMPLATE DE PLAN"));
    assert!(!ci_template.contains("run: exit 1"));

    let svg = std::fs::read_to_string(root.join("assets/optional-brand.svg"))
        .expect("canonical SVG asset must exist");
    let lower = svg.to_ascii_lowercase();
    for forbidden in [
        "<script",
        "foreignobject",
        "href=",
        "onload=",
        "onclick=",
        "onerror=",
        "<image",
    ] {
        assert!(!lower.contains(forbidden), "SVG contains {forbidden}");
    }
    assert!(lower.contains("<svg "));
}

#[test]
fn manifest_rejects_unknown_schema_with_location() {
    let source = MANIFEST.replacen("schema_version = 1", "schema_version = 2", 1);
    let result = parse_manifest(&source);
    assert_eq!(first_code(&result), "manifest.unsupported_schema");
    assert!(first_diagnostic(&result).line().is_some());
}

#[test]
fn manifest_rejects_invalid_uuid_and_semver_with_locations() {
    let invalid_id = MANIFEST.replace("f9c0199f-c4ce-4b04-85dd-ae12a7db292b", "not-a-uuid");
    let id_result = parse_manifest(&invalid_id);
    assert_eq!(first_code(&id_result), "manifest.id.invalid");
    assert!(first_diagnostic(&id_result).line().is_some());

    let invalid_version = MANIFEST.replace("version = \"1.0.0\"", "version = \"release-one\"");
    let version_result = parse_manifest(&invalid_version);
    assert_eq!(first_code(&version_result), "manifest.version.invalid");
    assert!(first_diagnostic(&version_result).line().is_some());
}

#[test]
fn manifest_rejects_unknown_fields_but_accepts_string_extensions() {
    let unknown = format!("{MANIFEST}\nunknown_option = true\n");
    let unknown_result = parse_manifest(&unknown);
    assert_eq!(first_code(&unknown_result), "manifest.invalid");
    assert!(first_diagnostic(&unknown_result).line().is_some());

    let extension = format!("{MANIFEST}\n[extensions]\n\"com.example.note\" = \"reviewed\"\n");
    assert_eq!(
        parse_manifest(&extension).unwrap().extensions()["com.example.note"],
        "reviewed"
    );
}

#[test]
fn manifest_validates_slug_limits_paths_and_unique_capabilities() {
    let invalid_slug = MANIFEST.replace("slug = \"repository-foundation\"", "slug = \"../escape\"");
    assert_eq!(
        first_code(&parse_manifest(&invalid_slug)),
        "manifest.slug.invalid"
    );

    let duplicate_capability =
        format!("{MANIFEST}\n[[capabilities]]\nid = \"git\"\nrequired = false\n");
    assert_eq!(
        first_code(&parse_manifest(&duplicate_capability)),
        "manifest.capability.duplicate"
    );

    let unsafe_resource = MANIFEST.replace("policies/repository.toml", "../outside.toml");
    assert_eq!(
        first_code(&parse_manifest(&unsafe_resource)),
        "manifest.path.invalid"
    );
}

#[test]
fn frontmatter_requires_standard_name_description_and_nonempty_body() {
    assert_eq!(
        first_code(&parse_frontmatter(b"# no frontmatter\n")),
        "frontmatter.missing"
    );
    assert_eq!(
        first_code(&parse_frontmatter(b"---\nname: skill\n---\n")),
        "frontmatter.invalid"
    );
}

#[test]
fn frontmatter_rejects_aliases_anchors_tags_and_multiple_documents() {
    for source in [
        "---\nname: &skill value\ndescription: Description\n---\nBody\n",
        "---\nname: *skill\ndescription: Description\n---\nBody\n",
        "---\nname: &skill skill\ndescription: *skill\n---\nBody\n",
        "---\nname: !custom skill\ndescription: Description\n---\nBody\n",
    ] {
        assert!(
            parse_frontmatter(source.as_bytes()).is_err(),
            "accepted {source:?}"
        );
    }
}

#[test]
fn frontmatter_rejects_merge_keys_and_oversized_header() {
    let merge_key =
        "---\nname: skill\ndescription: Description\nmetadata:\n  <<: \"value\"\n---\nBody\n";
    assert_eq!(
        first_code(&parse_frontmatter(merge_key.as_bytes())),
        "frontmatter.merge_key.unsupported"
    );

    let oversized_header = format!(
        "---\nname: skill\ndescription: Description\nmetadata:\n  note: \"{}\"\n---\nBody\n",
        "x".repeat(17 * 1024)
    );
    assert_eq!(
        first_code(&parse_frontmatter(oversized_header.as_bytes())),
        "frontmatter.header_too_large"
    );
}

#[test]
fn frontmatter_rejects_duplicate_and_unknown_fields_with_sanitized_diagnostics() {
    let duplicate = "---\nname: skill\nname: other\ndescription: Description\n---\nBody\n";
    let duplicate_result = parse_frontmatter(duplicate.as_bytes());
    assert_eq!(first_code(&duplicate_result), "frontmatter.duplicate_key");

    let unknown = "---\nname: skill\ndescription: Description\nexecute: true\n---\nBody\n";
    let unknown_result = parse_frontmatter(unknown.as_bytes());
    assert_eq!(first_code(&unknown_result), "frontmatter.unknown_field");
    assert!(
        !first_diagnostic(&unknown_result)
            .message()
            .contains("execute")
    );
}

#[test]
fn frontmatter_rejects_deep_structure_in_string_metadata() {
    let source = format!(
        "---\nname: skill\ndescription: Description\nmetadata:\n  nested: {}x{}\n---\nBody\n",
        "[".repeat(20),
        "]".repeat(20)
    );
    assert!(parse_frontmatter(source.as_bytes()).is_err());
}

#[test]
fn markdown_body_delimiters_are_preserved_after_frontmatter_closes() {
    let source = "---\nname: skill\ndescription: Description\n---\nBody\n---\nmarkdown separator\n";
    let parsed = parse_frontmatter(source.as_bytes()).unwrap();
    assert_eq!(parsed.body(), "Body\n---\nmarkdown separator\n");
}

#[test]
fn frontmatter_metadata_must_be_a_string_map_and_description_must_match_manifest() {
    let non_string_metadata = "---\nname: repository-foundation\ndescription: Description\nmetadata:\n  release: 3\n---\nBody\n";
    assert_eq!(
        first_code(&parse_frontmatter(non_string_metadata.as_bytes())),
        "frontmatter.invalid"
    );

    let manifest = parse_manifest(MANIFEST).unwrap();
    let other_description = SKILL.replace(
        "Aplica y verifica estándares Git, seguridad, CI y releases.",
        "Otra descripción.",
    );
    let other_frontmatter = parse_frontmatter(other_description.as_bytes()).unwrap();
    assert_eq!(
        first_code(&validate_skill_pair(&manifest, &other_frontmatter)),
        "frontmatter.description_mismatch"
    );
}

#[test]
fn parsers_enforce_document_size_limits() {
    let oversized_manifest = format!("{MANIFEST}\n#{}", "x".repeat(70 * 1024));
    assert_eq!(
        first_code(&parse_manifest(&oversized_manifest)),
        "manifest.too_large"
    );

    let oversized_skill = format!(
        "---\nname: skill\ndescription: Description\n---\n{}",
        "x".repeat(256 * 1024)
    );
    assert_eq!(
        first_code(&parse_frontmatter(oversized_skill.as_bytes())),
        "frontmatter.too_large"
    );
}
