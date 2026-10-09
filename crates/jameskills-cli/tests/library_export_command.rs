use jameskills_core::{domain::validate_bundle, ports::filesystem::FileSystemPort};
use jameskills_infra::fs::LocalFileSystem;
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
            "jameskills-cli-export-{}-{}",
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

fn run(app_data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_jameskills"))
        .arg("--app-data-dir")
        .arg(app_data)
        .args(args)
        .output()
        .expect("CLI binary starts")
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("CLI emits a JSON envelope")
}

fn fixture_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("repository-foundation")
}

#[test]
fn export_preview_apply_and_stale_overwrite_preserve_portable_bytes() {
    let root = TestRoot::new();
    let app_data = root.0.join("app-data");
    let bundle = fixture_directory();
    let import_preview = run(
        &app_data,
        &[
            "library",
            "import",
            "--path",
            bundle.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(import_preview.status.code(), Some(0));
    let import_json = json(&import_preview);
    let skill_id = import_json["data"]["skill_id"].as_str().unwrap();
    let import_confirmation = import_json["data"]["confirmations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["resolution"] == "add-concurrent-root")
        .unwrap();
    let import_digest = import_confirmation["digest"].as_str().unwrap();
    let imported = run(
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
            import_digest,
            "--json",
        ],
    );
    assert_eq!(imported.status.code(), Some(0));
    let revision_id = json(&imported)["data"]["revision_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let output_path = root.0.join("exports").join("repository.jskill");
    std::fs::create_dir_all(output_path.parent().unwrap()).unwrap();

    let preview = run(
        &app_data,
        &[
            "library",
            "export",
            "--skill",
            skill_id,
            "--revision",
            &revision_id,
            "--output",
            output_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(preview.status.code(), Some(0));
    let preview_json = json(&preview);
    assert_eq!(preview_json["data"]["phase"], "preview");
    assert_eq!(preview_json["data"]["destination"]["kind"], "missing");
    assert!(
        !String::from_utf8(preview.stdout.clone())
            .unwrap()
            .contains(root.0.to_str().unwrap())
    );
    assert!(!output_path.exists());
    let digest = preview_json["data"]["confirmation_digest"]
        .as_str()
        .unwrap()
        .to_owned();

    let applied = run(
        &app_data,
        &[
            "library",
            "export",
            "--skill",
            skill_id,
            "--revision",
            &revision_id,
            "--output",
            output_path.to_str().unwrap(),
            "--apply",
            "--confirmation-digest",
            &digest,
            "--json",
        ],
    );
    assert_eq!(applied.status.code(), Some(0));
    let applied_json = json(&applied);
    assert_eq!(applied_json["data"]["phase"], "applied");
    assert_eq!(applied_json["data"]["revision_id"], revision_id);
    let filesystem = LocalFileSystem;
    let files = filesystem.read_bundle_source(&output_path).unwrap();
    let exported = validate_bundle(&files).unwrap();
    assert_eq!(
        exported.content_hash().as_str(),
        applied_json["data"]["content_hash"].as_str().unwrap()
    );
    let original_archive = std::fs::read(&output_path).unwrap();

    let overwrite_preview = run(
        &app_data,
        &[
            "library",
            "export",
            "--skill",
            skill_id,
            "--revision",
            &revision_id,
            "--output",
            output_path.to_str().unwrap(),
            "--json",
        ],
    );
    let overwrite_json = json(&overwrite_preview);
    assert_eq!(overwrite_json["data"]["destination"]["kind"], "existing");
    assert_eq!(overwrite_json["data"]["overwrite_required"], true);
    let stale_digest = overwrite_json["data"]["confirmation_digest"]
        .as_str()
        .unwrap()
        .to_owned();
    let missing_overwrite = run(
        &app_data,
        &[
            "library",
            "export",
            "--skill",
            skill_id,
            "--revision",
            &revision_id,
            "--output",
            output_path.to_str().unwrap(),
            "--apply",
            "--confirmation-digest",
            &stale_digest,
            "--json",
        ],
    );
    assert_eq!(missing_overwrite.status.code(), Some(2));
    assert_eq!(std::fs::read(&output_path).unwrap(), original_archive);
    let overwrite_applied = run(
        &app_data,
        &[
            "library",
            "export",
            "--skill",
            skill_id,
            "--revision",
            &revision_id,
            "--output",
            output_path.to_str().unwrap(),
            "--apply",
            "--overwrite",
            "--confirmation-digest",
            &stale_digest,
            "--json",
        ],
    );
    assert_eq!(overwrite_applied.status.code(), Some(0));
    assert_eq!(json(&overwrite_applied)["data"]["overwritten"], true);

    std::fs::write(&output_path, b"external edit to preserve").unwrap();
    let stale = run(
        &app_data,
        &[
            "library",
            "export",
            "--skill",
            skill_id,
            "--revision",
            &revision_id,
            "--output",
            output_path.to_str().unwrap(),
            "--apply",
            "--overwrite",
            "--confirmation-digest",
            &stale_digest,
            "--json",
        ],
    );
    assert_eq!(stale.status.code(), Some(4));
    assert_eq!(
        json(&stale)["error"]["code"],
        "library.export.confirmation.stale"
    );
    assert_eq!(
        std::fs::read(&output_path).unwrap(),
        b"external edit to preserve"
    );
    let report = String::from_utf8(stale.stdout).unwrap();
    assert!(!report.contains(root.0.to_str().unwrap()));
}
