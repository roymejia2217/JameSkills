use async_trait::async_trait;
use command_group::CommandGroup;
use jameskills_core::{
    AppError, AppResult,
    domain::ToolId,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken, ProcessOutput,
        ProcessPermission, ProcessPort, ProcessSpec, RepositoryFacts, RepositoryState,
    },
};
use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// OS process adapter. It uses argv only, clears inherited environment, drains
/// stdout/stderr concurrently under a shared byte budget, and kills the whole
/// process group/job on timeout, cancellation or output overflow.
pub struct SystemProcessPort;

/// Reads repository metadata using only fixed, read-only Git argv. The selected
/// path is passed as the process working directory rather than interpolated
/// into command text.
pub async fn collect_repository_facts(
    root: &Path,
    executable: &ApprovedExecutable,
    environment: &ApprovedEnv,
    process: &dyn ProcessPort,
) -> AppResult<RepositoryFacts> {
    let root = std::fs::canonicalize(root).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound
        } else {
            repository_facts_error("repository.path.unavailable")
        }
    })?;
    if !root.is_dir() {
        return Err(repository_facts_error("repository.path.not_directory"));
    }
    let approved_root =
        ApprovedRoot::from_absolute_path(root.clone()).map_err(AppError::Validation)?;
    let version = run_git(
        process,
        executable,
        &approved_root,
        environment,
        &["--version"],
    )
    .await?;
    ensure_success(&version)?;
    let git_version = read_text_line(&version)
        .filter(|line| line.starts_with("git version "))
        .ok_or_else(|| repository_facts_error("repository.git.version.invalid"))?;

    let inside = run_git(
        process,
        executable,
        &approved_root,
        environment,
        &["rev-parse", "--is-inside-work-tree"],
    )
    .await?;
    if inside.exit_code() != Some(0) {
        return Ok(RepositoryFacts::new(
            root,
            None,
            git_version,
            None,
            RepositoryState::NotRepository,
            false,
            false,
        ));
    }
    if !read_bool(&inside)? {
        let bare = run_git(
            process,
            executable,
            &approved_root,
            environment,
            &["rev-parse", "--is-bare-repository"],
        )
        .await?;
        ensure_success(&bare)?;
        let state = if read_bool(&bare)? {
            RepositoryState::Bare
        } else {
            RepositoryState::NotRepository
        };
        return Ok(RepositoryFacts::new(
            root,
            None,
            git_version,
            None,
            state,
            false,
            false,
        ));
    }

    let bare = run_git(
        process,
        executable,
        &approved_root,
        environment,
        &["rev-parse", "--is-bare-repository"],
    )
    .await?;
    ensure_success(&bare)?;
    if read_bool(&bare)? {
        return Ok(RepositoryFacts::new(
            root,
            None,
            git_version,
            None,
            RepositoryState::Bare,
            false,
            false,
        ));
    }

    let top_level_output = run_git(
        process,
        executable,
        &approved_root,
        environment,
        &["rev-parse", "--show-toplevel"],
    )
    .await?;
    ensure_success(&top_level_output)?;
    let top_level = read_text_line(&top_level_output)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| repository_facts_error("repository.path.invalid_utf8"))?;

    let branch_output = run_git(
        process,
        executable,
        &approved_root,
        environment,
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
    )
    .await?;
    let branch = match branch_output.exit_code() {
        Some(0) => Some(
            read_text_line(&branch_output)
                .filter(|line| !line.is_empty())
                .ok_or_else(|| repository_facts_error("repository.branch.invalid"))?,
        ),
        Some(1) => None,
        _ => return Err(external_error("git", branch_output.exit_code())),
    };

    let git_dir = read_git_directory(
        process,
        executable,
        &approved_root,
        environment,
        "--git-dir",
        &root,
    )
    .await?;
    let common_dir = read_git_directory(
        process,
        executable,
        &approved_root,
        environment,
        "--git-common-dir",
        &root,
    )
    .await?;
    let superproject = run_git(
        process,
        executable,
        &approved_root,
        environment,
        &["rev-parse", "--show-superproject-working-tree"],
    )
    .await?;
    ensure_success(&superproject)?;
    let is_submodule = read_text_line(&superproject).is_some_and(|line| !line.is_empty());

    Ok(RepositoryFacts::new(
        root,
        Some(top_level),
        git_version,
        branch.clone(),
        if branch.is_some() {
            RepositoryState::Attached
        } else {
            RepositoryState::Detached
        },
        git_dir != common_dir,
        is_submodule,
    ))
}

