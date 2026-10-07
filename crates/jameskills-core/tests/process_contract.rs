use jameskills_core::{
    domain::{AgentId, ToolId},
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken, ExecutableFingerprint,
        ProcessIdentity, ProcessPermission, ProcessSpec,
    },
};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf, time::Duration};

fn approved_executable() -> ApprovedExecutable {
    ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap()
}

fn approved_root() -> ApprovedRoot {
    ApprovedRoot::from_absolute_path(std::env::current_dir().unwrap()).unwrap()
}

#[test]
fn process_arguments_preserve_spaces_and_metacharacters_as_separate_argv() {
    let args = vec![
        OsString::from("rev-parse"),
        OsString::from("path with spaces; $(not-a-shell)"),
    ];
    let spec = ProcessSpec::new(
        approved_executable(),
        ToolId::Git,
        args.clone(),
        approved_root(),
        ApprovedEnv::new(BTreeMap::new()).unwrap(),
        Duration::from_secs(2),
        4096,
        ProcessPermission::ReadOnlyCheck,
        CancellationToken::new(),
    )
    .unwrap();

    assert_eq!(spec.args(), args);
    assert_eq!(spec.output_limit_bytes(), 4096);
    assert!(spec.approved_executable_fingerprint().is_none());
    assert!(matches!(
        spec.permission(),
        ProcessPermission::ReadOnlyCheck
    ));
}

#[test]
fn executable_root_and_environment_require_explicit_safe_values() {
    assert!(ApprovedExecutable::from_absolute_path(PathBuf::from("relative/git")).is_err());
    assert!(ApprovedRoot::from_absolute_path(PathBuf::from("relative/repository")).is_err());

    let secret = BTreeMap::from([(
        OsString::from("XAI_API_KEY"),
        OsString::from("not-a-real-secret"),
    )]);
    assert!(ApprovedEnv::new(secret).is_err());
}

#[test]
fn approved_environment_accepts_msvc_tool_paths_without_compiler_options() {
    let tool_paths = BTreeMap::from([
        (OsString::from("INCLUDE"), OsString::from("C:\\VS\\include")),
        (OsString::from("LIB"), OsString::from("C:\\VS\\lib")),
        (
            OsString::from("LIBPATH"),
            OsString::from("C:\\VS\\metadata"),
        ),
        (OsString::from("VCINSTALLDIR"), OsString::from("C:\\VS\\VC")),
        (
            OsString::from("VCToolsInstallDir"),
            OsString::from("C:\\VS\\VC\\Tools"),
        ),
        (
            OsString::from("WindowsSdkDir"),
            OsString::from("C:\\Windows Kits\\10"),
        ),
        (
            OsString::from("WindowsSDKVersion"),
            OsString::from("10.0.26100.0\\"),
        ),
        (
            OsString::from("UniversalCRTSdkDir"),
            OsString::from("C:\\Windows Kits\\10"),
        ),
        (
            OsString::from("UCRTVersion"),
            OsString::from("10.0.26100.0"),
        ),
    ]);
    let environment = ApprovedEnv::new(tool_paths).unwrap();
    assert_eq!(environment.entries().len(), 9);

    let compiler_options = BTreeMap::from([(
        OsString::from("CL"),
        OsString::from("/DUNSAFE /link arbitrary.exe"),
    )]);
    assert!(ApprovedEnv::new(compiler_options).is_err());
}

#[test]
fn github_cli_can_resolve_windows_user_config_without_host_or_token_overrides() {
    let appdata = BTreeMap::from([(
        OsString::from("APPDATA"),
        OsString::from("C:\\Users\\user\\AppData\\Roaming"),
    )]);
    assert!(ApprovedEnv::new(appdata).is_ok());

    for forbidden in ["GH_HOST", "GH_TOKEN", "GITHUB_TOKEN"] {
        let entries = BTreeMap::from([(OsString::from(forbidden), OsString::from("untrusted"))]);
        assert!(ApprovedEnv::new(entries).is_err(), "{forbidden}");
    }
}

#[test]
fn cancellation_token_is_shared_across_process_owners() {
    let token = CancellationToken::new();
    let worker = token.clone();
    assert!(!worker.is_cancelled());
    token.cancel();
    assert!(worker.is_cancelled());
}

#[test]
fn process_spec_carries_the_explicitly_approved_executable_fingerprint() {
    let fingerprint = ExecutableFingerprint::from_sha256([0x42; 32]);
    let spec = ProcessSpec::new(
        approved_executable(),
        ToolId::Git,
        vec![OsString::from("--version")],
        approved_root(),
        ApprovedEnv::new(BTreeMap::new()).unwrap(),
        Duration::from_secs(2),
        4096,
        ProcessPermission::ReadOnlyCheck,
        CancellationToken::new(),
    )
    .unwrap()
    .with_approved_executable_fingerprint(fingerprint);

    assert!(spec.approved_executable_fingerprint() == Some(&fingerprint));
    assert_eq!(fingerprint.as_bytes(), &[0x42; 32]);
}

#[test]
fn agent_process_identity_stays_separate_from_policy_tool_ids() {
    let spec = ProcessSpec::new_for_agent(
        approved_executable(),
        AgentId::Codex,
        vec![OsString::from("--version")],
        approved_root(),
        ApprovedEnv::new(BTreeMap::new()).unwrap(),
        Duration::from_secs(2),
        4096,
        ProcessPermission::ReadOnlyCheck,
        CancellationToken::new(),
    )
    .unwrap();

    assert!(spec.process_identity() == ProcessIdentity::Agent(AgentId::Codex));
    assert!(spec.tool_id().is_none());
    assert_eq!(spec.args(), [OsString::from("--version")]);
    assert!(matches!(
        spec.permission(),
        ProcessPermission::ReadOnlyCheck
    ));
}
