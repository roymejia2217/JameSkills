use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_jameskills"))
        .args(args)
        .output()
        .expect("CLI test binary starts")
}

#[test]
fn help_lists_the_supported_command_tree() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("doctor"));
    assert!(stdout.contains("validate"));
    assert!(stdout.contains("library"));
    assert!(stdout.contains("agents"));
    assert!(stdout.contains("backup"));
    assert!(stdout.contains("sync"));
}

#[test]
fn invalid_arguments_return_redacted_stable_json_and_exit_two() {
    let output = run(&["not-a-command", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["command"], "parse");
    assert_eq!(value["error"]["code"], "arguments.invalid");
    assert_eq!(value["error"]["message"], "Command arguments are invalid.");
}

#[test]
fn commands_without_an_implemented_backend_never_report_success() {
    let output = run(&["sync", "run", "--json"]);
    assert_eq!(output.status.code(), Some(3));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["command"], "sync run");
    assert_eq!(value["data"], serde_json::Value::Null);
    assert_eq!(value["error"]["code"], "capability.unsupported");
}

#[test]
fn doctor_json_reports_observed_platform_facts_without_paths() {
    let output = run(&["doctor", "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["command"], "doctor");
    assert!(value["data"]["architecture"].is_string());
    assert!(value["data"]["platform"].is_string());
    assert!(value["data"]["display_environment"].is_string());
    assert!(value["data"]["gpu_device"].is_string());
    assert!(!String::from_utf8(output.stdout).unwrap().contains("/home/"));
}

#[test]
fn secrets_are_not_accepted_as_command_line_flags_or_echoed() {
    let secret = "cli-test-secret-927";
    let output = run(&[
        "backup",
        "restore",
        "--input",
        "backup.jskills-backup",
        "--password",
        secret,
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let output_text = String::from_utf8_lossy(&output.stdout);
    assert!(!output_text.contains(secret));
    assert!(output_text.contains("arguments.invalid"));
}

#[test]
fn check_accepts_typed_profile_and_strict_flags_but_stays_unsupported() {
    let output = run(&[
        "check",
        "--repo",
        ".",
        "--skill",
        "550e8400-e29b-41d4-a716-446655440000",
        "--profile",
        "rust",
        "--strict",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(3));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["command"], "check");
    assert_eq!(value["error"]["code"], "capability.unsupported");
}

#[test]
fn apply_restore_requires_digest_confirmation() {
    let output = run(&[
        "backup",
        "restore",
        "--input",
        "backup.jskills-backup",
        "--apply",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "arguments.invalid");
}

#[test]
fn non_json_argument_errors_do_not_echo_untrusted_values() {
    let secret = "parser-output-canary-638";
    let output = run(&["check", "--repo", ".", "--skill", secret]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(secret));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
}