async fn run_git(
    process: &dyn ProcessPort,
    executable: &ApprovedExecutable,
    cwd: &ApprovedRoot,
    environment: &ApprovedEnv,
    args: &[&str],
) -> AppResult<ProcessOutput> {
    let executable = ApprovedExecutable::from_absolute_path(executable.path().to_path_buf())
        .map_err(AppError::Validation)?;
    let cwd =
        ApprovedRoot::from_absolute_path(cwd.path().to_path_buf()).map_err(AppError::Validation)?;
    let environment =
        ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?;
    let spec = ProcessSpec::new(
        executable,
        ToolId::Git,
        args.iter().map(OsString::from).collect(),
        cwd,
        environment,
        Duration::from_secs(5),
        16 * 1024,
        ProcessPermission::ReadOnlyCheck,
        CancellationToken::new(),
    )
    .map_err(AppError::Validation)?;
    process.run(spec).await
}

async fn read_git_directory(
    process: &dyn ProcessPort,
    executable: &ApprovedExecutable,
    cwd: &ApprovedRoot,
    environment: &ApprovedEnv,
    argument: &str,
    root: &Path,
) -> AppResult<PathBuf> {
    let output = run_git(
        process,
        executable,
        cwd,
        environment,
        &["rev-parse", argument],
    )
    .await?;
    ensure_success(&output)?;
    let value = read_text_line(&output)
        .filter(|line| !line.is_empty())
        .ok_or_else(|| repository_facts_error("repository.git.directory.invalid"))?;
    let path = Path::new(&value);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    std::fs::canonicalize(path)
        .map_err(|_| repository_facts_error("repository.git.directory.invalid"))
}

fn read_text_line(output: &ProcessOutput) -> Option<String> {
    let text = std::str::from_utf8(output.stdout()).ok()?;
    let line = text.trim_end_matches(['\r', '\n']);
    (!line.contains('\r') && !line.contains('\n') && !line.contains('\0')).then(|| line.to_owned())
}

fn read_bool(output: &ProcessOutput) -> AppResult<bool> {
    ensure_success(output)?;
    match read_text_line(output).as_deref() {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => Err(repository_facts_error("repository.git.boolean.invalid")),
    }
}

fn ensure_success(output: &ProcessOutput) -> AppResult<()> {
    if output.exit_code() == Some(0) {
        Ok(())
    } else {
        Err(external_error("git", output.exit_code()))
    }
}

fn repository_facts_error(code: &'static str) -> AppError {
    AppError::Validation(vec![jameskills_core::Diagnostic::error(
        code,
        "Repository facts could not be read safely.",
    )])
}

#[async_trait]
impl ProcessPort for SystemProcessPort {
    async fn run(&self, spec: ProcessSpec) -> AppResult<ProcessOutput> {
        let tool_id = tool_id_name(spec.tool_id()).to_owned();
        tokio::task::spawn_blocking(move || run_blocking(spec))
            .await
            .unwrap_or_else(|_| Err(external_error(&tool_id, None)))
    }
}

