use jameskills_core::domain::{
    GuidanceProgressStatus, GuidanceStepStatus, PortablePath, skill::validate_bundle,
};
use jameskills_infra::{
    composition::build_services,
    platform::{HostPlatform, UserDirectories},
};
use std::{collections::BTreeMap, sync::Arc};

const MANIFEST: &str = include_str!("../../../docs/examples/repository-foundation/jameskills.toml");
const SKILL: &str = include_str!("../../../docs/examples/repository-foundation/SKILL.md");
const POLICY: &str =
    include_str!("../../../docs/examples/repository-foundation/policies/repository.toml");
const GUIDANCE: &str =
    include_str!("../../../docs/examples/repository-foundation/guidance/repository.toml");

fn bundle(guidance: &str) -> Arc<jameskills_core::domain::ValidatedBundle> {
    let files = [
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
    .collect::<BTreeMap<_, _>>();
    Arc::new(validate_bundle(&files).unwrap())
}

fn runtime() -> jameskills_infra::composition::RuntimeServices {
    let root = std::env::temp_dir().join(format!(
        "jameskills-guidance-runtime-{}",
        std::process::id()
    ));
    build_services(UserDirectories {
        config: root.join("config"),
        data: root.join("data"),
        cache: root.join("cache"),
    })
    .unwrap()
}

fn platform_os(platform: HostPlatform) -> &'static str {
    match platform {
        HostPlatform::Windows => "windows",
        HostPlatform::Linux => "linux",
        HostPlatform::Other => "other",
    }
}

fn guide_for(os: &str) -> String {
    GUIDANCE.replacen(
        "verification_requirement_ids = [\"git-ready\"]",
        &format!(
            "verification_requirement_ids = [\"git-ready\"]\napplies_when = {{ fact = \"os\", equals = \"{os}\" }}"
        ),
        1,
    )
}

#[test]
fn runtime_guidance_branches_on_observed_os_without_promoting_unknown_checks() {
    let runtime = runtime();
    let current_os = platform_os(runtime.facts().platform);
    let other_os = if current_os == "windows" {
        "linux"
    } else {
        "windows"
    };
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let skipped = executor
        .block_on(
            runtime
                .guidance()
                .start_guidance(bundle(&guide_for(other_os)), "git-setup"),
        )
        .unwrap();
    assert_eq!(
        skipped.decision().status(),
        GuidanceProgressStatus::Complete
    );
    assert_eq!(
        skipped.decision().steps()[0].status(),
        GuidanceStepStatus::NotApplicable
    );

    let applicable = executor
        .block_on(
            runtime
                .guidance()
                .start_guidance(bundle(&guide_for(current_os)), "git-setup"),
        )
        .unwrap();
    assert_eq!(
        applicable.decision().status(),
        GuidanceProgressStatus::AwaitingEvidence
    );
    assert_eq!(
        applicable.decision().steps()[0].verification_status(),
        jameskills_core::domain::policy::CheckStatus::Unknown
    );
}
