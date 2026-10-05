use async_trait::async_trait;
use jameskills_core::{
    AppError,
    application::policy::{TestSuiteRunApproval, TestSuiteService},
    domain::{
        ToolId,
        policy::{TestSuiteDeclaration, TestSuiteExecution, TestSuiteKind},
    },
    ports::ClockPort,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken, ProcessOutput,
        ProcessPermission, ProcessPort, ProcessSpec,
    },
};
use jameskills_infra::{
    fs::{ApprovedRepositoryTool, RepositoryPolicyCheckProvider},
    process::{SystemProcessPort, fingerprint_executable},
};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

const NOW: &str = "2026-10-04T00:00:00Z";
const ENVIRONMENT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEAD_A: &[u8] = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n";
const HEAD_B: &[u8] = b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n";
const CARGO_METADATA_WITH_TEST_TARGET: &[u8] = br#"{
  "version": 1,
  "workspace_members": ["fixture 0.1.0 (path+file:///fixture)"],
  "packages": [{
    "id": "fixture 0.1.0 (path+file:///fixture)",
    "targets": [{ "name": "fixture", "kind": ["lib"], "test": true }]
  }]
}"#;
const CARGO_METADATA_WITHOUT_TEST_TARGET: &[u8] = br#"{
  "version": 1,
  "workspace_members": ["fixture 0.1.0 (path+file:///fixture)"],
  "packages": [{
    "id": "fixture 0.1.0 (path+file:///fixture)",
    "targets": [{ "name": "fixture", "kind": ["lib"], "test": false }]
  }]
}"#;
static NEXT_ROOT_ID: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-suite-check-{}-{}",
            std::process::id(),
            NEXT_ROOT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\n[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        Self(root)
    }

    fn tool(&self, name: &str) -> ApprovedRepositoryTool {
        let filename = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        };
        let path = self.0.join("bin").join(filename);
        std::fs::write(&path, format!("approved {name} fixture")).unwrap();
        let path = std::fs::canonicalize(path).unwrap();
        ApprovedRepositoryTool::new(
            ApprovedExecutable::from_absolute_path(path.clone()).unwrap(),
            fingerprint_executable(&path).unwrap(),
        )
    }

    fn approved(&self) -> ApprovedRoot {
        ApprovedRoot::from_absolute_path(std::fs::canonicalize(&self.0).unwrap()).unwrap()
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct TestClock;

impl ClockPort for TestClock {
    fn now_utc(&self) -> String {
        NOW.to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        1
    }
}

struct Invocation {
    tool_id: ToolId,
    executable: PathBuf,
    args: Vec<OsString>,
    cwd: PathBuf,
    permission: ProcessPermission,
    timeout: Duration,
    output_limit: usize,
    fingerprinted: bool,
}

struct FakeProcess {
    outputs: Mutex<VecDeque<ProcessOutput>>,
    invocations: Mutex<Vec<Invocation>>,
}

impl FakeProcess {
    fn new(outputs: Vec<ProcessOutput>) -> Self {
        Self {
            outputs: Mutex::new(outputs.into()),
            invocations: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ProcessPort for FakeProcess {
    async fn run(&self, spec: ProcessSpec) -> Result<ProcessOutput, AppError> {
        self.invocations.lock().unwrap().push(Invocation {
            tool_id: spec.tool_id(),
            executable: spec.executable().path().to_path_buf(),
            args: spec.args().to_vec(),
            cwd: spec.cwd().path().to_path_buf(),
            permission: spec.permission(),
            timeout: spec.timeout(),
            output_limit: spec.output_limit_bytes(),
            fingerprinted: spec.approved_executable_fingerprint().is_some(),
        });
        self.outputs
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::ExternalTool {
                tool_id: "test-process".to_owned(),
                exit_code: None,
            })
    }
}

fn output(code: i32, stdout: &[u8]) -> ProcessOutput {
    ProcessOutput::new(Some(code), stdout.to_vec(), Vec::new())
}

fn outputs(head: &[u8], target: &[u8], test_exit: i32) -> Vec<ProcessOutput> {
    vec![
        output(0, b"git version 2.50.0\n"),
        output(0, head),
        output(0, b"cargo 1.95.0\n"),
        output(0, target),
        output(0, b"git version 2.50.0\n"),
        output(0, head),
        output(0, b"cargo 1.95.0\n"),
        output(0, target),
        output(0, head),
        output(test_exit, b""),
    ]
}

fn runner(root: &TestRoot, process: Arc<FakeProcess>) -> TestSuiteService {
    let provider = RepositoryPolicyCheckProvider::new(
        root.approved(),
        Some(root.tool("git")),
        None,
        ApprovedEnv::new(BTreeMap::new()).unwrap(),
        process,
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_cargo(root.tool("cargo"));
    TestSuiteService::new(Arc::new(provider))
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    struct NoopWake;
    impl std::task::Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }
    let waker = std::task::Waker::from(Arc::new(NoopWake));
    let mut context = std::task::Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => return output,
            std::task::Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[test]
fn cargo_inspection_is_read_only_and_explicit_run_uses_registered_argv() {
    let root = TestRoot::new();
    let process = Arc::new(FakeProcess::new(outputs(
        HEAD_A,
        CARGO_METADATA_WITH_TEST_TARGET,
        0,
    )));
    let runner = runner(&root, Arc::clone(&process));
    let cancellation = jameskills_core::ports::process::CancellationToken::new();
    let snapshot =
        block_on(runner.inspect(TestSuiteKind::CargoTest, cancellation.clone())).unwrap();

    assert!(matches!(
        snapshot.declaration(),
        TestSuiteDeclaration::Declared
    ));
    let inspected = process.invocations.lock().unwrap();
    assert_eq!(inspected.len(), 4);
    assert!(
        inspected
            .iter()
            .all(|invocation| matches!(invocation.permission, ProcessPermission::ReadOnlyCheck))
    );
    assert!(inspected.iter().any(|invocation| {
        matches!(invocation.tool_id, ToolId::Cargo)
            && invocation.args.first().is_some_and(|arg| arg == "metadata")
    }));
    drop(inspected);

    let approval = TestSuiteRunApproval::after_explicit_trust_confirmation(snapshot);
    let operation_id = approval.operation_id();
    let result = block_on(runner.run(approval, cancellation)).unwrap();

    assert!(matches!(result.execution(), TestSuiteExecution::Passed));
    let invocations = process.invocations.lock().unwrap();
    let suite_run = invocations.last().unwrap();
    assert!(matches!(suite_run.tool_id, ToolId::Cargo));
    assert!(
        suite_run
            .executable
            .file_name()
            .is_some_and(|name| { name.to_string_lossy().starts_with("cargo") })
    );
    assert_eq!(suite_run.cwd, std::fs::canonicalize(&root.0).unwrap());
    assert!(matches!(
        suite_run.permission,
        ProcessPermission::ExplicitMutation(id) if id == operation_id
    ));
    assert_eq!(
        &suite_run.args[..4],
        &[
            OsString::from("test"),
            OsString::from("--workspace"),
            OsString::from("--locked"),
            OsString::from("--manifest-path"),
        ]
    );
    assert!(suite_run.args[4].to_string_lossy().ends_with("Cargo.toml"));
    assert!(!suite_run.args[4].to_string_lossy().starts_with(r"\\?\"));
    assert_eq!(suite_run.timeout, Duration::from_secs(120));
    assert_eq!(suite_run.output_limit, 64 * 1024);
    assert!(suite_run.fingerprinted);
}

#[test]
fn cargo_suite_missing_test_target_is_not_run_and_stale_head_blocks_spawn() {
    let root = TestRoot::new();
    let missing_process = Arc::new(FakeProcess::new(outputs(
        HEAD_A,
        CARGO_METADATA_WITHOUT_TEST_TARGET,
        0,
    )));
    let missing_runner = runner(&root, Arc::clone(&missing_process));
    let token = jameskills_core::ports::process::CancellationToken::new();
    let missing_snapshot =
        block_on(missing_runner.inspect(TestSuiteKind::CargoTest, token.clone())).unwrap();
    let missing = block_on(missing_runner.run(
        TestSuiteRunApproval::after_explicit_trust_confirmation(missing_snapshot),
        token,
    ))
    .unwrap();
    assert!(matches!(
        missing.declaration(),
        TestSuiteDeclaration::Missing
    ));
    assert!(matches!(missing.execution(), TestSuiteExecution::NotRun));
    assert!(
        missing_process
            .invocations
            .lock()
            .unwrap()
            .iter()
            .all(|invocation| !matches!(
                invocation.permission,
                ProcessPermission::ExplicitMutation(_)
            ))
    );

    let changed_head_process = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.50.0\n"),
        output(0, HEAD_A),
        output(0, b"cargo 1.95.0\n"),
        output(0, CARGO_METADATA_WITH_TEST_TARGET),
        output(0, b"git version 2.50.0\n"),
        output(0, HEAD_B),
        output(0, b"cargo 1.95.0\n"),
        output(0, CARGO_METADATA_WITH_TEST_TARGET),
    ]));
    let changed_runner = runner(&root, Arc::clone(&changed_head_process));
    let snapshot = block_on(changed_runner.inspect(
        TestSuiteKind::CargoTest,
        jameskills_core::ports::process::CancellationToken::new(),
    ))
    .unwrap();
    let blocked = block_on(changed_runner.run(
        TestSuiteRunApproval::after_explicit_trust_confirmation(snapshot),
        jameskills_core::ports::process::CancellationToken::new(),
    ))
    .unwrap();
    assert!(matches!(blocked.execution(), TestSuiteExecution::Blocked));
    assert!(
        changed_head_process
            .invocations
            .lock()
            .unwrap()
            .iter()
            .all(|invocation| !matches!(
                invocation.permission,
                ProcessPermission::ExplicitMutation(_)
            ))
    );
}

#[test]
fn cargo_unavailable_is_unknown_and_blocked_not_missing_or_failed() {
    let root = TestRoot::new();
    let process = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.50.0\n"),
        output(0, HEAD_A),
        output(0, b"git version 2.50.0\n"),
        output(0, HEAD_A),
    ]));
    let provider = RepositoryPolicyCheckProvider::new(
        root.approved(),
        Some(root.tool("git")),
        None,
        ApprovedEnv::new(BTreeMap::new()).unwrap(),
        process.clone(),
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    );
    let runner = TestSuiteService::new(Arc::new(provider));
    let token = CancellationToken::new();
    let snapshot = block_on(runner.inspect(TestSuiteKind::CargoTest, token.clone())).unwrap();

    assert!(matches!(
        snapshot.declaration(),
        TestSuiteDeclaration::Unknown
    ));
    let result = block_on(runner.run(
        TestSuiteRunApproval::after_explicit_trust_confirmation(snapshot),
        token,
    ))
    .unwrap();
    assert!(matches!(result.execution(), TestSuiteExecution::Blocked));
    assert_eq!(result.exit_code(), None);
    assert!(
        process
            .invocations
            .lock()
            .unwrap()
            .iter()
            .all(|invocation| !matches!(
                invocation.permission,
                ProcessPermission::ExplicitMutation(_)
            ))
    );
}