fn run_blocking(spec: ProcessSpec) -> AppResult<ProcessOutput> {
    let tool_id = tool_id_name(spec.tool_id()).to_owned();
    if !matches!(spec.permission(), ProcessPermission::ReadOnlyCheck) {
        return Err(AppError::PermissionDenied {
            operation: "process.mutation.not_implemented".to_owned(),
        });
    }
    if spec.cancellation().is_cancelled() {
        return Err(AppError::Cancelled);
    }
    let executable = std::fs::canonicalize(spec.executable().path())
        .map_err(|_| external_error(&tool_id, None))?;
    let executable_metadata =
        std::fs::symlink_metadata(&executable).map_err(|_| external_error(&tool_id, None))?;
    if !executable_metadata.file_type().is_file() {
        return Err(AppError::PermissionDenied {
            operation: "process.executable.not_regular".to_owned(),
        });
    }
    let cwd = std::fs::canonicalize(spec.cwd().path()).map_err(|_| AppError::PermissionDenied {
        operation: "process.cwd.unavailable".to_owned(),
    })?;
    if !cwd.is_dir() {
        return Err(AppError::PermissionDenied {
            operation: "process.cwd.not_directory".to_owned(),
        });
    }

    let mut command = Command::new(executable);
    command
        .args(spec.args())
        .current_dir(cwd)
        .env_clear()
        .envs(spec.env().entries())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .group_spawn()
        .map_err(|_| external_error(&tool_id, None))?;
    let stdout = child
        .inner()
        .stdout
        .take()
        .ok_or_else(|| external_error(&tool_id, None))?;
    let stderr = child
        .inner()
        .stderr
        .take()
        .ok_or_else(|| external_error(&tool_id, None))?;
    let budget = Arc::new(OutputBudget::new(spec.output_limit_bytes()));
    let stdout_reader = spawn_reader(stdout, Arc::clone(&budget));
    let stderr_reader = spawn_reader(stderr, Arc::clone(&budget));
    let started = Instant::now();

    let status = loop {
        if spec.cancellation().is_cancelled() {
            terminate_group(&mut child);
            join_reader(stdout_reader, &tool_id)?;
            join_reader(stderr_reader, &tool_id)?;
            return Err(AppError::Cancelled);
        }
        if budget.exceeded.load(Ordering::Acquire) {
            terminate_group(&mut child);
            join_reader(stdout_reader, &tool_id)?;
            join_reader(stderr_reader, &tool_id)?;
            return Err(external_error(&tool_id, None));
        }
        if started.elapsed() >= spec.timeout() {
            terminate_group(&mut child);
            join_reader(stdout_reader, &tool_id)?;
            join_reader(stderr_reader, &tool_id)?;
            return Err(external_error(&tool_id, None));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(PROCESS_POLL_INTERVAL),
            Err(_) => {
                terminate_group(&mut child);
                join_reader(stdout_reader, &tool_id)?;
                join_reader(stderr_reader, &tool_id)?;
                return Err(external_error(&tool_id, None));
            }
        }
    };

    let stdout = join_reader(stdout_reader, &tool_id)?;
    let stderr = join_reader(stderr_reader, &tool_id)?;
    Ok(ProcessOutput::new(status.code(), stdout, stderr))
}

struct OutputBudget {
    limit: usize,
    used: AtomicUsize,
    exceeded: AtomicBool,
}

impl OutputBudget {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            used: AtomicUsize::new(0),
            exceeded: AtomicBool::new(false),
        }
    }

    fn reserve(&self, requested: usize) -> usize {
        let mut used = self.used.load(Ordering::Acquire);
        loop {
            let accepted = requested.min(self.limit.saturating_sub(used));
            match self.used.compare_exchange_weak(
                used,
                used + accepted,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    if accepted < requested {
                        self.exceeded.store(true, Ordering::Release);
                    }
                    return accepted;
                }
                Err(observed) => used = observed,
            }
        }
    }
}

fn spawn_reader<R>(reader: R, budget: Arc<OutputBudget>) -> JoinHandle<std::io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = reader;
        let mut output = Vec::new();
        let mut buffer = [0; 8192];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                return Ok(output);
            }
            let accepted = budget.reserve(read);
            output.extend_from_slice(&buffer[..accepted]);
            if accepted < read {
                return Ok(output);
            }
        }
    })
}

fn join_reader(reader: JoinHandle<std::io::Result<Vec<u8>>>, tool_id: &str) -> AppResult<Vec<u8>> {
    reader
        .join()
        .map_err(|_| external_error(tool_id, None))?
        .map_err(|_| external_error(tool_id, None))
}

fn terminate_group(child: &mut command_group::GroupChild) {
    let _ = child.kill();
    let _ = child.wait();
}

fn tool_id_name(tool_id: ToolId) -> &'static str {
    match tool_id {
        ToolId::Git => "git",
        ToolId::Gitleaks => "gitleaks",
        ToolId::Commitlint => "commitlint",
        ToolId::Gh => "gh",
        ToolId::Cargo => "cargo",
        ToolId::Npm => "npm",
    }
}

fn external_error(tool_id: &str, exit_code: Option<i32>) -> AppError {
    AppError::ExternalTool {
        tool_id: tool_id.to_owned(),
        exit_code,
    }
}
