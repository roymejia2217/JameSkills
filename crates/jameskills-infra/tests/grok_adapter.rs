use jameskills_core::{
    AppError,
    domain::{CapabilitySupport, PortablePath, Scope, validate_bundle},
    ports::{
        agent::{AgentAvailability, AgentPort, DetectionContext},
        process::ApprovedRoot,
    },
};
use jameskills_infra::agents::{AgentRegistry, grok::GrokAdapter};
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
            "schema_version = 1\nid = \"f9c0199f-c4ce-4b04-85dd-ae12a7db292b\"\nslug = \"portable-demo\"\ndisplay_name = \"Portable Demo\"\nversion = \"1.0.0\"\ndescription = \"Test fixture\"\nlicense = \"Apache-2.0\"\nminimum_app_version = \"0.1.0\"\npolicy_files = []\nguidance_files = []\n",
        ),
        (
            "SKILL.md",
            "---\nname: portable-demo\ndescription: Test fixture\n---\n\nInstructions remain data.\n",
        ),
        ("references/example.txt", "Preserve exact bytes.\n"),
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

fn adapter(grok_home: Option<PathBuf>) -> GrokAdapter {
    let registry = AgentRegistry::built_in().unwrap();
    GrokAdapter::with_paths(
        registry
            .get(jameskills_core::domain::agent::AgentId::Grok)
            .unwrap()
            .clone(),
        std::env::temp_dir().join("Grok User Home With Spaces"),
        grok_home,
    )
    .unwrap()
}

#[test]
fn grok_user_project_and_override_paths_are_documented_and_bytes_preserved() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let default_home = std::env::temp_dir().join("Grok User Home With Spaces");
    let default_adapter = adapter(None);

    let user = default_adapter
        .plan_artifact(&validated, files.clone(), None, Scope::User)
        .unwrap();
    let expected_user = default_home
        .join(".grok")
        .join("skills")
        .join("portable-demo");
    assert_eq!(user.target(), expected_user.as_path());
    assert_eq!(user.files(), &files);

    let custom_home = std::env::temp_dir().join("Grok Custom Home With Spaces");
    let custom_adapter = adapter(Some(custom_home.clone()));
    let overridden = custom_adapter
        .plan_artifact(&validated, files.clone(), None, Scope::User)
        .unwrap();
    assert_eq!(
        overridden.target(),
        custom_home.join("skills").join("portable-demo").as_path()
    );

    let project_root = std::env::temp_dir().join("Grok Project With Spaces");
    let root = ApprovedRoot::from_absolute_path(project_root.clone()).unwrap();
    let project = default_adapter
        .plan_artifact(&validated, files.clone(), Some(&root), Scope::Project)
        .unwrap();
    assert_eq!(
        project.target(),
        project_root
            .join(".grok")
            .join("skills")
            .join("portable-demo")
            .as_path()
    );
    assert_eq!(project.files(), &files);
}

#[test]
fn grok_rejects_relative_home_overrides_and_missing_project_roots() {
    let registry = AgentRegistry::built_in().unwrap();
    assert!(
        GrokAdapter::with_paths(
            registry
                .get(jameskills_core::domain::agent::AgentId::Grok)
                .unwrap()
                .clone(),
            std::env::temp_dir().join("Grok Home"),
            Some(PathBuf::from("relative/.grok")),
        )
        .is_err()
    );

    let files = files();
    let validated = validate_bundle(&files).unwrap();
    assert!(
        adapter(None)
            .plan_artifact(&validated, files, None, Scope::Project)
            .is_err()
    );
}

#[test]
fn grok_artifact_rejects_bytes_that_no_longer_match_the_validated_bundle() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let mut changed = files;
    changed.insert(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"---\nname: portable-demo\ndescription: changed\n---\n".to_vec(),
    );

    assert!(matches!(
        adapter(None).plan_artifact(&validated, changed, None, Scope::User),
        Err(AppError::Conflict { .. })
    ));
}

#[test]
fn grok_detection_stays_candidate_until_a_reviewed_version_fixture_exists() {
    let adapter = adapter(None);
    let detection =
        block_on(adapter.detect(DetectionContext::new(Scope::User, None).unwrap())).unwrap();

    assert!(matches!(
        detection.availability(),
        AgentAvailability::Candidate | AgentAvailability::Missing
    ));
    assert!(detection.version().is_none());
    assert_eq!(
        detection
            .capabilities()
            .get(jameskills_core::domain::agent::AgentCapabilityId::UserInstall),
        CapabilitySupport::NeedsVerification
    );
}