#[test]
fn cargo_head_changed_after_metadata_revalidation_blocks_spawn() {
    let root = TestRoot::new();
    let process = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.50.0\n"),
        output(0, HEAD_A),
        output(0, b"cargo 1.95.0\n"),
        output(0, CARGO_METADATA_WITH_TEST_TARGET),
        output(0, b"git version 2.50.0\n"),
        output(0, HEAD_A),
        output(0, b"cargo 1.95.0\n"),
        output(0, CARGO_METADATA_WITH_TEST_TARGET),
        output(0, HEAD_B),
    ]));
    let runner = runner(&root, Arc::clone(&process));
    let token = CancellationToken::new();
    let snapshot = block_on(runner.inspect(TestSuiteKind::CargoTest, token.clone())).unwrap();
    let result = block_on(runner.run(
        TestSuiteRunApproval::after_explicit_trust_confirmation(snapshot),
        token,
    ))
    .unwrap();

    assert!(matches!(result.execution(), TestSuiteExecution::Blocked));
    assert!(
        process
            .invocations
            .lock()
            .unwrap()
            .iter()
            .all(|invocation| !matches!(
                invocation.permission,
                ProcessPermission::ExplicitMutation(_)
            ))
    );
}

#[test]
fn cargo_nonzero_exit_is_a_failed_suite_not_a_missing_suite() {
    let root = TestRoot::new();
    let process = Arc::new(FakeProcess::new(outputs(
        HEAD_A,
        CARGO_METADATA_WITH_TEST_TARGET,
        101,
    )));
    let runner = runner(&root, Arc::clone(&process));
    let token = jameskills_core::ports::process::CancellationToken::new();
    let snapshot = block_on(runner.inspect(TestSuiteKind::CargoTest, token.clone())).unwrap();
    let result = block_on(runner.run(
        TestSuiteRunApproval::after_explicit_trust_confirmation(snapshot),
        token,
    ))
    .unwrap();

    assert!(matches!(
        result.declaration(),
        TestSuiteDeclaration::Declared
    ));
    assert!(matches!(result.execution(), TestSuiteExecution::Failed));
    assert_eq!(result.exit_code(), Some(101));
}

