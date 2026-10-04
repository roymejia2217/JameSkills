use jameskills_core::{
    domain::ToolId,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken, ExecutableFingerprint,
        ProcessPermission, ProcessSpec,
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
