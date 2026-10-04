use async_trait::async_trait;
use command_group::CommandGroup;
use jameskills_core::{
    AppError, AppResult,
    domain::ToolId,
    ports::process::{ProcessOutput, ProcessPermission, ProcessPort, ProcessSpec},
};
use std::{
    io::Read,
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