struct RealSuiteFixture(PathBuf);

impl RealSuiteFixture {
    fn new(failing_test: bool) -> Self {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("infra crate has a workspace root")
            .to_path_buf();
        let fixture = workspace.join("target").join(format!(
            "jameskills-suite-run-{}-{}",
            std::process::id(),
            NEXT_ROOT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(fixture.join("src")).unwrap();
        std::fs::write(
            fixture.join("Cargo.toml"),
            "[workspace]\n[package]\nname = \"jameskills-suite-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(
            fixture.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"jameskills-suite-fixture\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let assertion = if failing_test {
            "assert_eq!(1, 2);"
        } else {
            "assert_eq!(1, 1);"
        };
        std::fs::write(
            fixture.join("src/lib.rs"),
            format!("#[cfg(test)] mod tests {{ #[test] fn fixture() {{ {assertion} }} }}\n"),
        )
        .unwrap();
        Self(fixture)
    }
}

impl Drop for RealSuiteFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(&filename))
        .find_map(|path| {
            let metadata = std::fs::symlink_metadata(&path).ok()?;
            if !metadata.file_type().is_file() {
                return None;
            }
            std::fs::canonicalize(path).ok()
        })
}

fn real_tool(name: &str) -> ApprovedRepositoryTool {
    let path = find_executable(name).expect("approved tool must be available in PATH");
    ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(path.clone()).unwrap(),
        fingerprint_executable(&path).unwrap(),
    )
}

