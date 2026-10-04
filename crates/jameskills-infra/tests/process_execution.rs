use jameskills_core::{
    AppError,
    domain::ToolId,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken, ProcessPermission,
        ProcessPort, ProcessSpec,
    },
};
use jameskills_infra::process::{SystemProcessPort, fingerprint_executable};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::Write,
    time::{Duration, Instant},
};

fn safe_environment() -> ApprovedEnv {
    let mut values = BTreeMap::new();
    for key in [
        "PATH",
        "SystemRoot",
        "WINDIR",
        "HOME",
        "USERPROFILE",
        "TEMP",
        "TMP",
    ] {
        if let Some(value) = std::env::var_os(key) {
            values.insert(OsString::from(key), value);
        }
    }
    ApprovedEnv::new(values).unwrap()
}

fn helper_spec(
    mode: &str,
    timeout: Duration,
    output_cap: usize,
    cancellation: CancellationToken,
) -> ProcessSpec {
    let executable =
        ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap();
    let cwd = ApprovedRoot::from_absolute_path(std::env::current_dir().unwrap()).unwrap();
    let args = ["--exact", mode, "--ignored", "--nocapture"]
        .into_iter()
        .map(OsString::from)
        .collect();
    ProcessSpec::new(
        executable,
        ToolId::Git,
        args,
        cwd,
        safe_environment(),
        timeout,
        output_cap,
        ProcessPermission::ReadOnlyCheck,
        cancellation,
    )
    .unwrap()
}

fn run(spec: ProcessSpec) -> Result<jameskills_core::ports::process::ProcessOutput, AppError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(SystemProcessPort.run(spec))
}

#[test]
#[ignore = "spawned only by ProcessPort integration tests"]
fn process_probe_output() {
    println!("stdout-process-canary");
    eprintln!("stderr-process-canary");
}

#[test]
#[ignore = "spawned only by ProcessPort integration tests"]
fn process_probe_sleep() {
    std::thread::sleep(Duration::from_secs(30));
}

#[test]
#[ignore = "spawned only by ProcessPort integration tests"]
fn process_probe_flood() {
    let chunk = vec![b'x'; 8192];
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    loop {
        stdout.write_all(&chunk).unwrap();
        stderr.write_all(&chunk).unwrap();
        stdout.flush().unwrap();
        stderr.flush().unwrap();
    }
}

#[test]
fn process_runner_drains_both_streams_and_keeps_output_bounded() {
    let output = run(helper_spec(
        "process_probe_output",
        Duration::from_secs(5),
        4096,
        CancellationToken::new(),
    ))
    .unwrap();

    assert_eq!(output.exit_code(), Some(0));
    assert!(String::from_utf8_lossy(output.stdout()).contains("stdout-process-canary"));
    assert!(String::from_utf8_lossy(output.stderr()).contains("stderr-process-canary"));
}

#[test]
fn process_runner_terminates_group_after_output_overflow() {
    let error = match run(helper_spec(
        "process_probe_flood",
        Duration::from_secs(10),
        256,
        CancellationToken::new(),
    )) {
        Ok(_) => panic!("output overflow must fail closed"),
        Err(error) => error,
    };
    assert!(matches!(error, AppError::ExternalTool { .. }));
}

#[test]
fn process_runner_terminates_group_on_timeout_and_cancellation() {
    let started = Instant::now();
    let timed_out = run(helper_spec(
        "process_probe_sleep",
        Duration::from_millis(100),
        1024,
        CancellationToken::new(),
    ));
    assert!(matches!(timed_out, Err(AppError::ExternalTool { .. })));
    assert!(started.elapsed() < Duration::from_secs(5));

    let cancellation = CancellationToken::new();
    let notifier = cancellation.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        notifier.cancel();
    });
    let cancelled = run(helper_spec(
        "process_probe_sleep",
        Duration::from_secs(10),
        1024,
        cancellation,
    ));
    assert!(matches!(cancelled, Err(AppError::Cancelled)));
}

#[test]
fn process_runner_rejects_an_executable_replaced_after_fingerprint_approval() {
    let path = std::env::temp_dir().join(format!(
        "jameskills candidate fingerprint-{}.bin",
        std::process::id()
    ));
    std::fs::write(&path, b"candidate before approval").unwrap();
    let approved = fingerprint_executable(&path).unwrap();
    std::fs::write(&path, b"candidate replaced after approval").unwrap();

    let spec = ProcessSpec::new(
        ApprovedExecutable::from_absolute_path(std::fs::canonicalize(&path).unwrap()).unwrap(),
        ToolId::Git,
        vec![],
        ApprovedRoot::from_absolute_path(std::env::current_dir().unwrap()).unwrap(),
        safe_environment(),
        Duration::from_secs(2),
        4096,
        ProcessPermission::ReadOnlyCheck,
        CancellationToken::new(),
    )
    .unwrap()
    .with_approved_executable_fingerprint(approved);

    let error = match run(spec) {
        Err(error) => error,
        Ok(_) => panic!("changed executable identity must block process spawn"),
    };
    assert!(matches!(
        error,
        AppError::PermissionDenied { operation }
            if operation == "process.executable.identity_changed"
    ));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn process_runner_accepts_an_unchanged_approved_executable() {
    let executable_path = std::env::current_exe().unwrap();
    let approved = fingerprint_executable(&executable_path).unwrap();
    let spec = helper_spec(
        "process_probe_output",
        Duration::from_secs(5),
        4096,
        CancellationToken::new(),
    )
    .with_approved_executable_fingerprint(approved);

    assert_eq!(run(spec).unwrap().exit_code(), Some(0));
}
