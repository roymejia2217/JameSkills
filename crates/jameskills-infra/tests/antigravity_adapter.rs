use jameskills_core::{
    AppError,
    domain::{PortablePath, Scope, validate_bundle},
    ports::{
        agent::{AgentAvailability, AgentPort, DetectionContext},
        process::{ApprovedExecutable, ExecutableFingerprint},
    },
};
use jameskills_infra::{
    agents::{AgentRegistry, antigravity::AntigravityAdapter},
    platform::user_home_directory,
};
use std::{collections::BTreeMap, path::PathBuf};

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

fn files() -> BTreeMap<PortablePath, Vec<u8>> {
    [
        (
            "jameskills.toml",
            "schema_version = 1\nid = \"f9c0199f-c4ce-4b04-85dd-ae12a7db292b\"\nslug = \"portable-demo\"\ndisplay_name = \"Portable Demo\"\nversion = \"1.0.0\"\ndescription = \"Test fixture description\"\nlicense = \"Apache-2.0\"\nminimum_app_version = \"0.1.0\"\npolicy_files = []\nguidance_files = []\n",
        ),
        (
            "SKILL.md",
            "---\nname: portable-demo\ndescription: Test fixture description\n---\n\nInstructions are copied as data.\n",
        ),
        ("assets/example.txt", "Preserve exact bytes.\n"),
    ]
    .into_iter()
    .map(|(path, contents)| {
        (
            PortablePath::new(path.to_owned()).unwrap(),
            contents.as_bytes().to_vec(),
        )
    })
    .collect()
}

fn adapter(home: PathBuf) -> AntigravityAdapter {
    let registry = AgentRegistry::built_in().unwrap();
    AntigravityAdapter::new(
        registry
            .get(jameskills_core::domain::agent::AgentId::Antigravity)
            .unwrap()
            .clone(),
        home,
    )
    .unwrap()
}

#[test]
fn plugin_artifact_contains_only_closed_manifest_and_validated_skill_data() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let home = std::env::temp_dir().join("Antigravity Home With Spaces");
    let adapter = adapter(home.clone());

    let artifact = adapter
        .plan_plugin_artifact(&validated, files.clone(), Scope::User)
        .unwrap();

    assert_eq!(artifact.plugin_name(), "jameskills-portable-demo");
    assert_eq!(
        artifact.target(),
        home.join(".gemini")
            .join("antigravity-cli")
            .join("plugins")
            .join("jameskills-portable-demo")
            .as_path()
    );
    let manifest_path = PortablePath::new("plugin.json".to_owned()).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&artifact.files()[&manifest_path]).unwrap();
    assert_eq!(manifest["name"], "jameskills-portable-demo");
    assert_eq!(manifest["description"], "Test fixture description");
    assert!(manifest.get("$schema").is_none());
    assert_eq!(artifact.files().len(), files.len() + 1);
    for (path, bytes) in files {
        let output_path = PortablePath::new(format!("skills/portable-demo/{path}")).unwrap();
        assert_eq!(artifact.files().get(&output_path), Some(&bytes));
    }
    for forbidden in ["hooks.json", "mcp_config.json", "agents/", "rules/"] {
        assert!(
            !artifact
                .files()
                .keys()
                .any(|path| path.as_str().starts_with(forbidden))
        );
    }
}

#[test]
fn antigravity_project_scope_and_changed_bundle_bytes_are_rejected() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let adapter = adapter(std::env::temp_dir().join("Antigravity Home"));
    assert!(matches!(
        adapter.plan_plugin_artifact(&validated, files.clone(), Scope::Project),
        Err(AppError::CapabilityUnavailable { .. })
    ));

    let mut changed = files;
    changed.insert(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"---\nname: portable-demo\ndescription: changed\n---\n".to_vec(),
    );
    assert!(matches!(
        adapter.plan_plugin_artifact(&validated, changed, Scope::User),
        Err(AppError::Conflict { .. })
    ));
}

#[test]
fn antigravity_detection_never_claims_verified_without_a_reviewed_version_probe() {
    let adapter = adapter(std::env::temp_dir().join("Antigravity Home"));
    let context = DetectionContext::new(Scope::User, None).unwrap();

    let detection = block_on(adapter.detect(context)).unwrap();

    assert!(matches!(
        detection.availability(),
        AgentAvailability::Missing | AgentAvailability::Candidate
    ));
    assert!(detection.version().is_none());
    assert_eq!(
        detection.id(),
        jameskills_core::domain::agent::AgentId::Antigravity
    );
}

#[test]
fn approved_antigravity_binary_remains_blocked_without_an_official_version_contract() {
    let adapter = adapter(std::env::temp_dir().join("Antigravity Home"));
    let executable =
        ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap();
    let fingerprint = ExecutableFingerprint::from_sha256([0x51; 32]);
    let approved = jameskills_core::ports::agent::ApprovedAgentExecutable::after_explicit_fingerprint_confirmation(
        executable,
        fingerprint,
        &fingerprint,
    )
    .unwrap();
    let context = DetectionContext::new(Scope::User, None)
        .unwrap()
        .with_approved_executable(approved);

    let detection = block_on(adapter.detect(context)).unwrap();

    assert_eq!(detection.availability(), AgentAvailability::Blocked);
    assert!(detection.version().is_none());
}

#[test]
fn official_antigravity_profile_root_is_user_cli_path() {
    let home = user_home_directory().unwrap();
    let adapter = adapter(home.clone());
    let context = DetectionContext::new(Scope::User, None).unwrap();
    let detection = block_on(adapter.detect(context)).unwrap();

    assert_eq!(
        detection.profile_root().unwrap(),
        home.join(".gemini")
            .join("antigravity-cli")
            .join("plugins")
            .as_path()
    );
}
