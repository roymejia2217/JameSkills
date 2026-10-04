use crate::{
    AppResult, Diagnostic,
    domain::{OperationId, ToolId},
};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const MAX_ARGS: usize = 64;
const MAX_ARGUMENT_BYTES: usize = 4096;
const MAX_TOTAL_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_ENV_ENTRIES: usize = 16;
const MAX_ENV_BYTES: usize = 16 * 1024;

/// Absolute executable selected by trusted infrastructure or configuration.
/// This core value performs shape validation only; the provider verifies file
/// identity/provenance before execution.
pub struct ApprovedExecutable(PathBuf);

impl ApprovedExecutable {
    pub fn from_absolute_path(path: PathBuf) -> Result<Self, Vec<Diagnostic>> {
        if !valid_absolute_path(&path)
            || path.extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
            })
        {
            return Err(process_diagnostic(
                "process.executable.invalid",
                "Executable must be an absolute, non-shell path.",
            ));
        }
        Ok(Self(path))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// Canonical absolute working directory selected by the caller.
pub struct ApprovedRoot(PathBuf);

impl ApprovedRoot {
    pub fn from_absolute_path(path: PathBuf) -> Result<Self, Vec<Diagnostic>> {
        if !valid_absolute_path(&path) {
            return Err(process_diagnostic(
                "process.cwd.invalid",
                "Working directory must be an absolute path without parent components.",
            ));
        }
        Ok(Self(path))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// Minimal explicit environment. Credentials and arbitrary caller variables
/// are not accepted; providers start children with an empty environment.
pub struct ApprovedEnv(BTreeMap<OsString, OsString>);

impl ApprovedEnv {
    pub fn new(entries: BTreeMap<OsString, OsString>) -> Result<Self, Vec<Diagnostic>> {
        let bytes = entries
            .iter()
            .map(|(key, value)| key.len().saturating_add(value.len()))
            .sum::<usize>();
        if entries.len() > MAX_ENV_ENTRIES
            || bytes > MAX_ENV_BYTES
            || entries.iter().any(|(key, value)| {
                !approved_env_key(key)
                    || key.to_string_lossy().contains('\0')
                    || value.to_string_lossy().contains('\0')
            })
        {
            return Err(process_diagnostic(
                "process.env.invalid",
                "Process environment contains an unsupported or oversized entry.",
            ));
        }
        Ok(Self(entries))
    }

    pub fn entries(&self) -> &BTreeMap<OsString, OsString> {
        &self.0
    }
}

/// Shared cooperative cancellation flag. The process adapter observes it
/// while polling and terminates the complete child process group.
#[derive(Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessPermission {
    ReadOnlyCheck,
    ExplicitMutation(OperationId),
}

/// Approved argv invocation. It carries no shell string, and each argument
/// remains a distinct OS string all the way to the provider.
pub struct ProcessSpec {
    executable: ApprovedExecutable,
    tool_id: ToolId,
    args: Vec<OsString>,
    cwd: ApprovedRoot,
    env: ApprovedEnv,
    timeout: Duration,
    output_limit_bytes: usize,
    permission: ProcessPermission,
    cancellation: CancellationToken,
}

impl ProcessSpec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        executable: ApprovedExecutable,
        tool_id: ToolId,
        args: Vec<OsString>,
        cwd: ApprovedRoot,
        env: ApprovedEnv,
        timeout: Duration,
        output_limit_bytes: usize,
        permission: ProcessPermission,
        cancellation: CancellationToken,
    ) -> Result<Self, Vec<Diagnostic>> {
        let total_argument_bytes = args.iter().map(|arg| arg.len()).sum::<usize>();
        if args.len() > MAX_ARGS
            || total_argument_bytes > MAX_TOTAL_ARGUMENT_BYTES
            || args
                .iter()
                .any(|arg| arg.len() > MAX_ARGUMENT_BYTES || arg.to_string_lossy().contains('\0'))
            || timeout.is_zero()
            || timeout > MAX_TIMEOUT
            || output_limit_bytes == 0
            || output_limit_bytes > MAX_OUTPUT_BYTES
        {
            return Err(process_diagnostic(
                "process.spec.invalid",
                "Process arguments, timeout, or output limit are outside their bounds.",
            ));
        }
        Ok(Self {
            executable,
            tool_id,
            args,
            cwd,
            env,
            timeout,
            output_limit_bytes,
            permission,
            cancellation,
        })
    }

    pub fn executable(&self) -> &ApprovedExecutable {
        &self.executable
    }

    pub fn tool_id(&self) -> ToolId {
        self.tool_id
    }

    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    pub fn cwd(&self) -> &ApprovedRoot {
        &self.cwd
    }

    pub fn env(&self) -> &ApprovedEnv {
        &self.env
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn output_limit_bytes(&self) -> usize {
        self.output_limit_bytes
    }

    pub fn permission(&self) -> ProcessPermission {
        self.permission
    }

    pub fn cancellation(&self) -> &CancellationToken {
        &self.cancellation
    }
}

/// Bounded child result. Provider errors never carry raw stderr or command
/// output; callers decide which allowlisted stdout fields are meaningful.
pub struct ProcessOutput {
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl ProcessOutput {
    pub fn new(exit_code: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        Self {
            exit_code,
            stdout,
            stderr,
        }
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

#[async_trait::async_trait]
pub trait ProcessPort: Send + Sync {
    async fn run(&self, spec: ProcessSpec) -> AppResult<ProcessOutput>;
}

fn valid_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path.as_os_str().to_string_lossy().len() <= 32 * 1024
        && !path.as_os_str().to_string_lossy().contains('\0')
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir))
}

fn approved_env_key(key: &OsStr) -> bool {
    [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "WINDIR",
        "LANG",
        "LC_ALL",
        "LANGUAGE",
        "TMP",
        "TEMP",
    ]
    .iter()
    .any(|approved| key.to_string_lossy().eq_ignore_ascii_case(approved))
}

fn process_diagnostic(code: &'static str, message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error(code, message)]
}
