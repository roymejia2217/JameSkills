use jameskills_core::{
    application::policy::CheckRequest,
    domain::{parse_policy, policy::CheckStatus},
};
use jameskills_infra::{
    composition::{RepositoryCheckTools, build_services},
    platform::UserDirectories,
};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(std::path::PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-repository-policy-runtime-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn per_repository_policy_service_runs_filesystem_checks_and_blocks_unapproved_tools() {
    const POLICY: &str = r#"
schema_version = 1
profile = "repository-policy-runtime"

[[tool_requirements]]
tool_id = "gitleaks"
operation = "scan-tracked"
version = ">=8.0.0"

[[requirements]]
id = "readme-check"
description = "README has a nonempty Start section."
severity = "error"
required = true
phase = "pre-install"
enforcement = "local-check"
depends_on = []
[requirements.check]
kind = "readme-sections"
path = "README.md"
headings = ["Start"]

[[requirements]]
id = "secret-check"
description = "The selected tree is scanned for secrets."
severity = "error"
required = true
phase = "commit"
enforcement = "local-check"
depends_on = []
[requirements.check]
kind = "tracked-secrets"
include_history = false

[[requirements]]
id = "rust-readme"
description = "Rust projects include a Start section."
severity = "error"
required = true
phase = "pre-install"
enforcement = "local-check"
depends_on = []
applies_when = { fact = "stack", equals = "rust" }
[requirements.check]
kind = "readme-sections"
path = "README.md"
headings = ["Start"]
"#;
    let root = TestRoot::new();
    std::fs::write(
        root.0.join("README.md"),
        "# Start\n\nProject documentation.\n",
    )
    .unwrap();
    std::fs::write(
        root.0.join("Cargo.toml"),
        "[package]\nname = \"check-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    let app_dirs = TestRoot::new();
    let runtime = build_services(UserDirectories {
        config: app_dirs.0.join("config"),
        data: app_dirs.0.join("data"),
        cache: app_dirs.0.join("cache"),
    })
    .unwrap();
    let service = runtime
        .policy_for_repository(&root.0, RepositoryCheckTools::default())
        .unwrap();
    let context = runtime.repository_check_context(&root.0, "rust").unwrap();
    assert!(runtime.repository_check_context(&root.0, "node").is_err());
    let report = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(service.check(CheckRequest::new(
            parse_policy(POLICY.as_bytes()).unwrap(),
            context,
        )))
        .unwrap();

    assert_eq!(report.results().len(), 3);
    assert_eq!(report.results()[0].status(), CheckStatus::Pass);
    assert_eq!(report.results()[1].status(), CheckStatus::Blocked);
    assert_eq!(report.results()[2].status(), CheckStatus::Pass);
    assert_eq!(report.strict_exit(), 1);
}
