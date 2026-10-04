use async_trait::async_trait;
use jameskills_core::{
    AppError,
    domain::{ToolId, policy::CheckStatus},
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, ProcessOutput, ProcessPort, ProcessSpec,
    },
};
use jameskills_infra::{
    fs::{ApprovedRepositoryTool, LocalFileSystem},
    process::fingerprint_executable,
};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

const NOW: &str = "2026-10-04T00:00:00Z";
const ENVIRONMENT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
static NEXT_ROOT_ID: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-commit-check-{}-{}",
            std::process::id(),
            NEXT_ROOT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("bin")).unwrap();
        Self(root)
    }

    fn tool(&self, name: &str) -> ApprovedRepositoryTool {
        let executable_name = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        };
        self.tool_named(&executable_name)
    }

    fn tool_named(&self, executable_name: &str) -> ApprovedRepositoryTool {
        let path = self.0.join("bin").join(executable_name);
        std::fs::write(&path, b"synthetic approved executable").unwrap();
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

struct Invocation {
    tool_id: ToolId,
    args: Vec<OsString>,
    cwd: PathBuf,
    edit_path: Option<PathBuf>,
    message: Option<Vec<u8>>,
    config_path: Option<PathBuf>,
    config: Option<Vec<u8>>,
    path_environment: Option<OsString>,
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
        let edit_path = spec
            .args()
            .windows(2)
            .find(|pair| pair[0] == "--edit")
            .map(|pair| PathBuf::from(&pair[1]));
        let config_path = spec
            .args()
            .windows(2)
            .find(|pair| pair[0] == "--config")
            .map(|pair| PathBuf::from(&pair[1]));
        let message = edit_path.as_ref().and_then(|path| std::fs::read(path).ok());
        let config = config_path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok());
        self.invocations.lock().unwrap().push(Invocation {
            tool_id: spec.tool_id(),
            args: spec.args().to_vec(),
            cwd: spec.cwd().path().to_path_buf(),
            edit_path,
            message,
            config_path,
            config,
            path_environment: spec.env().entries().get(&OsString::from("PATH")).cloned(),
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

fn output(exit_code: i32, stdout: &[u8]) -> ProcessOutput {
    ProcessOutput::new(Some(exit_code), stdout.to_vec(), Vec::new())
}

fn run_check(
    root: &TestRoot,
    git: &ApprovedRepositoryTool,
    commitlint: &ApprovedRepositoryTool,
    process: Arc<FakeProcess>,
) -> jameskills_core::domain::policy::CheckObservation {
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime
        .block_on(LocalFileSystem.check_conventional_commit(
            &root.approved(),
            git,
            commitlint,
            &environment,
            process.as_ref(),
            NOW,
            ENVIRONMENT,
        ))
        .unwrap()
}

#[test]
fn conventional_commit_check_uses_private_message_file_and_builtin_rules() {
    let root = TestRoot::new();
    let git = root.tool("git");
    let commitlint = root.tool("commitlint");
    let message =
        b"feat(policy-engine): verify commit messages\n\nUse the approved commitlint driver.\n";
    let process = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.55.0\n"),
        output(0, b"@commitlint/cli@21.2.2\n"),
        output(0, message),
        output(0, b""),
    ]));

    let status = run_check(&root, &git, &commitlint, Arc::clone(&process));

    assert_eq!(status.status(), CheckStatus::Pass);
    let invocations = process.invocations.lock().unwrap();
    assert_eq!(invocations.len(), 4);
    assert!(matches!(invocations[2].tool_id, ToolId::Git));
    assert_eq!(
        invocations[2].args,
        ["--no-pager", "log", "-1", "--format=%B"]
    );
    assert!(matches!(invocations[3].tool_id, ToolId::Commitlint));
    assert!(
        invocations[3]
            .args
            .iter()
            .any(|arg| arg == "--default-config")
    );
    assert_eq!(invocations[3].message.as_deref(), Some(message.as_slice()));
    let edit_path = invocations[3].edit_path.as_ref().unwrap();
    let config_path = invocations[3].config_path.as_ref().unwrap();
    assert!(!edit_path.starts_with(&root.0));
    assert!(!config_path.starts_with(&root.0));
    assert!(!invocations[3].cwd.starts_with(&root.0));
    assert!(!edit_path.exists());
    assert!(!config_path.exists());
    assert_eq!(
        invocations[3].config.as_deref(),
        Some(b"{\"rules\":{}}\n".as_slice())
    );
    let path_entries = std::env::split_paths(invocations[3].path_environment.as_ref().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        path_entries[0],
        std::fs::canonicalize(root.0.join("bin")).unwrap()
    );
}

#[test]
fn conventional_commit_failure_withholds_message_and_tool_output() {
    let root = TestRoot::new();
    let git = root.tool("git");
    let commitlint = root.tool("commitlint");
    let private_text = b"feat: synthetic secret-marker-42";
    let process = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.55.0\n"),
        output(0, b"@commitlint/cli@21.2.2\n"),
        output(0, private_text),
        output(1, b"secret-marker-42 invalid commit message"),
    ]));

    let observation = run_check(&root, &git, &commitlint, process);

    assert_eq!(observation.status(), CheckStatus::Fail);
    assert!(
        observation
            .evidence()
            .iter()
            .all(|evidence| { !evidence.summary().contains("secret-marker-42") })
    );
}

#[test]
fn conventional_commit_blocks_unreviewed_version_and_unregistered_executable() {
    let root = TestRoot::new();
    let git = root.tool("git");
    let commitlint = root.tool("commitlint");
    let unreviewed = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.55.0\n"),
        output(0, b"@commitlint/cli@21.2.3\n"),
    ]));
    let observation = run_check(&root, &git, &commitlint, Arc::clone(&unreviewed));
    assert_eq!(observation.status(), CheckStatus::Blocked);
    assert_eq!(unreviewed.invocations.lock().unwrap().len(), 2);

    let root = TestRoot::new();
    let git = root.tool("git");
    let commitlint = root.tool_named("commitlint-helper.exe");
    let blocked_process = Arc::new(FakeProcess::new(Vec::new()));
    let observation = run_check(&root, &git, &commitlint, Arc::clone(&blocked_process));
    assert_eq!(observation.status(), CheckStatus::Blocked);
    assert!(blocked_process.invocations.lock().unwrap().is_empty());
}
