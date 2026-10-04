use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempBundle {
    root: std::path::PathBuf,
}

impl TempBundle {
    fn invalid_manifest() -> Self {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "jameskills-invalid-bundle-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("SKILL.md"),
            "---\nname: invalid\ndescription: Invalid\n---\nBody\n",
        )
        .unwrap();
        std::fs::write(
            root.join("jameskills.toml"),
            "schema_version = 1\nid = \"not-a-uuid\"\n",
        )
        .unwrap();
        Self { root }
    }
}

impl Drop for TempBundle {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn official_repository_example_validates_through_the_cli() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/repository-foundation");
    let output = Command::new(env!("CARGO_BIN_EXE_jameskills"))
        .args(["validate", "--path", root.to_str().unwrap(), "--json"])
        .output()
        .expect("CLI test binary starts");

    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["command"], "validate");
    assert_eq!(value["data"]["valid"], true);
    assert_eq!(value["data"]["slug"], "repository-foundation");
    assert_eq!(value["data"]["version"], "1.0.0");
    assert!(value["data"]["file_count"].as_u64().unwrap() >= 9);
    assert!(value["data"]["content_hash"].as_str().unwrap().len() == 64);
}

#[test]
fn invalid_bundle_reports_relative_path_and_code_in_json_and_text() {
    let bundle = TempBundle::invalid_manifest();
    let path = bundle.root.to_str().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_jameskills"))
        .args(["validate", "--path", path, "--json"])
        .output()
        .expect("CLI test binary starts");

    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["command"], "validate");
    assert_eq!(value["data"]["valid"], false);
    assert_eq!(value["data"]["diagnostics"][0]["path"], "jameskills.toml");
    assert_eq!(value["data"]["diagnostics"][0]["code"], "manifest.invalid");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("not-a-uuid"));

    let output = Command::new(env!("CARGO_BIN_EXE_jameskills"))
        .args(["validate", "--path", path])
        .output()
        .expect("CLI test binary starts");
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("jameskills.toml"));
    assert!(stdout.contains("manifest.invalid"));
    assert!(!stdout.contains("not-a-uuid"));
}
