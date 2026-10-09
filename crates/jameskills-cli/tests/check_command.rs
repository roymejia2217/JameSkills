use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-cli-check-{}-{}",
            std::process::id(),
            NEXT_CASE.fetch_add(1, Ordering::Relaxed)
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

fn run(app_data_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_jameskills"))
        .arg("--app-data-dir")
        .arg(app_data_dir)
        .args(args)
        .output()
        .expect("CLI binary starts")
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("CLI emits its JSON envelope")
}

#[test]
fn real_cli_imports_fixture_and_checks_its_policy_in_isolated_storage() {
    let case = TestRoot::new();
    let app_data = case.0.join("app-data");
    let repository = case.0.join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    std::fs::write(
        repository.join("README.md"),
        "# Inicio rápido\n\nStart.\n\n# Arquitectura\n\nArchitecture.\n\n# Contribuir\n\nContribute.\n",
    )
    .unwrap();
    std::fs::write(
        repository.join("Cargo.toml"),
        "[package]\nname = \"check-command-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    let bundle = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("repository-foundation");

    let preview = run(
        &app_data,
        &[
            "library",
            "import",
            "--path",
            bundle.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(preview.status.code(), Some(0));
    let preview_json = json(&preview);
    assert_eq!(preview_json["data"]["phase"], "preview");
    let skill_id = preview_json["data"]["skill_id"].as_str().unwrap();
    let confirmation = preview_json["data"]["confirmations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["resolution"] == "add-concurrent-root")
        .unwrap();
    let digest = confirmation["digest"].as_str().unwrap();

    let applied = run(
        &app_data,
        &[
            "library",
            "import",
            "--path",
            bundle.to_str().unwrap(),
            "--apply",
            "--resolution",
            "add-concurrent-root",
            "--confirmation-digest",
            digest,
            "--json",
        ],
    );
    assert_eq!(applied.status.code(), Some(0));
    assert_eq!(json(&applied)["data"]["trust_state"], "quarantined");

    let checked = run(
        &app_data,
        &[
            "check",
            "--repo",
            repository.to_str().unwrap(),
            "--skill",
            skill_id,
            "--profile",
            "rust",
            "--strict",
            "--json",
        ],
    );
    assert_eq!(checked.status.code(), Some(1));
    let checked_json = json(&checked);
    assert_eq!(checked_json["schema_version"], 1);
    assert_eq!(checked_json["command"], "check");
    assert_eq!(checked_json["error"]["code"], "checks.failed");
    assert_eq!(checked_json["data"]["skill_id"], skill_id);
    assert_eq!(checked_json["data"]["required_passed"], false);
    let results = checked_json["data"]["results"].as_array().unwrap();
    assert!(results.iter().any(|result| {
        result["requirement_id"] == "readme-structure" && result["status"] == "pass"
    }));
    assert!(results.iter().any(|result| {
        result["requirement_id"] == "git-ready" && result["status"] == "unknown"
    }));
    let stdout = String::from_utf8(checked.stdout).unwrap();
    assert!(!stdout.contains(repository.to_str().unwrap()));
    assert!(!stdout.contains(app_data.to_str().unwrap()));
}

#[test]
fn relative_app_data_override_is_rejected_without_echo_or_storage_creation() {
    let relative = format!("relative-jameskills-data-{}", std::process::id());
    let _ = std::fs::remove_dir_all(&relative);
    let output = Command::new(env!("CARGO_BIN_EXE_jameskills"))
        .args(["--app-data-dir", &relative, "doctor", "--json"])
        .output()
        .expect("CLI binary starts");

    assert_eq!(output.status.code(), Some(2));
    let response = json(&output);
    assert_eq!(response["error"]["code"], "input.invalid");
    assert!(!String::from_utf8_lossy(&output.stdout).contains(&relative));
    assert!(!Path::new(&relative).exists());
}
