use jameskills_core::domain::policy::parse_policy;
use jameskills_core::domain::skill::{parse_frontmatter, parse_manifest, validate_skill_pair};

const MANIFEST: &str = include_str!("../../../tests/fixtures/valid-suite/jameskills.toml");
const SKILL: &str = include_str!("../../../tests/fixtures/valid-suite/SKILL.md");
const FOUNDATION_MANIFEST: &str =
    include_str!("../../../docs/examples/repository-foundation/jameskills.toml");
const FOUNDATION_SKILL: &str =
    include_str!("../../../docs/examples/repository-foundation/SKILL.md");
const FOUNDATION_POLICY: &str =
    include_str!("../../../docs/examples/repository-foundation/policies/repository.toml");

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