fn real_environment() -> ApprovedEnv {
    let mut environment = BTreeMap::new();
    for key in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "WINDIR",
        "LANG",
        "LC_ALL",
        "LANGUAGE",
        "TEMP",
        "TMP",
        "INCLUDE",
        "LIB",
        "LIBPATH",
        "VCINSTALLDIR",
        "VCToolsInstallDir",
        "WindowsSdkDir",
        "WindowsSDKVersion",
        "UniversalCRTSdkDir",
        "UCRTVersion",
    ] {
        if let Some(value) = std::env::var_os(key) {
            environment.insert(OsString::from(key), value);
        }
    }
    ApprovedEnv::new(environment).unwrap()
}

fn real_suite_service(root: &RealSuiteFixture) -> TestSuiteService {
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(std::fs::canonicalize(&root.0).unwrap()).unwrap(),
        Some(real_tool("git")),
        None,
        real_environment(),
        Arc::new(SystemProcessPort),
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_cargo(real_tool("cargo"));
    TestSuiteService::new(Arc::new(provider))
}

#[test]
#[ignore = "explicitly executes Cargo against app-authored temporary fixtures"]
fn real_cargo_test_driver_passes_and_fails_from_approved_fixtures() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    for (failing_test, expected_execution) in [
        (false, TestSuiteExecution::Passed),
        (true, TestSuiteExecution::Failed),
    ] {
        let fixture = RealSuiteFixture::new(failing_test);
        let service = real_suite_service(&fixture);
        let snapshot = runtime
            .block_on(service.inspect(TestSuiteKind::CargoTest, CancellationToken::new()))
            .unwrap();
        assert!(matches!(
            snapshot.declaration(),
            TestSuiteDeclaration::Declared
        ));
        let result = runtime
            .block_on(service.run(
                TestSuiteRunApproval::after_explicit_trust_confirmation(snapshot),
                CancellationToken::new(),
            ))
            .unwrap();
        assert!(
            matches!(result.execution(), actual if actual == expected_execution),
            "Cargo fixture had an unexpected execution outcome (exit code {:?})",
            result.exit_code()
        );
        assert_eq!(result.exit_code(), Some(if failing_test { 101 } else { 0 }));
    }
}
