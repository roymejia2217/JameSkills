use async_trait::async_trait;
use jameskills_core::{
    AppError,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ProcessOutput, ProcessPort, ProcessSpec, RepositoryState,
    },
};
use jameskills_infra::process::{SystemProcessPort, collect_repository_facts};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::Mutex,
};

struct Invocation {
    args: Vec<OsString>,
    cwd: PathBuf,
}

struct FakeProcessPort {
    responses: Mutex<VecDeque<ProcessOutput>>,
    invocations: Mutex<Vec<Invocation>>,
}

impl FakeProcessPort {
    fn new(responses: Vec<ProcessOutput>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            invocations: Mutex::new(Vec::new()),
        }
    }
}

fn stdout(text: impl AsRef<str>, status: i32) -> ProcessOutput {
    ProcessOutput::new(Some(status), text.as_ref().as_bytes().to_vec(), vec![])
}

#[async_trait]
impl ProcessPort for FakeProcessPort {
    async fn run(&self, spec: ProcessSpec) -> Result<ProcessOutput, AppError> {
        self.invocations.lock().unwrap().push(Invocation {
            args: spec.args().to_vec(),
            cwd: spec.cwd().path().to_path_buf(),
        });
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::ExternalTool {
                tool_id: "git".to_owned(),
                exit_code: None,
            })
    }
}

fn empty_environment() -> ApprovedEnv {
    ApprovedEnv::new(BTreeMap::new()).unwrap()
}

