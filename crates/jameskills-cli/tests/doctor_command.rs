use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempPath(PathBuf);

impl TempPath {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jameskills-doctor-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run_with_path(path: &std::path::Path, json: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_jameskills"));
    command.arg("doctor");
    if json {
        command.arg("--json");
    }
    command
        .env("PATH", path)
        .output()
        .expect("doctor executable starts by absolute path")
}

#[test]
fn doctor_lists_missing_tools_with_registered_guides_without_installing() {
    let path = TempPath::new();
    let output = run_with_path(path.path(), true);
    assert_eq!(output.status.code(), Some(0));
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let tools = response["data"]["tools"]
        .as_array()
        .expect("doctor must report registered tools");
    let git = tools
        .iter()
        .find(|tool| tool["id"] == "git")
        .expect("Git is registered in the app profile");
    assert_eq!(git["availability"], "missing");
    assert_eq!(git["capabilities"]["repository-root"], "unsupported");
    let guidance = response["data"]["guidance"]
        .as_array()
        .expect("missing tools must produce visible guide actions");
    assert!(guidance.iter().any(|step| {
        step["tool_id"] == "git"
            && step["action"]["kind"] == "open-official-url"
            && step["action"]["source_id"]
                == if cfg!(windows) {
                    "git-install-windows"
                } else {
                    "git-install-linux"
                }
            && step["action"]["url"].is_string()
    }));
    let output_text = String::from_utf8_lossy(&output.stdout);
    assert!(!output_text.contains(&path.path().to_string_lossy().to_string()));
    assert!(std::fs::read_dir(path.path()).unwrap().next().is_none());
}

#[test]
fn doctor_never_runs_or_marks_an_unfingerprinted_native_candidate_verified() {
    let path = TempPath::new();
    let executable = path
        .path()
        .join(if cfg!(windows) { "git.exe" } else { "git" });
    std::fs::write(&executable, b"not a runnable executable").unwrap();
    let output = run_with_path(path.path(), true);
    assert_eq!(output.status.code(), Some(0));
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let git = response["data"]["tools"]
        .as_array()
        .and_then(|tools| tools.iter().find(|tool| tool["id"] == "git"))
        .expect("doctor must include the candidate tool");
    assert_eq!(git["availability"], "candidate");
    assert_eq!(git["version"], serde_json::Value::Null);
    assert!(git["capabilities"]["repository-root"] == "needs-verification");
    let output_text = String::from_utf8_lossy(&output.stdout);
    assert!(!output_text.contains(&path.path().to_string_lossy().to_string()));
    assert_eq!(
        std::fs::read(&executable).unwrap(),
        b"not a runnable executable"
    );
}

#[test]
fn doctor_text_shows_tool_status_and_official_next_step() {
    let path = TempPath::new();
    let output = run_with_path(path.path(), false);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Tools:"));
    assert!(text.contains("git: missing"));
    assert!(text.contains("Next steps:"));
    assert!(text.contains(if cfg!(windows) {
        "https://git-scm.com/install/windows"
    } else {
        "https://git-scm.com/install/linux"
    }));
}