#[test]
fn repository_facts_use_fixed_argv_and_keep_paths_out_of_arguments() {
    let root = std::env::temp_dir().join(format!(
        "jameskills repo; $(not-shell)-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join(".git")).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let git = ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap();
    let environment = empty_environment();
    let fake = FakeProcessPort::new(vec![
        stdout("git version 2.55.0\n", 0),
        stdout("true\n", 0),
        stdout("false\n", 0),
        stdout(format!("{}\n", root.display()), 0),
        stdout("main\n", 0),
        stdout(".git\n", 0),
        stdout(".git\n", 0),
        stdout("\n", 0),
    ]);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let facts = runtime
        .block_on(collect_repository_facts(&root, &git, &environment, &fake))
        .unwrap();

    assert_eq!(facts.state(), RepositoryState::Attached);
    assert_eq!(facts.root(), &root);
    assert_eq!(facts.branch(), Some("main"));
    let invocations = fake.invocations.lock().unwrap();
    assert_eq!(invocations.len(), 8);
    assert!(invocations.iter().all(|call| call.cwd == root));
    assert!(invocations.iter().all(|call| {
        call.args.iter().all(|argument| {
            !argument
                .to_string_lossy()
                .contains(root.to_string_lossy().as_ref())
        })
    }));
    assert_eq!(invocations[0].args, vec![OsString::from("--version")]);
    assert_eq!(
        invocations[1].args,
        vec![
            OsString::from("rev-parse"),
            OsString::from("--is-inside-work-tree")
        ]
    );
    assert_eq!(
        invocations[4].args,
        vec![
            OsString::from("symbolic-ref"),
            OsString::from("--quiet"),
            OsString::from("--short"),
            OsString::from("HEAD"),
        ]
    );
}

#[test]
fn repository_path_without_git_metadata_is_reported_as_not_a_repository() {
    let repo = TempRepo::new("not a repository");
    let git = ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap();
    let fake = FakeProcessPort::new(vec![
        stdout("git version 2.55.0\n", 0),
        stdout("fatal: not a git repository\n", 128),
    ]);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let facts = runtime
        .block_on(collect_repository_facts(
            repo.path(),
            &git,
            &empty_environment(),
            &fake,
        ))
        .unwrap();

    assert_eq!(facts.state(), RepositoryState::NotRepository);
    assert_eq!(facts.top_level(), None);
}

struct TempRepo {
    root: PathBuf,
}

impl TempRepo {
    fn new(name: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "jameskills {name}; $(not-shell)-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn approved_git() -> ApprovedExecutable {
    let path_variable = std::env::var_os("PATH").expect("CI provides Git on PATH");
    let executable_name = if cfg!(windows) { "git.exe" } else { "git" };
    for directory in std::env::split_paths(&path_variable) {
        let candidate = directory.join(executable_name);
        if candidate.is_file() {
            return ApprovedExecutable::from_absolute_path(
                std::fs::canonicalize(candidate).unwrap(),
            )
            .unwrap();
        }
    }
    panic!("CI must provide the Git executable");
}

fn safe_env() -> ApprovedEnv {
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

fn git_setup(git: &ApprovedExecutable, cwd: &Path, args: &[&str]) -> Output {
    let hooks_dir = cwd.join("hooks-disabled-for-test");
    std::fs::create_dir_all(&hooks_dir).unwrap();
    Command::new(git.path())
        .arg("-c")
        .arg(format!("core.hooksPath={}", hooks_dir.display()))
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(safe_env().entries())
        .output()
        .unwrap()
}

fn assert_git_setup(git: &ApprovedExecutable, cwd: &Path, args: &[&str]) {
    let output = git_setup(git, cwd, args);
    assert!(
        output.status.success(),
        "Git fixture setup failed for args {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn real_git_facts_cover_attached_detached_linked_worktree_and_submodule() {
    let git = approved_git();
    let environment = safe_env();
    let process = SystemProcessPort;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let repo = TempRepo::new("repo with spaces");

    assert_git_setup(
        &git,
        repo.path(),
        &["init", "--quiet", "--initial-branch=main"],
    );
    assert_git_setup(&git, repo.path(), &["config", "user.name", "Fixture"]);
    assert_git_setup(
        &git,
        repo.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    assert_git_setup(
        &git,
        repo.path(),
        &["commit", "--quiet", "--allow-empty", "-m", "fixture"],
    );

    let attached = runtime
        .block_on(collect_repository_facts(
            repo.path(),
            &git,
            &environment,
            &process,
        ))
        .unwrap();
    assert_eq!(attached.state(), RepositoryState::Attached);
    assert_eq!(attached.branch(), Some("main"));
    assert_eq!(attached.top_level(), Some(repo.path()));
    assert!(!attached.is_linked_worktree());
    assert!(!attached.is_submodule());
    assert!(attached.git_version().starts_with("git version "));

    assert_git_setup(
        &git,
        repo.path(),
        &["checkout", "--quiet", "--detach", "HEAD"],
    );
    let detached = runtime
        .block_on(collect_repository_facts(
            repo.path(),
            &git,
            &environment,
            &process,
        ))
        .unwrap();
    assert_eq!(detached.state(), RepositoryState::Detached);
    assert_eq!(detached.branch(), None);

    assert_git_setup(&git, repo.path(), &["checkout", "--quiet", "main"]);
    let linked_path = repo.path().join("linked worktree");
    assert_git_setup(
        &git,
        repo.path(),
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "linked",
            linked_path.to_str().unwrap(),
            "HEAD",
        ],
    );
    let linked = runtime
        .block_on(collect_repository_facts(
            &linked_path,
            &git,
            &environment,
            &process,
        ))
        .unwrap();
    assert_eq!(linked.state(), RepositoryState::Attached);
    assert_eq!(linked.branch(), Some("linked"));
    assert!(linked.is_linked_worktree());

    let subrepo = repo.path().join("child repository");
    std::fs::create_dir_all(&subrepo).unwrap();
    assert_git_setup(
        &git,
        &subrepo,
        &["init", "--quiet", "--initial-branch=main"],
    );
    assert_git_setup(&git, &subrepo, &["config", "user.name", "Fixture"]);
    assert_git_setup(
        &git,
        &subrepo,
        &["config", "user.email", "fixture@example.invalid"],
    );
    assert_git_setup(
        &git,
        &subrepo,
        &["commit", "--quiet", "--allow-empty", "-m", "submodule"],
    );
    assert_git_setup(
        &git,
        repo.path(),
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "--quiet",
            subrepo.to_str().unwrap(),
            "submodule",
        ],
    );
    let submodule = runtime
        .block_on(collect_repository_facts(
            &repo.path().join("submodule"),
            &git,
            &environment,
            &process,
        ))
        .unwrap();
    assert_eq!(submodule.state(), RepositoryState::Attached);
    assert!(submodule.is_submodule());
}
