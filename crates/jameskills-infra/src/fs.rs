use crate::github::{GithubEvidenceDriver, GithubRepository};
use crate::platform::{
    PlatformFacts, ToolCandidate, ToolCandidateKind, ToolProfile, find_tool_candidates,
    load_tool_profiles, parse_tool_version_output, probe_registered_tool_version,
};
use jameskills_core::{
    AppError, AppResult, Diagnostic,
    application::policy::{
        PolicyCheckProvider, TestSuiteRunApproval, TestSuiteRunnerPort, TestSuiteSnapshot,
    },
    domain::{
        BundleEntry, Check, ContentHash, EntryKind, PortablePath, Requirement, ToolId,
        ValidatedInventory,
        guidance::{ToolAvailability, ToolVersionStatus},
        hash_bundle,
        policy::{
            CheckEvidence, CheckObservation, CheckStatus, Enforcement, RepositoryHead,
            TestSuiteDeclaration, TestSuiteExecution, TestSuiteKind, TestSuiteRunResult,
        },
        validate_bundle_inventory,
    },
    ports::ClockPort,
    ports::filesystem::{
        BundleFiles, FileSystemPort, bundle_entry_from_path, extract_archive_files,
        validate_archive_entries,
    },
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, ApprovedScript, CancellationToken,
        ExecutableFingerprint, ProcessPermission, ProcessPort, ProcessSpec,
    },
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

const MAX_README_BYTES: u64 = 1024 * 1024;
const MAX_GITIGNORE_PROBES: usize = 32;
const MAX_GITLEAKS_REPORT_BYTES: usize = 64 * 1024;
const MAX_COMMIT_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_COMMIT_HOOK_BYTES: usize = 16 * 1024;
const MAX_CI_WORKFLOW_BYTES: usize = 256 * 1024;
const MAX_CI_WORKFLOW_FILES: usize = 8;
const REGISTERED_GITHUB_PERMISSIONS: &[&str] = &[
    "actions",
    "artifact-metadata",
    "attestations",
    "checks",
    "code-quality",
    "contents",
    "deployments",
    "discussions",
    "id-token",
    "issues",
    "packages",
    "pages",
    "pull-requests",
    "security-events",
    "statuses",
    "vulnerability-alerts",
];
const MAX_CARGO_METADATA_BYTES: usize = 1024 * 1024;
const MAX_NPM_VERSION_BYTES: usize = 1024;
const MAX_NPM_OUTPUT_BYTES: usize = 64 * 1024;
const REVIEWED_NPM_VERSION: &str = "11.16.0";
const GITLEAKS_CONFIG_PLACEHOLDER: &str = "{APP_GITLEAKS_CONFIG}";
const GITLEAKS_DEFAULT_CONFIG: &str = "[extend]\nuseDefault = true\n";
static NEXT_GITLEAKS_CONFIG_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_COMMIT_MESSAGE_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_NPM_RUN_ID: AtomicU64 = AtomicU64::new(0);

struct PrivateCommitMessage {
    directory: PathBuf,
    path: PathBuf,
    config_path: PathBuf,
}

impl PrivateCommitMessage {
    fn create(repository_root: &Path, message: &[u8]) -> std::io::Result<Self> {
        for _ in 0..8 {
            let directory = std::env::temp_dir().join(format!(
                "jameskills-commitlint-{}-{}",
                std::process::id(),
                NEXT_COMMIT_MESSAGE_ID.fetch_add(1, Ordering::Relaxed)
            ));
            #[allow(unused_mut)]
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
            let canonical_directory = match std::fs::canonicalize(&directory) {
                Ok(path) => path,
                Err(error) => {
                    let _ = std::fs::remove_dir_all(&directory);
                    return Err(error);
                }
            };
            if canonical_directory.starts_with(repository_root)
                || repository_root.starts_with(&canonical_directory)
            {
                let _ = std::fs::remove_dir_all(&directory);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "private commit message directory overlaps the repository",
                ));
            }
            let path = canonical_directory.join("message.txt");
            if let Err(error) = write_private_file(&path, message) {
                let _ = std::fs::remove_dir_all(&canonical_directory);
                return Err(error);
            }
            let config_path = canonical_directory.join("commitlint.json");
            if let Err(error) = write_private_file(&config_path, b"{\"rules\":{}}\n") {
                let _ = std::fs::remove_dir_all(&canonical_directory);
                return Err(error);
            }
            return Ok(Self {
                directory: canonical_directory,
                path,
                config_path,
            });
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "private commit message path is unavailable",
        ))
    }
}

fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    })
}

impl Drop for PrivateCommitMessage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct PrivateGitleaksConfig {
    directory: PathBuf,
    path: PathBuf,
}

impl PrivateGitleaksConfig {
    fn create() -> std::io::Result<Self> {
        for _ in 0..8 {
            let directory = std::env::temp_dir().join(format!(
                "jameskills-gitleaks-{}-{}",
                std::process::id(),
                NEXT_GITLEAKS_CONFIG_ID.fetch_add(1, Ordering::Relaxed)
            ));
            #[allow(unused_mut)]
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
            let path = directory.join("gitleaks.toml");
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let write_result = options.open(&path).and_then(|mut file| {
                file.write_all(GITLEAKS_DEFAULT_CONFIG.as_bytes())?;
                file.sync_all()
            });
            if let Err(error) = write_result {
                let _ = std::fs::remove_dir_all(&directory);
                return Err(error);
            }
            return Ok(Self { directory, path });
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "private Gitleaks config path is unavailable",
        ))
    }
}

impl Drop for PrivateGitleaksConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct PrivateNpmRunConfig {
    directory: PathBuf,
    user_config: PathBuf,
    global_config: PathBuf,
    cache: PathBuf,
    logs: PathBuf,
}

impl PrivateNpmRunConfig {
    fn create(repository_root: &Path) -> std::io::Result<Self> {
        for _ in 0..8 {
            let directory = std::env::temp_dir().join(format!(
                "jameskills-npm-run-{}-{}",
                std::process::id(),
                NEXT_NPM_RUN_ID.fetch_add(1, Ordering::Relaxed)
            ));
            #[allow(unused_mut)]
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
            let canonical_directory = match std::fs::canonicalize(&directory) {
                Ok(path) => path,
                Err(error) => {
                    let _ = std::fs::remove_dir_all(&directory);
                    return Err(error);
                }
            };
            if canonical_directory.starts_with(repository_root)
                || repository_root.starts_with(&canonical_directory)
            {
                let _ = std::fs::remove_dir_all(&canonical_directory);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "private npm configuration overlaps the repository",
                ));
            }
            let user_config = canonical_directory.join("user.npmrc");
            let global_config = canonical_directory.join("global.npmrc");
            let cache = canonical_directory.join("cache");
            let logs = canonical_directory.join("logs");
            for file in [&user_config, &global_config] {
                if let Err(error) = write_private_file(file, b"") {
                    let _ = std::fs::remove_dir_all(&canonical_directory);
                    return Err(error);
                }
            }
            for subdirectory in [&cache, &logs] {
                if let Err(error) = std::fs::create_dir(subdirectory) {
                    let _ = std::fs::remove_dir_all(&canonical_directory);
                    return Err(error);
                }
            }
            return Ok(Self {
                directory: canonical_directory,
                user_config,
                global_config,
                cache,
                logs,
            });
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "private npm configuration path is unavailable",
        ))
    }
}

impl Drop for PrivateNpmRunConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn repository_has_gitleaks_ignore(root: &ApprovedRoot) -> std::io::Result<bool> {
    match std::fs::symlink_metadata(root.path().join(".gitleaksignore")) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn candidate_for_approved_tool(
    profile: &ToolProfile,
    tool: &ApprovedRepositoryTool,
    platform: crate::platform::HostPlatform,
) -> Option<ToolCandidate> {
    let search_path = tool.executable.path().parent()?.to_path_buf();
    let approved_path = std::fs::canonicalize(tool.executable.path()).ok()?;
    find_tool_candidates(std::slice::from_ref(profile), &[search_path], platform)
        .into_iter()
        .find(|candidate| {
            candidate
                .path()
                .and_then(|path| std::fs::canonicalize(path).ok())
                .is_some_and(|path| path == approved_path)
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitleaksReportStatus {
    NoFindings,
    Findings,
    Unknown,
}

/// Parses only the bounded report shape and returns no report fields, which may
/// contain secret material even when the driver requested redaction.
pub fn parse_gitleaks_report(output: &[u8]) -> GitleaksReportStatus {
    if output.is_empty() || output.len() > MAX_GITLEAKS_REPORT_BYTES || output.contains(&0) {
        return GitleaksReportStatus::Unknown;
    }
    let Ok(report) = serde_json::from_slice::<serde_json::Value>(output) else {
        return GitleaksReportStatus::Unknown;
    };
    let Some(findings) = report.as_array() else {
        return GitleaksReportStatus::Unknown;
    };
    let valid_finding = findings.iter().all(|finding| {
        let Some(finding) = finding.as_object() else {
            return false;
        };
        finding
            .get("RuleID")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty())
            && finding
                .get("File")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.is_empty())
            && finding
                .get("StartLine")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|value| value > 0)
            && finding
                .get("Secret")
                .and_then(serde_json::Value::as_str)
                .is_some()
            && finding
                .get("Fingerprint")
                .and_then(serde_json::Value::as_str)
                .is_some()
    });
    if !valid_finding {
        GitleaksReportStatus::Unknown
    } else if findings.is_empty() {
        GitleaksReportStatus::NoFindings
    } else {
        GitleaksReportStatus::Findings
    }
}

/// Local filesystem adapter: validation, explicit staging, and content-addressed
/// blob IO live here so core never touches the disk. Writes follow validation.
pub struct LocalFileSystem;

impl LocalFileSystem {
    pub fn inspect_bundle(&self, root: &Path) -> Result<ValidatedInventory, Vec<Diagnostic>> {
        validate_bundle_inventory(&inspect_bundle_tree(root)?)
    }

    pub fn check_readme_sections(
        &self,
        root: &ApprovedRoot,
        path: &PortablePath,
        required_headings: &[String],
        observed_at: &str,
        environment_fingerprint: &str,
    ) -> AppResult<CheckObservation> {
        let evidence = CheckEvidence::new(
            "repo.readme",
            observed_at,
            None,
            environment_fingerprint,
            "Required README sections were checked.",
            None,
        )
        .map_err(AppError::Validation)?;
        let root_path = std::fs::canonicalize(root.path()).map_err(|_| AppError::NotFound)?;
        if !root_path.is_dir() {
            return Err(AppError::NotFound);
        }
        let status = match read_repository_document(&root_path, path) {
            ReadmeFile::Missing => CheckStatus::Fail,
            ReadmeFile::Blocked => CheckStatus::Blocked,
            ReadmeFile::Bytes(bytes) => match std::str::from_utf8(&bytes) {
                Ok(source) if readme_has_required_sections(source, required_headings) => {
                    CheckStatus::Pass
                }
                Ok(_) | Err(_) => CheckStatus::Fail,
            },
        };
        CheckObservation::new(
            status,
            Some(jameskills_core::domain::policy::Enforcement::LocalCheck),
            vec![evidence],
        )
        .map_err(AppError::Validation)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn check_gitignore_patterns(
        &self,
        root: &ApprovedRoot,
        path: &PortablePath,
        patterns: &[String],
        git_driver: Option<(&ApprovedExecutable, ExecutableFingerprint)>,
        environment: &ApprovedEnv,
        process: &dyn ProcessPort,
        observed_at: &str,
        environment_fingerprint: &str,
    ) -> AppResult<CheckObservation> {
        let make_evidence = |source_id: &str, summary| {
            CheckEvidence::new(
                source_id,
                observed_at,
                None,
                environment_fingerprint,
                summary,
                None,
            )
            .map_err(AppError::Validation)
        };
        if path.as_str() != ".gitignore" {
            return observation(
                CheckStatus::Unsupported,
                None,
                vec![make_evidence(
                    "repo.gitignore",
                    "Only the root .gitignore check is registered.",
                )?],
            );
        }
        if patterns.is_empty() || patterns.len() > MAX_GITIGNORE_PROBES {
            return observation(
                CheckStatus::Unsupported,
                None,
                vec![make_evidence(
                    "repo.gitignore",
                    "Declared ignore patterns exceed the registered probe set.",
                )?],
            );
        }
        let mut probes = Vec::with_capacity(patterns.len());
        for pattern in patterns {
            let Some((synthetic_path, evidence_source)) = synthetic_ignore_path(pattern) else {
                return observation(
                    CheckStatus::Unsupported,
                    None,
                    vec![make_evidence(
                        "repo.gitignore",
                        "An ignore pattern has no registered synthetic probe.",
                    )?],
                );
            };
            probes.push((pattern.as_str(), synthetic_path, evidence_source));
        }
        let root_path = std::fs::canonicalize(root.path()).map_err(|_| AppError::NotFound)?;
        if !root_path.is_dir() {
            return Err(AppError::NotFound);
        }
        match read_repository_document(&root_path, path) {
            ReadmeFile::Missing => {
                return observation(
                    CheckStatus::Fail,
                    Some(Enforcement::LocalCheck),
                    vec![make_evidence(
                        "repo.gitignore",
                        "The declared .gitignore file is missing.",
                    )?],
                );
            }
            ReadmeFile::Blocked => {
                return observation(
                    CheckStatus::Blocked,
                    None,
                    vec![make_evidence(
                        "repo.gitignore",
                        "The declared .gitignore file is not safely readable.",
                    )?],
                );
            }
            ReadmeFile::Bytes(_) => {}
        }
        let Some((executable, fingerprint)) = git_driver else {
            return observation(
                CheckStatus::Blocked,
                None,
                vec![make_evidence(
                    "repo.gitignore",
                    "Git executable fingerprint approval is unavailable.",
                )?],
            );
        };

        let executable = ApprovedExecutable::from_absolute_path(executable.path().to_path_buf())
            .map_err(AppError::Validation)?;
        let cwd = ApprovedRoot::from_absolute_path(root_path).map_err(AppError::Validation)?;
        let environment =
            ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?;
        let mut evidence = Vec::with_capacity(probes.len());
        let mut all_matched = true;
        for (pattern, synthetic_path, evidence_source) in probes {
            let args = [
                "check-ignore",
                "--no-index",
                "-v",
                "-z",
                "--",
                synthetic_path,
            ]
            .into_iter()
            .map(OsString::from)
            .collect();
            let spec = ProcessSpec::new(
                ApprovedExecutable::from_absolute_path(executable.path().to_path_buf())
                    .map_err(AppError::Validation)?,
                ToolId::Git,
                args,
                ApprovedRoot::from_absolute_path(cwd.path().to_path_buf())
                    .map_err(AppError::Validation)?,
                ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?,
                Duration::from_secs(5),
                4096,
                ProcessPermission::ReadOnlyCheck,
                CancellationToken::new(),
            )
            .map_err(AppError::Validation)?
            .with_approved_executable_fingerprint(fingerprint);
            let output = match process.run(spec).await {
                Ok(output) => output,
                Err(AppError::PermissionDenied { .. } | AppError::ExternalTool { .. }) => {
                    return observation(
                        CheckStatus::Blocked,
                        None,
                        vec![make_evidence(
                            "repo.gitignore",
                            "Git process or approved identity is unavailable.",
                        )?],
                    );
                }
                Err(error) => return Err(error),
            };
            let matched = match output.exit_code() {
                Some(0) => {
                    git_ignore_match(output.stdout(), pattern, synthetic_path, path.as_str())
                }
                Some(1) => false,
                _ => {
                    return observation(
                        CheckStatus::Blocked,
                        None,
                        vec![make_evidence(
                            "repo.gitignore",
                            "Git returned an unrecognized gitignore exit status.",
                        )?],
                    );
                }
            };
            all_matched &= matched;
            evidence.push(make_evidence(
                evidence_source,
                if matched {
                    "A registered gitignore pattern matched its synthetic path."
                } else {
                    "A registered gitignore pattern did not match its synthetic path."
                },
            )?);
        }
        observation(
            if all_matched {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            },
            Some(Enforcement::LocalCheck),
            evidence,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn check_conventional_commit(
        &self,
        root: &ApprovedRoot,
        git: Option<&ApprovedRepositoryTool>,
        commitlint: Option<&ApprovedCommitlint>,
        environment: &ApprovedEnv,
        process: &dyn ProcessPort,
        observed_at: &str,
        environment_fingerprint: &str,
    ) -> AppResult<CheckObservation> {
        let report = |status, enforcement, summary| {
            let evidence = CheckEvidence::new(
                "tool.commitlint.lint",
                observed_at,
                None,
                environment_fingerprint,
                summary,
                None,
            )
            .map_err(AppError::Validation)?;
            CheckObservation::new(status, enforcement, vec![evidence]).map_err(AppError::Validation)
        };
        let root_path = std::fs::canonicalize(root.path()).map_err(|_| AppError::NotFound)?;
        if !root_path.is_dir() {
            return Err(AppError::NotFound);
        }
        let (Some(git), Some(commitlint)) = (git, commitlint) else {
            return report(
                CheckStatus::Blocked,
                None,
                "Approved Git and Commitlint execution identities are required.",
            );
        };
        let profiles = load_tool_profiles().map_err(AppError::Validation)?;
        let Some(git_profile) = profiles
            .iter()
            .find(|profile| profile.tool_id() == ToolId::Git)
        else {
            return report(
                CheckStatus::Blocked,
                None,
                "The app-owned Git profile is unavailable.",
            );
        };
        let Some(commitlint_profile) = profiles
            .iter()
            .find(|profile| profile.tool_id() == ToolId::Commitlint)
        else {
            return report(
                CheckStatus::Blocked,
                None,
                "The app-owned Commitlint profile is unavailable.",
            );
        };
        let platform = PlatformFacts::detect().platform;
        let Some(git_candidate) = candidate_for_approved_tool(git_profile, git, platform) else {
            return report(
                CheckStatus::Blocked,
                None,
                "The approved Git executable is not a registered candidate.",
            );
        };
        if git_candidate.kind() != ToolCandidateKind::NativeExecutable {
            return report(
                CheckStatus::Blocked,
                None,
                "Git command shims are not executed by this driver.",
            );
        }
        let environment =
            ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?;
        let commitlint_route = match commitlint {
            ApprovedCommitlint::Native(tool) => {
                let Some(candidate) =
                    candidate_for_approved_tool(commitlint_profile, tool, platform)
                else {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "The approved Commitlint executable is not registered.",
                    );
                };
                if candidate.kind() != ToolCandidateKind::NativeExecutable {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "Commitlint command shims are not executed by this driver.",
                    );
                }
                CommitlintRoute::Native { tool, candidate }
            }
            ApprovedCommitlint::Node(driver) => {
                let Some(node_profile) = profiles
                    .iter()
                    .find(|profile| profile.tool_id() == ToolId::Node)
                else {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "The app-owned Node profile is unavailable.",
                    );
                };
                let Some(candidate) =
                    candidate_for_approved_tool(node_profile, &driver.node, platform)
                else {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "The approved Node executable is not registered.",
                    );
                };
                if candidate.kind() != ToolCandidateKind::NativeExecutable
                    || !is_commitlint_cli_entrypoint(driver.entrypoint.path())
                {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "Node or the approved Commitlint CLI entrypoint is not supported.",
                    );
                }
                CommitlintRoute::Node {
                    driver,
                    profile: node_profile,
                    candidate,
                }
            }
        };
        let git_version = probe_registered_tool_version(
            git_profile,
            &git_candidate,
            Some(git.fingerprint),
            root,
            &environment,
            process,
            observed_at,
        )
        .await?;
        if git_version.availability() != ToolAvailability::Candidate
            || git_version.version_status() != ToolVersionStatus::Compatible
            || git_version.version().is_none()
        {
            return report(
                CheckStatus::Blocked,
                None,
                "Git version is outside the verified app-owned profile.",
            );
        }
        match &commitlint_route {
            CommitlintRoute::Native { tool, candidate } => {
                let version = probe_registered_tool_version(
                    commitlint_profile,
                    candidate,
                    Some(tool.fingerprint),
                    root,
                    &environment,
                    process,
                    observed_at,
                )
                .await?;
                if version.availability() != ToolAvailability::Candidate
                    || version.version_status() != ToolVersionStatus::Compatible
                    || version.version().is_none()
                {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "Commitlint version is outside the verified app-owned profile.",
                    );
                }
            }
            CommitlintRoute::Node {
                driver,
                profile: node_profile,
                candidate: node_candidate,
            } => {
                let node_version = probe_registered_tool_version(
                    node_profile,
                    node_candidate,
                    Some(driver.node.fingerprint),
                    root,
                    &environment,
                    process,
                    observed_at,
                )
                .await?;
                let minimum_node = semver::Version::new(22, 12, 0);
                if node_version.availability() != ToolAvailability::Candidate
                    || node_version.version_status() != ToolVersionStatus::Compatible
                    || node_version
                        .version()
                        .is_none_or(|version| version < &minimum_node)
                {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "Commitlint v21.2.2 requires approved Node >=22.12.0.",
                    );
                }
                let version_output = match run_node_commitlint(
                    driver,
                    &[OsString::from("--version")],
                    root,
                    &environment,
                    process,
                )
                .await
                {
                    Ok(output) => output,
                    Err(AppError::Cancelled) => return Err(AppError::Cancelled),
                    Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                        return report(
                            CheckStatus::Blocked,
                            None,
                            "Commitlint CLI version could not be verified through Node.",
                        );
                    }
                    Err(error) => return Err(error),
                };
                let version =
                    parse_tool_version_output(commitlint_profile, version_output.stdout());
                if version_output.exit_code() != Some(0) {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "Node could not query the Commitlint CLI version successfully.",
                    );
                }
                let Some(version) = version else {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "Commitlint CLI version output did not match its registered format.",
                    );
                };
                if !commitlint_profile.version_range().matches(&version) {
                    return report(
                        CheckStatus::Blocked,
                        None,
                        "Commitlint CLI is not the reviewed 21.2.2 package.",
                    );
                }
            }
        }

        let git_spec = ProcessSpec::new(
            ApprovedExecutable::from_absolute_path(git.executable.path().to_path_buf())
                .map_err(AppError::Validation)?,
            ToolId::Git,
            ["--no-pager", "log", "-1", "--format=%B"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            ApprovedRoot::from_absolute_path(root_path.clone()).map_err(AppError::Validation)?,
            ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?,
            Duration::from_secs(5),
            MAX_COMMIT_MESSAGE_BYTES,
            ProcessPermission::ReadOnlyCheck,
            CancellationToken::new(),
        )
        .map_err(AppError::Validation)?
        .with_approved_executable_fingerprint(git.fingerprint);
        let message = match process.run(git_spec).await {
            Ok(output) if output.exit_code() == Some(0) => output.stdout().to_vec(),
            Ok(_) | Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return report(
                    CheckStatus::Blocked,
                    None,
                    "Git could not safely provide the current commit message.",
                );
            }
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(error) => return Err(error),
        };
        if message.is_empty()
            || message.len() > MAX_COMMIT_MESSAGE_BYTES
            || message.contains(&0)
            || std::str::from_utf8(&message).is_err()
        {
            return report(
                CheckStatus::Unknown,
                None,
                "The current commit message is absent or outside safe UTF-8 limits.",
            );
        }
        let message_file = match PrivateCommitMessage::create(&root_path, &message) {
            Ok(message_file) => message_file,
            Err(_) => {
                return report(
                    CheckStatus::Blocked,
                    None,
                    "A private commit-message file could not be staged outside the repository.",
                );
            }
        };
        let observed_git_fingerprint =
            crate::process::fingerprint_executable(git.executable.path());
        if !matches!(observed_git_fingerprint, Ok(fingerprint) if fingerprint == git.fingerprint) {
            return report(
                CheckStatus::Blocked,
                None,
                "The approved Git executable changed before Commitlint ran.",
            );
        }
        let mut commitlint_environment_entries = environment.entries().clone();
        let git_directory = git.executable.path().parent().ok_or_else(|| {
            AppError::Validation(vec![Diagnostic::error(
                "tool.git.path.invalid",
                "Approved Git executable has no parent directory.",
            )])
        })?;
        let mut search_paths = vec![git_directory.to_path_buf()];
        if let Some(existing_path) = commitlint_environment_entries.get(&OsString::from("PATH")) {
            search_paths.extend(std::env::split_paths(existing_path));
        }
        let path = std::env::join_paths(search_paths).map_err(|_| {
            AppError::Validation(vec![Diagnostic::error(
                "process.env.path.invalid",
                "Approved executable search path could not be constructed.",
            )])
        })?;
        commitlint_environment_entries.insert(OsString::from("PATH"), path);
        let commitlint_environment =
            ApprovedEnv::new(commitlint_environment_entries).map_err(AppError::Validation)?;
        let mut commitlint_args = match &commitlint_route {
            CommitlintRoute::Native { .. } => Vec::new(),
            CommitlintRoute::Node { driver, .. } => {
                vec![
                    node_path_argument(driver.entrypoint.path()),
                    OsString::from("--cwd"),
                    node_path_argument(&root_path),
                ]
            }
        };
        commitlint_args.extend(
            ["--default-config", "--config"]
                .into_iter()
                .map(OsString::from),
        );
        let config_argument = if matches!(&commitlint_route, CommitlintRoute::Node { .. }) {
            node_path_argument(&message_file.config_path)
        } else {
            message_file.config_path.as_os_str().to_os_string()
        };
        commitlint_args.push(config_argument);
        commitlint_args.push(OsString::from("--edit"));
        let message_argument = if matches!(&commitlint_route, CommitlintRoute::Node { .. }) {
            node_path_argument(&message_file.path)
        } else {
            message_file.path.as_os_str().to_os_string()
        };
        commitlint_args.push(message_argument);
        commitlint_args.extend([OsString::from("--quiet"), OsString::from("--color=false")]);
        let (executable, executable_fingerprint) = match &commitlint_route {
            CommitlintRoute::Native { tool, .. } => (
                ApprovedExecutable::from_absolute_path(tool.executable.path().to_path_buf())
                    .map_err(AppError::Validation)?,
                tool.fingerprint,
            ),
            CommitlintRoute::Node { driver, .. } => (
                ApprovedExecutable::from_absolute_path(driver.node.executable.path().to_path_buf())
                    .map_err(AppError::Validation)?,
                driver.node.fingerprint,
            ),
        };
        let commitlint_spec = ProcessSpec::new(
            executable,
            ToolId::Commitlint,
            commitlint_args,
            ApprovedRoot::from_absolute_path(message_file.directory.clone())
                .map_err(AppError::Validation)?,
            commitlint_environment,
            Duration::from_secs(10),
            16 * 1024,
            ProcessPermission::ReadOnlyCheck,
            CancellationToken::new(),
        )
        .map_err(AppError::Validation)?
        .with_approved_executable_fingerprint(executable_fingerprint);
        let commitlint_spec = match &commitlint_route {
            CommitlintRoute::Native { .. } => commitlint_spec,
            CommitlintRoute::Node { driver, .. } => commitlint_spec.with_approved_script(
                ApprovedScript::from_absolute_path(driver.entrypoint.path().to_path_buf())
                    .map_err(AppError::Validation)?,
                driver.entrypoint_fingerprint,
            ),
        };
        let output = match process.run(commitlint_spec).await {
            Ok(output) => output,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return report(
                    CheckStatus::Blocked,
                    None,
                    "Commitlint could not run with the approved executable identity.",
                );
            }
            Err(error) => return Err(error),
        };
        match output.exit_code() {
            Some(exit_code @ (0 | 1)) => {
                let status = if exit_code == 0 {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Fail
                };
                let message_summary = if exit_code == 0 {
                    "Commitlint 21.2.2 accepted the current commit using built-in Conventional Commits rules."
                } else {
                    "Commitlint 21.2.2 rejected the current commit; message details are withheld."
                };
                let hook_summary =
                    inspect_effective_commit_hook(&root_path, git, &environment, process).await?;
                let message_evidence = CheckEvidence::new(
                    "tool.commitlint.lint",
                    observed_at,
                    None,
                    environment_fingerprint,
                    message_summary,
                    None,
                )
                .map_err(AppError::Validation)?;
                let hook_evidence = CheckEvidence::new(
                    "repo.commit-hook",
                    observed_at,
                    None,
                    environment_fingerprint,
                    hook_summary,
                    None,
                )
                .map_err(AppError::Validation)?;
                CheckObservation::new(
                    status,
                    Some(Enforcement::LocalCheck),
                    vec![message_evidence, hook_evidence],
                )
                .map_err(AppError::Validation)
            }
            _ => report(
                CheckStatus::Blocked,
                None,
                "Commitlint returned an unregistered exit status; output is withheld.",
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn check_ci_contract(
        &self,
        root: &ApprovedRoot,
        workflow_paths: &[PortablePath],
        required_jobs: &[String],
        git: Option<&ApprovedRepositoryTool>,
        environment: &ApprovedEnv,
        process: &dyn ProcessPort,
        observed_at: &str,
        environment_fingerprint: &str,
    ) -> AppResult<CheckObservation> {
        let report = |status, enforcement, summary| {
            let evidence = CheckEvidence::new(
                "repo.ci-contract",
                observed_at,
                None,
                environment_fingerprint,
                summary,
                None,
            )
            .map_err(AppError::Validation)?;
            CheckObservation::new(status, enforcement, vec![evidence]).map_err(AppError::Validation)
        };
        if workflow_paths.is_empty()
            || workflow_paths.len() > MAX_CI_WORKFLOW_FILES
            || required_jobs.is_empty()
            || required_jobs.len() > 64
        {
            return report(
                CheckStatus::Unsupported,
                None,
                "The registered CI contract is outside supported workflow/job limits.",
            );
        }
        if workflow_paths.iter().any(|path| {
            let value = path.as_str();
            !value.starts_with(".github/workflows/")
                || !(value.ends_with(".yml") || value.ends_with(".yaml"))
        }) {
            return report(
                CheckStatus::Unsupported,
                None,
                "Only GitHub Actions workflow YAML paths are supported by this check.",
            );
        }

        let repository_root = std::fs::canonicalize(root.path()).map_err(|_| AppError::NotFound)?;
        match observe_github_remote_host(&repository_root, git, environment, process, observed_at)
            .await?
        {
            Ok(_) => {}
            Err(summary) => return report(CheckStatus::Unknown, None, summary),
        }

        let mut push_jobs = BTreeSet::new();
        let mut pull_request_jobs = BTreeSet::new();
        for path in workflow_paths {
            let bytes = match read_repository_document(&repository_root, path) {
                ReadmeFile::Missing => {
                    return report(
                        CheckStatus::Fail,
                        Some(Enforcement::LocalCheck),
                        "A workflow declared by the CI contract is missing.",
                    );
                }
                ReadmeFile::Blocked => {
                    return report(
                        CheckStatus::Unknown,
                        None,
                        "A declared CI workflow is not a bounded regular repository file.",
                    );
                }
                ReadmeFile::Bytes(bytes) => bytes,
            };
            if bytes.len() > MAX_CI_WORKFLOW_BYTES {
                return report(
                    CheckStatus::Unknown,
                    None,
                    "A CI workflow exceeds the parser inspection limit.",
                );
            }
            let Ok(source) = std::str::from_utf8(&bytes) else {
                return report(
                    CheckStatus::Unknown,
                    None,
                    "A CI workflow is not valid UTF-8 YAML.",
                );
            };
            let facts = match inspect_github_workflow(source, required_jobs) {
                Ok(facts) => facts,
                Err(CiWorkflowIssue::Fail(summary)) => {
                    return report(CheckStatus::Fail, Some(Enforcement::LocalCheck), summary);
                }
                Err(CiWorkflowIssue::Unknown(summary)) => {
                    return report(CheckStatus::Unknown, None, summary);
                }
            };
            if facts.push {
                push_jobs.extend(facts.job_ids.iter().cloned());
            }
            if facts.pull_request {
                pull_request_jobs.extend(facts.job_ids);
            }
        }
        if push_jobs.is_empty() || pull_request_jobs.is_empty() {
            return report(
                CheckStatus::Fail,
                Some(Enforcement::LocalCheck),
                "Declared CI workflows do not run jobs on both push and pull_request events.",
            );
        }
        if required_jobs
            .iter()
            .any(|job| !push_jobs.contains(job) || !pull_request_jobs.contains(job))
        {
            return report(
                CheckStatus::Fail,
                Some(Enforcement::LocalCheck),
                "A required CI job is missing from push or pull_request workflow coverage.",
            );
        }
        report(
            CheckStatus::Pass,
            Some(Enforcement::LocalCheck),
            "The declared GitHub Actions workflow contract is valid locally; host enforcement and current-SHA CI are not established.",
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn check_tracked_secrets(
        &self,
        root: &ApprovedRoot,
        profile: &ToolProfile,
        candidate: &ToolCandidate,
        approved_fingerprint: Option<ExecutableFingerprint>,
        include_history: bool,
        environment: &ApprovedEnv,
        process: &dyn ProcessPort,
        observed_at: &str,
        environment_fingerprint: &str,
    ) -> AppResult<CheckObservation> {
        let report = |status, enforcement, summary| {
            let evidence = CheckEvidence::new(
                "tool.gitleaks.scan",
                observed_at,
                None,
                environment_fingerprint,
                summary,
                None,
            )
            .map_err(AppError::Validation)?;
            CheckObservation::new(status, enforcement, vec![evidence]).map_err(AppError::Validation)
        };
        if profile.tool_id() != ToolId::Gitleaks || candidate.tool_id() != ToolId::Gitleaks {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "tool.profile.mismatch",
                "Gitleaks profile and candidate identifiers do not match.",
            )]));
        }
        if include_history {
            return report(
                CheckStatus::Unsupported,
                None,
                "Gitleaks profile supports current-tree scanning only; history is Unsupported without approved nested Git.",
            );
        }
        match repository_has_gitleaks_ignore(root) {
            Ok(false) => {}
            Ok(true) => {
                return report(
                    CheckStatus::Blocked,
                    None,
                    "Repository .gitleaksignore is unsupported because Gitleaks applies it during scanning.",
                );
            }
            Err(_) => {
                return report(
                    CheckStatus::Blocked,
                    None,
                    "Repository .gitleaksignore could not be checked safely; Gitleaks was not run.",
                );
            }
        }
        let Some(scan) = profile.scan_probe() else {
            return report(
                CheckStatus::Blocked,
                None,
                "Gitleaks scan profile is unavailable; only version 8.30.1 is verified.",
            );
        };
        if candidate.kind() != ToolCandidateKind::NativeExecutable {
            return report(
                CheckStatus::Blocked,
                None,
                "A native Gitleaks 8.30.1 candidate is required.",
            );
        }
        let Some(fingerprint) = approved_fingerprint else {
            return report(
                CheckStatus::Blocked,
                None,
                "Gitleaks 8.30.1 requires an approved executable fingerprint.",
            );
        };
        let version = probe_registered_tool_version(
            profile,
            candidate,
            Some(fingerprint),
            root,
            environment,
            process,
            observed_at,
        )
        .await?;
        if version.availability() != ToolAvailability::Candidate
            || version.version_status() != ToolVersionStatus::Compatible
            || version.version().is_none()
        {
            return report(
                CheckStatus::Blocked,
                None,
                "Gitleaks version is not 8.30.1 or identity verification failed.",
            );
        }

        let Some(path) = candidate.path() else {
            return report(
                CheckStatus::Blocked,
                None,
                "Gitleaks compatible candidate has no executable path.",
            );
        };
        let executable = ApprovedExecutable::from_absolute_path(path.to_path_buf())
            .map_err(AppError::Validation)?;
        let environment =
            ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?;
        let cwd = ApprovedRoot::from_absolute_path(root.path().to_path_buf())
            .map_err(AppError::Validation)?;
        let config = match PrivateGitleaksConfig::create() {
            Ok(config) => config,
            Err(_) => {
                return report(
                    CheckStatus::Blocked,
                    None,
                    "A private app-owned Gitleaks config could not be staged.",
                );
            }
        };
        let args = scan
            .args()
            .iter()
            .map(|argument| {
                if argument == GITLEAKS_CONFIG_PLACEHOLDER {
                    config.path.as_os_str().to_os_string()
                } else {
                    OsString::from(argument)
                }
            })
            .collect();
        let spec = ProcessSpec::new(
            executable,
            ToolId::Gitleaks,
            args,
            cwd,
            environment,
            Duration::from_secs(60),
            scan.output_limit_bytes(),
            ProcessPermission::ReadOnlyCheck,
            CancellationToken::new(),
        )
        .map_err(AppError::Validation)?
        .with_approved_executable_fingerprint(fingerprint);
        let output = match process.run(spec).await {
            Ok(output) => output,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return report(
                    CheckStatus::Blocked,
                    None,
                    "Gitleaks process failed or executable identity changed; output cap is 65536 bytes.",
                );
            }
            Err(error) => return Err(error),
        };
        let report_status = parse_gitleaks_report(output.stdout());
        let (status, summary) = match (output.exit_code(), report_status) {
            (Some(code), GitleaksReportStatus::NoFindings) if code == scan.clean_exit_code() => (
                CheckStatus::Pass,
                "Gitleaks 8.30.1 checked current tree; history excluded; JSON cap 65536 bytes.",
            ),
            (Some(code), GitleaksReportStatus::Findings) if code == scan.findings_exit_code() => (
                CheckStatus::Fail,
                "Gitleaks 8.30.1 found current-tree findings; JSON cap 65536 bytes; redacted details withheld.",
            ),
            (_, GitleaksReportStatus::Unknown)
            | (Some(_), GitleaksReportStatus::Findings | GitleaksReportStatus::NoFindings) => (
                CheckStatus::Unknown,
                "Gitleaks 8.30.1 report unknown; current-tree JSON cap 65536 bytes; history excluded.",
            ),
            _ => (
                CheckStatus::Blocked,
                "Gitleaks returned an unregistered exit status; output cap is 65536 bytes.",
            ),
        };
        report(status, Some(Enforcement::LocalCheck), summary)
    }
}

pub struct ApprovedRepositoryTool {
    executable: ApprovedExecutable,
    fingerprint: ExecutableFingerprint,
}

impl ApprovedRepositoryTool {
    pub fn new(executable: ApprovedExecutable, fingerprint: ExecutableFingerprint) -> Self {
        Self {
            executable,
            fingerprint,
        }
    }

    pub(crate) fn executable_path(&self) -> &Path {
        self.executable.path()
    }

    pub(crate) fn fingerprint(&self) -> ExecutableFingerprint {
        self.fingerprint
    }
}

pub struct ApprovedCommitlintNode {
    node: ApprovedRepositoryTool,
    entrypoint: ApprovedScript,
    entrypoint_fingerprint: ExecutableFingerprint,
}

pub struct ApprovedNodeNpm {
    node: ApprovedRepositoryTool,
    entrypoint: ApprovedScript,
    entrypoint_fingerprint: ExecutableFingerprint,
}

impl ApprovedNodeNpm {
    pub fn new(
        node: ApprovedRepositoryTool,
        entrypoint: ApprovedScript,
        entrypoint_fingerprint: ExecutableFingerprint,
    ) -> Self {
        Self {
            node,
            entrypoint,
            entrypoint_fingerprint,
        }
    }
}

impl ApprovedCommitlintNode {
    pub fn new(
        node: ApprovedRepositoryTool,
        entrypoint: ApprovedScript,
        entrypoint_fingerprint: ExecutableFingerprint,
    ) -> Self {
        Self {
            node,
            entrypoint,
            entrypoint_fingerprint,
        }
    }
}

pub enum ApprovedCommitlint {
    Native(ApprovedRepositoryTool),
    Node(ApprovedCommitlintNode),
}

enum CommitlintRoute<'a> {
    Native {
        tool: &'a ApprovedRepositoryTool,
        candidate: ToolCandidate,
    },
    Node {
        driver: &'a ApprovedCommitlintNode,
        profile: &'a ToolProfile,
        candidate: ToolCandidate,
    },
}

fn is_commitlint_cli_entrypoint(path: &Path) -> bool {
    let components = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    components.len() >= 4
        && components[components.len() - 4..]
            .iter()
            .zip(["node_modules", "@commitlint", "cli", "cli.js"])
            .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
}

fn node_path_argument(path: &Path) -> OsString {
    #[cfg(windows)]
    {
        let value = path.to_string_lossy();
        if let Some(unc_path) = value.strip_prefix(r"\\?\UNC\") {
            return OsString::from(format!(r"\\{unc_path}"));
        }
        if let Some(path) = value.strip_prefix(r"\\?\") {
            return OsString::from(path);
        }
    }
    path.as_os_str().to_os_string()
}

fn is_npm_cli_entrypoint(path: &Path) -> bool {
    let components = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    components.len() >= 4
        && (components[components.len() - 4..]
            .iter()
            .zip(["node_modules", "npm", "bin", "npm-cli.js"])
            .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
            || components[components.len() - 4..]
                .iter()
                .zip(["nodejs", "npm", "bin", "npm-cli.js"])
                .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected)))
}

/// Resolves a registered npm launcher to the npm JavaScript entrypoint without
/// executing the launcher (which may be a Windows `.cmd` shim).
pub fn npm_cli_entrypoint_for_candidate(candidate: &ToolCandidate) -> Option<PathBuf> {
    if candidate.tool_id() != ToolId::Npm {
        return None;
    }
    let launcher = candidate.path()?;
    let mut candidates = Vec::new();
    if is_npm_cli_entrypoint(launcher) {
        candidates.push(launcher.to_path_buf());
    }
    for ancestor in launcher.ancestors().take(6) {
        for relative in [
            Path::new("node_modules/npm/bin/npm-cli.js"),
            Path::new("lib/node_modules/npm/bin/npm-cli.js"),
            Path::new("share/nodejs/npm/bin/npm-cli.js"),
            Path::new("lib/nodejs/npm/bin/npm-cli.js"),
        ] {
            candidates.push(ancestor.join(relative));
        }
    }
    candidates.into_iter().find_map(|candidate| {
        let metadata = std::fs::symlink_metadata(&candidate).ok()?;
        if !metadata.file_type().is_file() {
            return None;
        }
        let canonical = std::fs::canonicalize(candidate).ok()?;
        is_npm_cli_entrypoint(&canonical).then_some(canonical)
    })
}

fn npm_script_for_suite(suite: TestSuiteKind) -> Option<&'static str> {
    match suite {
        TestSuiteKind::NodeLint => Some("lint"),
        TestSuiteKind::NodeTest => Some("test"),
        TestSuiteKind::NodeBuild => Some("build"),
        TestSuiteKind::CargoTest => None,
    }
}

fn node_version_satisfies_npm_11(version: &semver::Version) -> bool {
    let node_20_minimum = semver::Version::new(20, 17, 0);
    let node_22_minimum = semver::Version::new(22, 9, 0);
    (version.major == 20 && version >= &node_20_minimum)
        || (version.major >= 22 && version < &semver::Version::new(25, 0, 0))
            && version >= &node_22_minimum
}

fn approved_npm_shell(_environment: &ApprovedEnv) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let system_root = _environment
            .entries()
            .iter()
            .find(|(key, _)| key.to_string_lossy().eq_ignore_ascii_case("SYSTEMROOT"))
            .map(|(_, value)| PathBuf::from(value.as_os_str()))?;
        let shell = system_root.join("System32").join("cmd.exe");
        return std::fs::metadata(&shell)
            .is_ok_and(|metadata| metadata.is_file())
            .then_some(shell);
    }
    #[cfg(unix)]
    {
        let shell = PathBuf::from("/bin/sh");
        return std::fs::metadata(&shell)
            .is_ok_and(|metadata| metadata.is_file())
            .then_some(shell);
    }
    #[allow(unreachable_code)]
    None
}

fn approved_environment_with_node_path(
    environment: &ApprovedEnv,
    node_path: &Path,
) -> AppResult<ApprovedEnv> {
    let node_directory = node_path.parent().ok_or_else(|| {
        AppError::Validation(vec![Diagnostic::error(
            "test_suite.node.path.invalid",
            "The approved Node executable has no parent directory.",
        )])
    })?;
    let mut entries = environment.entries().clone();
    let path_key = entries
        .keys()
        .find(|key| key.to_string_lossy().eq_ignore_ascii_case("PATH"))
        .cloned();
    let current_path = path_key.as_ref().and_then(|key| entries.get(key)).cloned();
    entries.retain(|key, _| !key.to_string_lossy().eq_ignore_ascii_case("PATH"));
    let mut paths = vec![node_directory.to_path_buf()];
    if let Some(current_path) = current_path {
        paths.extend(std::env::split_paths(&current_path));
    }
    let path = std::env::join_paths(paths).map_err(|_| {
        AppError::Validation(vec![Diagnostic::error(
            "test_suite.node.path.invalid",
            "The approved Node search path could not be constructed.",
        )])
    })?;
    entries.insert(OsString::from("PATH"), path);
    ApprovedEnv::new(entries).map_err(AppError::Validation)
}

fn repository_has_npmrc(root: &Path) -> bool {
    match std::fs::symlink_metadata(root.join(".npmrc")) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

fn npm_config_args(config: &PrivateNpmRunConfig, root: &Path) -> Vec<OsString> {
    vec![
        OsString::from("--userconfig"),
        node_path_argument(&config.user_config),
        OsString::from("--globalconfig"),
        node_path_argument(&config.global_config),
        OsString::from("--cache"),
        node_path_argument(&config.cache),
        OsString::from("--logs-dir"),
        node_path_argument(&config.logs),
        OsString::from("--offline"),
        OsString::from("--no-audit"),
        OsString::from("--no-fund"),
        OsString::from("--no-update-notifier"),
        OsString::from("--prefix"),
        node_path_argument(root),
    ]
}

#[allow(clippy::too_many_arguments)]
fn node_npm_process_spec(
    driver: &ApprovedNodeNpm,
    arguments: Vec<OsString>,
    cwd: &ApprovedRoot,
    environment: &ApprovedEnv,
    permission: ProcessPermission,
    cancellation: CancellationToken,
    timeout: Duration,
    output_limit_bytes: usize,
) -> AppResult<ProcessSpec> {
    Ok(ProcessSpec::new(
        ApprovedExecutable::from_absolute_path(driver.node.executable.path().to_path_buf())
            .map_err(AppError::Validation)?,
        ToolId::Npm,
        arguments,
        ApprovedRoot::from_absolute_path(cwd.path().to_path_buf()).map_err(AppError::Validation)?,
        ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?,
        timeout,
        output_limit_bytes,
        permission,
        cancellation,
    )
    .map_err(AppError::Validation)?
    .with_approved_executable_fingerprint(driver.node.fingerprint)
    .with_approved_script(
        ApprovedScript::from_absolute_path(driver.entrypoint.path().to_path_buf())
            .map_err(AppError::Validation)?,
        driver.entrypoint_fingerprint,
    ))
}

async fn run_node_commitlint(
    driver: &ApprovedCommitlintNode,
    arguments: &[OsString],
    cwd: &ApprovedRoot,
    environment: &ApprovedEnv,
    process: &dyn ProcessPort,
) -> AppResult<jameskills_core::ports::process::ProcessOutput> {
    let mut args = vec![node_path_argument(driver.entrypoint.path())];
    args.extend_from_slice(arguments);
    let spec = ProcessSpec::new(
        ApprovedExecutable::from_absolute_path(driver.node.executable.path().to_path_buf())
            .map_err(AppError::Validation)?,
        ToolId::Commitlint,
        args,
        ApprovedRoot::from_absolute_path(cwd.path().to_path_buf()).map_err(AppError::Validation)?,
        ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?,
        Duration::from_secs(5),
        16 * 1024,
        ProcessPermission::ReadOnlyCheck,
        CancellationToken::new(),
    )
    .map_err(AppError::Validation)?
    .with_approved_executable_fingerprint(driver.node.fingerprint)
    .with_approved_script(
        ApprovedScript::from_absolute_path(driver.entrypoint.path().to_path_buf())
            .map_err(AppError::Validation)?,
        driver.entrypoint_fingerprint,
    );
    process.run(spec).await
}

/// Per-repository provider. It stores only approved executable identities and
/// returns Unknown for check kinds that do not have an implemented driver.
pub struct RepositoryPolicyCheckProvider {
    root: PathBuf,
    git: Option<ApprovedRepositoryTool>,
    gitleaks: Option<ApprovedRepositoryTool>,
    cargo: Option<ApprovedRepositoryTool>,
    node_npm: Option<ApprovedNodeNpm>,
    commitlint: Option<ApprovedCommitlint>,
    github: Option<ApprovedRepositoryTool>,
    environment: ApprovedEnv,
    process: Arc<dyn ProcessPort>,
    clock: Arc<dyn ClockPort>,
    environment_fingerprint: String,
}

impl RepositoryPolicyCheckProvider {
    pub fn new(
        root: ApprovedRoot,
        git: Option<ApprovedRepositoryTool>,
        gitleaks: Option<ApprovedRepositoryTool>,
        environment: ApprovedEnv,
        process: Arc<dyn ProcessPort>,
        clock: Arc<dyn ClockPort>,
        environment_fingerprint: String,
    ) -> Self {
        Self {
            root: root.path().to_path_buf(),
            git,
            gitleaks,
            cargo: None,
            node_npm: None,
            commitlint: None,
            github: None,
            environment,
            process,
            clock,
            environment_fingerprint,
        }
    }

    pub fn with_commitlint(mut self, commitlint: ApprovedCommitlint) -> Self {
        self.commitlint = Some(commitlint);
        self
    }

    pub fn with_github_cli(mut self, gh: ApprovedRepositoryTool) -> Self {
        self.github = Some(gh);
        self
    }

    pub fn with_cargo(mut self, cargo: ApprovedRepositoryTool) -> Self {
        self.cargo = Some(cargo);
        self
    }

    pub fn with_node_npm(mut self, node_npm: ApprovedNodeNpm) -> Self {
        self.node_npm = Some(node_npm);
        self
    }

    async fn inspect_test_suite(
        &self,
        suite: TestSuiteKind,
        cancellation: CancellationToken,
    ) -> AppResult<TestSuiteSnapshot> {
        if cancellation.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        let root_path = std::fs::canonicalize(&self.root).map_err(|_| AppError::NotFound)?;
        let root =
            ApprovedRoot::from_absolute_path(root_path.clone()).map_err(AppError::Validation)?;
        let manifest_fingerprint = test_suite_manifest_fingerprint(&root_path)?;
        let profiles = load_tool_profiles().map_err(AppError::Validation)?;
        let platform = PlatformFacts::detect().platform;
        let git = self.git.as_ref().ok_or_else(|| AppError::ExternalTool {
            tool_id: "git".to_owned(),
            exit_code: None,
        })?;
        let git_profile = profiles
            .iter()
            .find(|profile| profile.tool_id() == ToolId::Git)
            .ok_or_else(|| {
                AppError::Validation(vec![Diagnostic::error(
                    "tool.profile.missing",
                    "Git profile is not registered.",
                )])
            })?;
        let Some(git_candidate) = candidate_for_approved_tool(git_profile, git, platform) else {
            return Err(AppError::ExternalTool {
                tool_id: "git".to_owned(),
                exit_code: None,
            });
        };
        if git_candidate.kind() != ToolCandidateKind::NativeExecutable {
            return Err(AppError::PermissionDenied {
                operation: "test_suite.git.shim.blocked".to_owned(),
            });
        }
        let git_version = probe_registered_tool_version(
            git_profile,
            &git_candidate,
            Some(git.fingerprint),
            &root,
            &self.environment,
            self.process.as_ref(),
            &self.clock.now_utc(),
        )
        .await?;
        if git_version.availability() != ToolAvailability::Candidate
            || git_version.version_status() != ToolVersionStatus::Compatible
            || git_version.version().is_none()
        {
            return Err(AppError::ExternalTool {
                tool_id: "git".to_owned(),
                exit_code: None,
            });
        }
        let head_spec = registered_process_spec(
            git,
            ToolId::Git,
            ["rev-parse", "--verify", "HEAD"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            &root,
            &self.environment,
            ProcessPermission::ReadOnlyCheck,
            cancellation.clone(),
            Duration::from_secs(5),
            256,
        )?;
        let head_output = self.process.run(head_spec).await?;
        let head_bytes = head_output.stdout();
        let head_text = std::str::from_utf8(head_bytes).map_err(|_| AppError::UntrustedInput {
            code: "test_suite.head.invalid".to_owned(),
        })?;
        let head_text = head_text.trim_end_matches(['\r', '\n']);
        let expected_head =
            if head_output.exit_code() == Some(0) && !head_text.contains(['\r', '\n']) {
                RepositoryHead::parse(head_text).map_err(AppError::Validation)?
            } else {
                return Err(AppError::ExternalTool {
                    tool_id: "git".to_owned(),
                    exit_code: head_output.exit_code(),
                });
            };
        let declaration = match suite {
            TestSuiteKind::CargoTest => {
                self.inspect_cargo_test_targets(&profiles, &root, &cancellation)
                    .await?
            }
            TestSuiteKind::NodeLint | TestSuiteKind::NodeTest | TestSuiteKind::NodeBuild => {
                let facts = crate::platform::inspect_project_manifests(&root)?;
                if matches!(facts.stack(), crate::platform::ProjectStack::Unknown) {
                    TestSuiteDeclaration::Unknown
                } else {
                    let script = match suite {
                        TestSuiteKind::NodeLint => "lint",
                        TestSuiteKind::NodeTest => "test",
                        TestSuiteKind::NodeBuild => "build",
                        TestSuiteKind::CargoTest => unreachable!(),
                    };
                    if facts.node_script_names().iter().any(|name| name == script) {
                        TestSuiteDeclaration::Declared
                    } else {
                        TestSuiteDeclaration::Missing
                    }
                }
            }
        };
        Ok(TestSuiteSnapshot::new(
            root,
            suite,
            expected_head,
            manifest_fingerprint,
            declaration,
        ))
    }

    async fn inspect_cargo_test_targets(
        &self,
        profiles: &[ToolProfile],
        root: &ApprovedRoot,
        cancellation: &CancellationToken,
    ) -> AppResult<TestSuiteDeclaration> {
        let Some(cargo) = self.cargo.as_ref() else {
            return Ok(TestSuiteDeclaration::Unknown);
        };
        let Some(profile) = profiles
            .iter()
            .find(|profile| profile.tool_id() == ToolId::Cargo)
        else {
            return Ok(TestSuiteDeclaration::Unknown);
        };
        let Some(candidate) =
            candidate_for_approved_tool(profile, cargo, PlatformFacts::detect().platform)
        else {
            return Ok(TestSuiteDeclaration::Unknown);
        };
        if candidate.kind() != ToolCandidateKind::NativeExecutable {
            return Ok(TestSuiteDeclaration::Unknown);
        }
        let version = probe_registered_tool_version(
            profile,
            &candidate,
            Some(cargo.fingerprint),
            root,
            &self.environment,
            self.process.as_ref(),
            &self.clock.now_utc(),
        )
        .await?;
        if version.availability() != ToolAvailability::Candidate
            || version.version_status() != ToolVersionStatus::Compatible
            || version.version().is_none()
        {
            return Ok(TestSuiteDeclaration::Unknown);
        }
        if matches!(
            read_repository_document(
                root.path(),
                &PortablePath::new("Cargo.toml".to_owned()).map_err(|_| {
                    AppError::Validation(vec![Diagnostic::error(
                        "test_suite.manifest.path.invalid",
                        "The app-owned Cargo manifest path is invalid.",
                    )])
                })?
            ),
            ReadmeFile::Missing
        ) {
            return Ok(TestSuiteDeclaration::Missing);
        }
        let manifest_path = node_path_argument(&root.path().join("Cargo.toml"));
        let metadata_spec = registered_process_spec(
            cargo,
            ToolId::Cargo,
            [
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--locked",
                "--offline",
            ]
            .into_iter()
            .map(OsString::from)
            .chain([OsString::from("--manifest-path"), manifest_path])
            .collect(),
            root,
            &self.environment,
            ProcessPermission::ReadOnlyCheck,
            cancellation.clone(),
            Duration::from_secs(15),
            MAX_CARGO_METADATA_BYTES,
        )?;
        let output = match self.process.run(metadata_spec).await {
            Ok(output) => output,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return Ok(TestSuiteDeclaration::Unknown);
            }
            Err(error) => return Err(error),
        };
        if output.exit_code() != Some(0) {
            return Ok(TestSuiteDeclaration::Unknown);
        }
        Ok(match cargo_metadata_declares_test_target(output.stdout()) {
            Some(true) => TestSuiteDeclaration::Declared,
            Some(false) => TestSuiteDeclaration::Missing,
            None => TestSuiteDeclaration::Unknown,
        })
    }

    async fn run_cargo_test(
        &self,
        approval: TestSuiteRunApproval,
        cancellation: CancellationToken,
    ) -> AppResult<TestSuiteRunResult> {
        let snapshot = approval.snapshot();
        let Some(cargo) = self.cargo.as_ref() else {
            return suite_result(
                snapshot.suite(),
                snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        };
        let profiles = load_tool_profiles().map_err(AppError::Validation)?;
        let Some(profile) = profiles
            .iter()
            .find(|profile| profile.tool_id() == ToolId::Cargo)
        else {
            return suite_result(
                snapshot.suite(),
                snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        };
        let Some(candidate) =
            candidate_for_approved_tool(profile, cargo, PlatformFacts::detect().platform)
        else {
            return suite_result(
                snapshot.suite(),
                snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        };
        if candidate.kind() != ToolCandidateKind::NativeExecutable {
            return suite_result(
                snapshot.suite(),
                snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        }
        if !matches!(
            test_suite_manifest_fingerprint(snapshot.root().path()),
            Ok(fingerprint) if &fingerprint == snapshot.manifest_fingerprint()
        ) {
            return suite_result(
                snapshot.suite(),
                snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        }
        let manifest_path = node_path_argument(&snapshot.root().path().join("Cargo.toml"));
        let Some(git) = self.git.as_ref() else {
            return suite_result(
                snapshot.suite(),
                snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        };
        let head_spec = registered_process_spec(
            git,
            ToolId::Git,
            ["rev-parse", "--verify", "HEAD"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            snapshot.root(),
            &self.environment,
            ProcessPermission::ReadOnlyCheck,
            cancellation.clone(),
            Duration::from_secs(5),
            256,
        )?;
        let current_head = match self.process.run(head_spec).await {
            Ok(output) if output.exit_code() == Some(0) => std::str::from_utf8(output.stdout())
                .ok()
                .map(|head| head.trim_end_matches(['\r', '\n']).to_owned()),
            Ok(_) => None,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => None,
            Err(error) => return Err(error),
        };
        if !matches!(current_head.as_deref(), Some(head) if head == snapshot.expected_head().as_str())
        {
            return suite_result(
                snapshot.suite(),
                snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        }
        let spec = registered_process_spec(
            cargo,
            ToolId::Cargo,
            ["test", "--workspace", "--locked"]
                .into_iter()
                .map(OsString::from)
                .chain([OsString::from("--manifest-path"), manifest_path])
                .collect(),
            snapshot.root(),
            &self.environment,
            ProcessPermission::ExplicitMutation(approval.operation_id()),
            cancellation,
            Duration::from_secs(120),
            64 * 1024,
        )?;
        let output = match self.process.run(spec).await {
            Ok(output) => output,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return suite_result(
                    snapshot.suite(),
                    snapshot.declaration(),
                    TestSuiteExecution::Blocked,
                    None,
                );
            }
            Err(error) => return Err(error),
        };
        let (execution, exit_code) = match output.exit_code() {
            Some(0) => (TestSuiteExecution::Passed, Some(0)),
            Some(code) => (TestSuiteExecution::Failed, Some(code)),
            None => (TestSuiteExecution::Blocked, None),
        };
        suite_result(
            snapshot.suite(),
            snapshot.declaration(),
            execution,
            exit_code,
        )
    }

    async fn run_node_suite(
        &self,
        approval: TestSuiteRunApproval,
        cancellation: CancellationToken,
    ) -> AppResult<TestSuiteRunResult> {
        let snapshot = approval.snapshot();
        let blocked = || {
            suite_result(
                snapshot.suite(),
                snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            )
        };
        let Some(driver) = self.node_npm.as_ref() else {
            return blocked();
        };
        let Some(script) = npm_script_for_suite(snapshot.suite()) else {
            return blocked();
        };
        if !is_npm_cli_entrypoint(driver.entrypoint.path())
            || repository_has_npmrc(snapshot.root().path())
        {
            return blocked();
        }
        if !matches!(
            test_suite_manifest_fingerprint(snapshot.root().path()),
            Ok(fingerprint) if &fingerprint == snapshot.manifest_fingerprint()
        ) {
            return blocked();
        }
        let config = match PrivateNpmRunConfig::create(snapshot.root().path()) {
            Ok(config) => config,
            Err(_) => return blocked(),
        };
        let environment = match approved_environment_with_node_path(
            &self.environment,
            driver.node.executable.path(),
        ) {
            Ok(environment) => environment,
            Err(_) => return blocked(),
        };
        let shell = match approved_npm_shell(&environment) {
            Some(shell) => shell,
            None => return blocked(),
        };

        let profiles = load_tool_profiles().map_err(AppError::Validation)?;
        let Some(node_profile) = profiles
            .iter()
            .find(|profile| profile.tool_id() == ToolId::Node)
        else {
            return blocked();
        };
        let Some(npm_profile) = profiles
            .iter()
            .find(|profile| profile.tool_id() == ToolId::Npm)
        else {
            return blocked();
        };
        let platform = PlatformFacts::detect().platform;
        let Some(node_candidate) =
            candidate_for_approved_tool(node_profile, &driver.node, platform)
        else {
            return blocked();
        };
        if node_candidate.kind() != ToolCandidateKind::NativeExecutable {
            return blocked();
        }
        let node_version_spec = registered_process_spec(
            &driver.node,
            ToolId::Node,
            node_profile
                .version_args()
                .iter()
                .cloned()
                .map(OsString::from)
                .collect(),
            snapshot.root(),
            &environment,
            ProcessPermission::ReadOnlyCheck,
            cancellation.clone(),
            Duration::from_secs(3),
            MAX_NPM_VERSION_BYTES,
        )?;
        let node_output = match self.process.run(node_version_spec).await {
            Ok(output) if output.exit_code() == Some(0) => output,
            Ok(_) => return blocked(),
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return blocked();
            }
            Err(error) => return Err(error),
        };
        let Some(node_version) = parse_tool_version_output(node_profile, node_output.stdout())
        else {
            return blocked();
        };
        if !node_profile.version_range().matches(&node_version)
            || !node_version_satisfies_npm_11(&node_version)
        {
            return blocked();
        }

        let mut npm_version_args = vec![node_path_argument(driver.entrypoint.path())];
        npm_version_args.extend(npm_config_args(&config, snapshot.root().path()));
        npm_version_args.push(OsString::from("--version"));
        let npm_version_spec = node_npm_process_spec(
            driver,
            npm_version_args,
            snapshot.root(),
            &environment,
            ProcessPermission::ReadOnlyCheck,
            cancellation.clone(),
            Duration::from_secs(5),
            MAX_NPM_VERSION_BYTES,
        )?;
        let npm_output = match self.process.run(npm_version_spec).await {
            Ok(output) if output.exit_code() == Some(0) => output,
            Ok(_) => return blocked(),
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return blocked();
            }
            Err(error) => return Err(error),
        };
        let Some(npm_version) = parse_tool_version_output(npm_profile, npm_output.stdout()) else {
            return blocked();
        };
        if !npm_profile.version_range().matches(&npm_version)
            || npm_version != semver::Version::parse(REVIEWED_NPM_VERSION).expect("app version")
        {
            return blocked();
        }

        if !matches!(
            test_suite_manifest_fingerprint(snapshot.root().path()),
            Ok(fingerprint) if &fingerprint == snapshot.manifest_fingerprint()
        ) || repository_has_npmrc(snapshot.root().path())
        {
            return blocked();
        }
        let Some(git) = self.git.as_ref() else {
            return blocked();
        };
        let head_spec = registered_process_spec(
            git,
            ToolId::Git,
            ["rev-parse", "--verify", "HEAD"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            snapshot.root(),
            &environment,
            ProcessPermission::ReadOnlyCheck,
            cancellation.clone(),
            Duration::from_secs(5),
            256,
        )?;
        let current_head = match self.process.run(head_spec).await {
            Ok(output) if output.exit_code() == Some(0) => std::str::from_utf8(output.stdout())
                .ok()
                .map(|head| head.trim_end_matches(['\r', '\n']).to_owned()),
            Ok(_) => None,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => None,
            Err(error) => return Err(error),
        };
        if !matches!(current_head.as_deref(), Some(head) if head == snapshot.expected_head().as_str())
        {
            return blocked();
        }

        let shell = node_path_argument(&shell);
        let mut arguments = vec![node_path_argument(driver.entrypoint.path())];
        arguments.extend(npm_config_args(&config, snapshot.root().path()));
        arguments.extend([
            OsString::from("--ignore-scripts"),
            OsString::from("--script-shell"),
            shell,
            OsString::from("--workspaces=false"),
            OsString::from("--include-workspace-root=false"),
            OsString::from("run-script"),
            OsString::from(script),
        ]);
        let spec = node_npm_process_spec(
            driver,
            arguments,
            snapshot.root(),
            &environment,
            ProcessPermission::ExplicitMutation(approval.operation_id()),
            cancellation,
            Duration::from_secs(120),
            MAX_NPM_OUTPUT_BYTES,
        )?;
        let output = match self.process.run(spec).await {
            Ok(output) => output,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return blocked();
            }
            Err(error) => return Err(error),
        };
        let (execution, exit_code) = match output.exit_code() {
            Some(0) => (TestSuiteExecution::Passed, Some(0)),
            Some(code) => (TestSuiteExecution::Failed, Some(code)),
            None => (TestSuiteExecution::Blocked, None),
        };
        suite_result(
            snapshot.suite(),
            snapshot.declaration(),
            execution,
            exit_code,
        )
    }
}

#[async_trait::async_trait]
impl PolicyCheckProvider for RepositoryPolicyCheckProvider {
    async fn observe(&self, requirement: &Requirement) -> AppResult<CheckObservation> {
        let root =
            ApprovedRoot::from_absolute_path(self.root.clone()).map_err(AppError::Validation)?;
        let observed_at = self.clock.now_utc();
        let filesystem = LocalFileSystem;
        match requirement.check() {
            Check::GithubAccess => {
                let Some(git) = self.git.as_ref() else {
                    return Ok(CheckObservation::unknown());
                };
                let Some(gh) = self.github.as_ref() else {
                    return Ok(CheckObservation::unknown());
                };
                let repository_root = match std::fs::canonicalize(&self.root) {
                    Ok(path) => path,
                    Err(_) => return Ok(CheckObservation::unknown()),
                };
                let repository = match observe_github_remote_host(
                    &repository_root,
                    Some(git),
                    &self.environment,
                    self.process.as_ref(),
                    &observed_at,
                )
                .await?
                {
                    Ok(Some(repository)) => repository,
                    Ok(None) => return Ok(CheckObservation::unknown()),
                    Err(_) => return Ok(CheckObservation::unknown()),
                };
                let head = match observe_repository_head(
                    &repository_root,
                    git,
                    &self.environment,
                    self.process.as_ref(),
                )
                .await?
                {
                    Some(head) => head,
                    None => return Ok(CheckObservation::unknown()),
                };
                let github_root = ApprovedRoot::from_absolute_path(repository_root)
                    .map_err(AppError::Validation)?;
                let driver = GithubEvidenceDriver::new(
                    gh,
                    &github_root,
                    &self.environment,
                    self.process.as_ref(),
                    self.clock.as_ref(),
                    &self.environment_fingerprint,
                );
                driver.identify_repository(&repository, &head).await
            }
            Check::GithubBranchPolicy {
                branch,
                require_pull_request,
                required_checks,
                require_no_bypass,
            } => {
                let Some(git) = self.git.as_ref() else {
                    return Ok(CheckObservation::unknown());
                };
                let Some(gh) = self.github.as_ref() else {
                    return Ok(CheckObservation::unknown());
                };
                let repository_root = match std::fs::canonicalize(&self.root) {
                    Ok(path) => path,
                    Err(_) => return Ok(CheckObservation::unknown()),
                };
                let repository = match observe_github_remote_host(
                    &repository_root,
                    Some(git),
                    &self.environment,
                    self.process.as_ref(),
                    &observed_at,
                )
                .await?
                {
                    Ok(Some(repository)) => repository,
                    Ok(None) | Err(_) => return Ok(CheckObservation::unknown()),
                };
                let head = match observe_repository_head(
                    &repository_root,
                    git,
                    &self.environment,
                    self.process.as_ref(),
                )
                .await?
                {
                    Some(head) => head,
                    None => return Ok(CheckObservation::unknown()),
                };
                let github_root = ApprovedRoot::from_absolute_path(repository_root)
                    .map_err(AppError::Validation)?;
                let driver = GithubEvidenceDriver::new(
                    gh,
                    &github_root,
                    &self.environment,
                    self.process.as_ref(),
                    self.clock.as_ref(),
                    &self.environment_fingerprint,
                );
                driver
                    .check_branch_policy(
                        &repository,
                        &head,
                        branch,
                        *require_pull_request,
                        required_checks,
                        *require_no_bypass,
                    )
                    .await
            }
            Check::CiEvidence { required_checks } => {
                let Some(git) = self.git.as_ref() else {
                    return Ok(CheckObservation::unknown());
                };
                let Some(gh) = self.github.as_ref() else {
                    return Ok(CheckObservation::unknown());
                };
                let repository_root = match std::fs::canonicalize(&self.root) {
                    Ok(path) => path,
                    Err(_) => return Ok(CheckObservation::unknown()),
                };
                let repository = match observe_github_remote_host(
                    &repository_root,
                    Some(git),
                    &self.environment,
                    self.process.as_ref(),
                    &observed_at,
                )
                .await?
                {
                    Ok(Some(repository)) => repository,
                    Ok(None) | Err(_) => return Ok(CheckObservation::unknown()),
                };
                let head = match observe_repository_head(
                    &repository_root,
                    git,
                    &self.environment,
                    self.process.as_ref(),
                )
                .await?
                {
                    Some(head) => head,
                    None => return Ok(CheckObservation::unknown()),
                };
                let github_root = ApprovedRoot::from_absolute_path(repository_root)
                    .map_err(AppError::Validation)?;
                let driver = GithubEvidenceDriver::new(
                    gh,
                    &github_root,
                    &self.environment,
                    self.process.as_ref(),
                    self.clock.as_ref(),
                    &self.environment_fingerprint,
                );
                driver
                    .check_ci_evidence(&repository, &head, required_checks)
                    .await
            }
            Check::ReadmeSections { path, headings } => filesystem.check_readme_sections(
                &root,
                path,
                headings,
                &observed_at,
                &self.environment_fingerprint,
            ),
            Check::GitignorePatterns { path, patterns } => {
                let driver = self
                    .git
                    .as_ref()
                    .map(|tool| (&tool.executable, tool.fingerprint));
                filesystem
                    .check_gitignore_patterns(
                        &root,
                        path,
                        patterns,
                        driver,
                        &self.environment,
                        self.process.as_ref(),
                        &observed_at,
                        &self.environment_fingerprint,
                    )
                    .await
            }
            Check::TrackedSecrets { include_history } => {
                let profiles = load_tool_profiles().map_err(AppError::Validation)?;
                let profile = profiles
                    .iter()
                    .find(|profile| profile.tool_id() == ToolId::Gitleaks)
                    .ok_or_else(|| {
                        AppError::Validation(vec![Diagnostic::error(
                            "tool.profile.missing",
                            "Gitleaks profile is not registered.",
                        )])
                    })?;
                let search_paths = self
                    .gitleaks
                    .as_ref()
                    .and_then(|tool| tool.executable.path().parent())
                    .map(Path::to_path_buf)
                    .into_iter()
                    .collect::<Vec<_>>();
                let platform = PlatformFacts::detect().platform;
                let candidate =
                    find_tool_candidates(std::slice::from_ref(profile), &search_paths, platform)
                        .into_iter()
                        .next()
                        .ok_or_else(|| {
                            AppError::Validation(vec![Diagnostic::error(
                                "tool.candidate.missing",
                                "Gitleaks candidate could not be resolved.",
                            )])
                        })?;
                let approved_fingerprint = self.gitleaks.as_ref().and_then(|tool| {
                    let candidate_path = candidate.path()?;
                    let approved_path = std::fs::canonicalize(tool.executable.path()).ok()?;
                    (candidate_path == approved_path).then_some(tool.fingerprint)
                });
                filesystem
                    .check_tracked_secrets(
                        &root,
                        profile,
                        &candidate,
                        approved_fingerprint,
                        *include_history,
                        &self.environment,
                        self.process.as_ref(),
                        &observed_at,
                        &self.environment_fingerprint,
                    )
                    .await
            }
            Check::ConventionalCommit => {
                filesystem
                    .check_conventional_commit(
                        &root,
                        self.git.as_ref(),
                        self.commitlint.as_ref(),
                        &self.environment,
                        self.process.as_ref(),
                        &observed_at,
                        &self.environment_fingerprint,
                    )
                    .await
            }
            Check::CiContract {
                workflow_paths,
                required_jobs,
            } => {
                filesystem
                    .check_ci_contract(
                        &root,
                        workflow_paths,
                        required_jobs,
                        self.git.as_ref(),
                        &self.environment,
                        self.process.as_ref(),
                        &observed_at,
                        &self.environment_fingerprint,
                    )
                    .await
            }
            _ => Ok(CheckObservation::unknown()),
        }
    }
}

#[async_trait::async_trait]
impl TestSuiteRunnerPort for RepositoryPolicyCheckProvider {
    async fn inspect(
        &self,
        suite: TestSuiteKind,
        cancellation: CancellationToken,
    ) -> AppResult<TestSuiteSnapshot> {
        self.inspect_test_suite(suite, cancellation).await
    }

    async fn run(
        &self,
        approval: TestSuiteRunApproval,
        cancellation: CancellationToken,
    ) -> AppResult<TestSuiteRunResult> {
        let approved_snapshot = approval.snapshot();
        let Some(current_root) = std::fs::canonicalize(&self.root).ok() else {
            return suite_result(
                approved_snapshot.suite(),
                approved_snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        };
        let Some(approved_root) = std::fs::canonicalize(approved_snapshot.root().path()).ok()
        else {
            return suite_result(
                approved_snapshot.suite(),
                approved_snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        };
        if current_root != approved_root {
            return suite_result(
                approved_snapshot.suite(),
                approved_snapshot.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        }
        let current = self
            .inspect_test_suite(approved_snapshot.suite(), cancellation.clone())
            .await?;
        if current.expected_head() != approved_snapshot.expected_head()
            || current.manifest_fingerprint() != approved_snapshot.manifest_fingerprint()
            || current.declaration() != approved_snapshot.declaration()
        {
            return suite_result(
                current.suite(),
                current.declaration(),
                TestSuiteExecution::Blocked,
                None,
            );
        }
        match current.declaration() {
            TestSuiteDeclaration::Missing => suite_result(
                current.suite(),
                current.declaration(),
                TestSuiteExecution::NotRun,
                None,
            ),
            TestSuiteDeclaration::Unknown => suite_result(
                current.suite(),
                current.declaration(),
                TestSuiteExecution::Blocked,
                None,
            ),
            TestSuiteDeclaration::Declared => match current.suite() {
                TestSuiteKind::CargoTest => self.run_cargo_test(approval, cancellation).await,
                TestSuiteKind::NodeLint | TestSuiteKind::NodeTest | TestSuiteKind::NodeBuild => {
                    self.run_node_suite(approval, cancellation).await
                }
            },
        }
    }
}

fn suite_result(
    suite: TestSuiteKind,
    declaration: TestSuiteDeclaration,
    execution: TestSuiteExecution,
    exit_code: Option<i32>,
) -> AppResult<TestSuiteRunResult> {
    TestSuiteRunResult::new(suite, declaration, execution, exit_code).map_err(AppError::Validation)
}

#[allow(clippy::too_many_arguments)]
fn registered_process_spec(
    tool: &ApprovedRepositoryTool,
    tool_id: ToolId,
    args: Vec<OsString>,
    cwd: &ApprovedRoot,
    environment: &ApprovedEnv,
    permission: ProcessPermission,
    cancellation: CancellationToken,
    timeout: Duration,
    output_limit_bytes: usize,
) -> AppResult<ProcessSpec> {
    Ok(ProcessSpec::new(
        ApprovedExecutable::from_absolute_path(tool.executable.path().to_path_buf())
            .map_err(AppError::Validation)?,
        tool_id,
        args,
        ApprovedRoot::from_absolute_path(cwd.path().to_path_buf()).map_err(AppError::Validation)?,
        ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?,
        timeout,
        output_limit_bytes,
        permission,
        cancellation,
    )
    .map_err(AppError::Validation)?
    .with_approved_executable_fingerprint(tool.fingerprint))
}

enum ReadmeFile {
    Missing,
    Blocked,
    Bytes(Vec<u8>),
}

fn read_repository_document(root: &Path, path: &PortablePath) -> ReadmeFile {
    let target = root.join(path.as_str());
    let metadata = match std::fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return ReadmeFile::Missing,
        Err(_) => return ReadmeFile::Blocked,
    };
    if !metadata.file_type().is_file() || metadata.len() > MAX_README_BYTES {
        return ReadmeFile::Blocked;
    }
    let Ok(canonical) = std::fs::canonicalize(&target) else {
        return ReadmeFile::Blocked;
    };
    if canonical != target {
        return ReadmeFile::Blocked;
    }
    let Ok(mut file) = std::fs::File::open(canonical) else {
        return ReadmeFile::Blocked;
    };
    let Ok(opened_metadata) = file.metadata() else {
        return ReadmeFile::Blocked;
    };
    if !opened_metadata.file_type().is_file()
        || opened_metadata.len() > MAX_README_BYTES
        || opened_metadata.len() != metadata.len()
    {
        return ReadmeFile::Blocked;
    }
    let mut bytes = Vec::with_capacity(opened_metadata.len() as usize);
    if Read::by_ref(&mut file)
        .take(MAX_README_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > MAX_README_BYTES
    {
        return ReadmeFile::Blocked;
    }
    let Ok(after_metadata) = file.metadata() else {
        return ReadmeFile::Blocked;
    };
    if bytes.len() as u64 != after_metadata.len()
        || opened_metadata.modified().ok() != after_metadata.modified().ok()
    {
        return ReadmeFile::Blocked;
    }
    ReadmeFile::Bytes(bytes)
}

fn test_suite_manifest_fingerprint(root: &Path) -> AppResult<ContentHash> {
    let mut hasher = Sha256::new();
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "package.json",
        "package-lock.json",
        ".npmrc",
        ".npmrc",
    ] {
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        let portable = PortablePath::new(name.to_owned()).map_err(|_| {
            AppError::Validation(vec![Diagnostic::error(
                "test_suite.manifest.path.invalid",
                "The app-owned suite manifest path is invalid.",
            )])
        })?;
        match read_repository_document(root, &portable) {
            ReadmeFile::Missing => hasher.update([0]),
            ReadmeFile::Blocked => {
                return Err(AppError::UntrustedInput {
                    code: "test_suite.manifest.unreadable".to_owned(),
                });
            }
            ReadmeFile::Bytes(bytes) => {
                hasher.update([1]);
                hasher.update((bytes.len() as u64).to_le_bytes());
                hasher.update(bytes);
            }
        }
    }
    Ok(ContentHash::from_digest(hasher.finalize().into()))
}

fn cargo_metadata_declares_test_target(output: &[u8]) -> Option<bool> {
    if output.is_empty() || output.len() > MAX_CARGO_METADATA_BYTES || output.contains(&0) {
        return None;
    }
    let metadata: serde_json::Value = serde_json::from_slice(output).ok()?;
    let workspace_members = metadata.get("workspace_members")?.as_array()?;
    let packages = metadata.get("packages")?.as_array()?;
    let mut found_target = false;
    let mut declared_test = false;
    for package in packages {
        let id = package.get("id")?.as_str()?;
        if !workspace_members
            .iter()
            .any(|member| member.as_str() == Some(id))
        {
            continue;
        }
        for target in package.get("targets")?.as_array()? {
            found_target = true;
            let is_test = target.get("test")?.as_bool()?;
            declared_test |= is_test;
        }
    }
    found_target.then_some(declared_test)
}

fn synthetic_ignore_path(pattern: &str) -> Option<(&'static str, &'static str)> {
    match pattern {
        ".env" => Some((".env", "repo.gitignore.env")),
        "*.key" => Some(("jameskills-policy-probe.key", "repo.gitignore.key")),
        "target/" => Some((
            "target/jameskills-policy-probe.txt",
            "repo.gitignore.target",
        )),
        "node_modules/" => Some((
            "node_modules/jameskills-policy-probe.txt",
            "repo.gitignore.node-modules",
        )),
        _ => None,
    }
}

fn git_ignore_match(output: &[u8], pattern: &str, synthetic_path: &str, source_path: &str) -> bool {
    if output.len() > 4096 {
        return false;
    }
    let fields = output.split(|byte| *byte == 0).collect::<Vec<_>>();
    fields.len() == 5
        && fields[0] == source_path.as_bytes()
        && !fields[1].is_empty()
        && fields[1].iter().all(u8::is_ascii_digit)
        && fields[2] == pattern.as_bytes()
        && fields[3] == synthetic_path.as_bytes()
        && fields[4].is_empty()
}

struct CiWorkflowFacts {
    push: bool,
    pull_request: bool,
    job_ids: BTreeSet<String>,
}

enum CiWorkflowIssue {
    Fail(&'static str),
    Unknown(&'static str),
}

async fn observe_github_remote_host(
    repository_root: &Path,
    git: Option<&ApprovedRepositoryTool>,
    environment: &ApprovedEnv,
    process: &dyn ProcessPort,
    observed_at: &str,
) -> AppResult<Result<Option<GithubRepository>, &'static str>> {
    let unknown = "The configured Git remote host is absent, unsupported, or unreadable.";
    let Some(git) = git else {
        return Ok(Err(
            "An approved Git driver is required to observe the workflow host.",
        ));
    };
    let profiles = load_tool_profiles().map_err(AppError::Validation)?;
    let Some(profile) = profiles
        .iter()
        .find(|profile| profile.tool_id() == ToolId::Git)
    else {
        return Ok(Err(
            "The registered Git profile is unavailable; workflow host is unknown.",
        ));
    };
    let platform = PlatformFacts::detect().platform;
    let Some(candidate) = candidate_for_approved_tool(profile, git, platform) else {
        return Ok(Err(
            "The approved Git driver is not a registered candidate; workflow host is unknown.",
        ));
    };
    if candidate.kind() != ToolCandidateKind::NativeExecutable {
        return Ok(Err(
            "A Git command shim cannot establish workflow host identity.",
        ));
    }
    let cwd = ApprovedRoot::from_absolute_path(repository_root.to_path_buf())
        .map_err(AppError::Validation)?;
    let version = probe_registered_tool_version(
        profile,
        &candidate,
        Some(git.fingerprint),
        &cwd,
        environment,
        process,
        observed_at,
    )
    .await?;
    if version.availability() != ToolAvailability::Candidate
        || version.version_status() != ToolVersionStatus::Compatible
    {
        return Ok(Err(
            "The approved Git version is not verified; workflow host is unknown.",
        ));
    }
    let spec = registered_process_spec(
        git,
        ToolId::Git,
        ["remote", "-v"].into_iter().map(OsString::from).collect(),
        &cwd,
        environment,
        ProcessPermission::ReadOnlyCheck,
        CancellationToken::new(),
        Duration::from_secs(5),
        4096,
    )?;
    let output = match process.run(spec).await {
        Ok(output) if output.exit_code() == Some(0) => output,
        Err(AppError::Cancelled) => return Err(AppError::Cancelled),
        Ok(_) | Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
            return Ok(Err(unknown));
        }
        Err(error) => return Err(error),
    };
    if output.stdout().len() > 4096 || !remote_list_targets_github(output.stdout()) {
        return Ok(Err(unknown));
    }
    Ok(Ok(GithubRepository::from_git_remote_output(
        output.stdout(),
    )))
}

async fn observe_repository_head(
    repository_root: &Path,
    git: &ApprovedRepositoryTool,
    environment: &ApprovedEnv,
    process: &dyn ProcessPort,
) -> AppResult<Option<RepositoryHead>> {
    let cwd = ApprovedRoot::from_absolute_path(repository_root.to_path_buf())
        .map_err(AppError::Validation)?;
    let spec = registered_process_spec(
        git,
        ToolId::Git,
        ["rev-parse", "HEAD"]
            .into_iter()
            .map(OsString::from)
            .collect(),
        &cwd,
        environment,
        ProcessPermission::ReadOnlyCheck,
        CancellationToken::new(),
        Duration::from_secs(5),
        128,
    )?;
    let output = match process.run(spec).await {
        Ok(output) if output.exit_code() == Some(0) => output,
        Err(AppError::Cancelled) => return Err(AppError::Cancelled),
        Ok(_) | Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let Ok(text) = std::str::from_utf8(output.stdout()) else {
        return Ok(None);
    };
    let value = text.trim_end_matches(['\r', '\n']);
    if value.contains(['\r', '\n', '\0']) {
        return Ok(None);
    }
    Ok(RepositoryHead::parse(value).ok())
}

fn remote_list_targets_github(output: &[u8]) -> bool {
    let Ok(output) = std::str::from_utf8(output) else {
        return false;
    };
    let mut found_remote = false;
    for line in output.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        let Some((_name, details)) = line.split_once('\t') else {
            return false;
        };
        let Some(remote_url) = details
            .strip_suffix(" (fetch)")
            .or_else(|| details.strip_suffix(" (push)"))
        else {
            return false;
        };
        let Some(host) = remote_url_host(remote_url) else {
            return false;
        };
        if !host.eq_ignore_ascii_case("github.com") {
            return false;
        }
        found_remote = true;
    }
    found_remote
}

fn remote_url_host(remote_url: &str) -> Option<&str> {
    let authority = if let Some((scheme, rest)) = remote_url.split_once("://") {
        if !matches!(scheme, "https" | "http" | "ssh" | "git") {
            return None;
        }
        rest.split(['/', '?', '#']).next()?
    } else {
        let (_, scp_path) = remote_url.rsplit_once('@')?;
        let (host, path) = scp_path.split_once(':')?;
        if path.is_empty() {
            return None;
        }
        host
    };
    let host_with_optional_user = authority.rsplit('@').next()?;
    let host = host_with_optional_user.split(':').next()?;
    (!host.is_empty()).then_some(host)
}

fn inspect_github_workflow(
    source: &str,
    required_jobs: &[String],
) -> Result<CiWorkflowFacts, CiWorkflowIssue> {
    let yaml: serde_json::Value = serde_saphyr::from_str_with_options(
        source,
        serde_saphyr::options! {
            budget: serde_saphyr::budget! {
                flow_nesting_limit: 32,
                max_events: 16_384,
                max_aliases: 0,
                max_anchors: 0,
                max_recorded_anchor_events: 0,
                max_recorded_anchor_bytes: 0,
                max_depth: 32,
                max_inclusion_depth: 0,
                max_documents: 1,
                max_nodes: 8_192,
                max_total_scalar_bytes: MAX_CI_WORKFLOW_BYTES,
                max_total_comment_bytes: 0,
                max_merge_keys: 0,
                max_total_property_interpolation_work: 0,
            },
            duplicate_keys: serde_saphyr::DuplicateKeyPolicy::Error,
            merge_keys: serde_saphyr::MergeKeyPolicy::Error,
            alias_limits: serde_saphyr::alias_limits! {
                max_total_replayed_events: 0,
                max_replay_stack_depth: 0,
                max_alias_expansions_per_anchor: 0,
            },
            emit_comments: false,
            strict_booleans: true,
            no_schema: true,
            reject_unsupported_tags: true,
            with_snippet: false,
        },
    )
    .map_err(|_| {
        CiWorkflowIssue::Unknown(
            "A GitHub Actions workflow could not be parsed within the YAML limits.",
        )
    })?;
    let Some(root) = yaml.as_object() else {
        return Err(CiWorkflowIssue::Unknown(
            "A GitHub Actions workflow must be a YAML mapping.",
        ));
    };
    let Some(on) = root.get("on") else {
        return Err(CiWorkflowIssue::Fail(
            "The workflow declares no push or pull_request trigger.",
        ));
    };
    let triggers = parse_ci_triggers(on)?;
    if triggers.push_config.is_some_and(push_trigger_is_tag_only) {
        return Err(CiWorkflowIssue::Fail(
            "A tags-only push trigger does not run for branch updates.",
        ));
    }
    if let Some(config) = triggers.pull_request_config
        && let Some(config) = config.as_object()
    {
        if config.contains_key("paths") || config.contains_key("paths-ignore") {
            return Err(CiWorkflowIssue::Fail(
                "pull_request path filters can omit required CI checks for a change.",
            ));
        }
        if config.contains_key("branches") || config.contains_key("branches-ignore") {
            return Err(CiWorkflowIssue::Unknown(
                "pull_request branch filters require a known protected-branch scope.",
            ));
        }
        if let Some(types) = config.get("types") {
            let Some(types) = types.as_array() else {
                return Err(CiWorkflowIssue::Unknown(
                    "pull_request activity types have an unsupported shape.",
                ));
            };
            let mut opened = false;
            let mut synchronize = false;
            for event_type in types {
                let Some(event_type) = event_type.as_str() else {
                    return Err(CiWorkflowIssue::Unknown(
                        "pull_request activity types must be YAML strings.",
                    ));
                };
                opened |= event_type == "opened";
                synchronize |= event_type == "synchronize";
            }
            if !opened || !synchronize {
                return Err(CiWorkflowIssue::Fail(
                    "pull_request activity filters must include opened and synchronize.",
                ));
            }
        }
    }
    if !triggers.push || !triggers.pull_request {
        return Err(CiWorkflowIssue::Fail(
            "The workflow must run on both push and pull_request events.",
        ));
    }
    inspect_ci_permissions(root.get("permissions"))?;

    let Some(jobs) = root.get("jobs").and_then(serde_json::Value::as_object) else {
        return Err(CiWorkflowIssue::Fail(
            "The workflow declares no GitHub Actions jobs mapping.",
        ));
    };
    if jobs.is_empty() {
        return Err(CiWorkflowIssue::Fail(
            "The workflow declares no GitHub Actions jobs.",
        ));
    }
    for (job_id, job) in jobs {
        let Some(job) = job.as_object() else {
            continue;
        };
        let Some(needs) = job.get("needs") else {
            continue;
        };
        let needs: Vec<&str> = match needs {
            serde_json::Value::String(need) => vec![need.as_str()],
            serde_json::Value::Array(needs) => needs
                .iter()
                .map(|need| {
                    need.as_str().ok_or(CiWorkflowIssue::Unknown(
                        "A workflow job needs list has a non-string entry.",
                    ))
                })
                .collect::<Result<_, _>>()?,
            _ => {
                return Err(CiWorkflowIssue::Unknown(
                    "A workflow job needs value has an unsupported shape.",
                ));
            }
        };
        if needs
            .iter()
            .any(|need| *need == job_id.as_str() || !jobs.contains_key(*need))
        {
            return Err(CiWorkflowIssue::Fail(
                "A workflow job needs an unknown job or depends on itself.",
            ));
        }
    }
    let mut job_ids = BTreeSet::new();
    for (job_id, job) in jobs {
        let Some(job) = job.as_object() else {
            return Err(CiWorkflowIssue::Unknown(
                "A workflow job has an unsupported YAML shape.",
            ));
        };
        job_ids.insert(job_id.clone());
        if let Some(permissions) = job.get("permissions") {
            inspect_ci_permissions(Some(permissions))?;
        }
        inspect_job_action_references(job)?;
        if !job.contains_key("uses") {
            match job.get("runs-on") {
                Some(serde_json::Value::String(runner)) if !runner.trim().is_empty() => {}
                Some(serde_json::Value::Array(runners))
                    if !runners.is_empty() && runners.iter().all(serde_json::Value::is_string) => {}
                None => {
                    return Err(CiWorkflowIssue::Fail(
                        "A workflow job has neither a reusable workflow nor a runner.",
                    ));
                }
                _ => {
                    return Err(CiWorkflowIssue::Unknown(
                        "A workflow job has an unsupported runs-on shape.",
                    ));
                }
            }
        }
        if !required_jobs.iter().any(|required| required == job_id) {
            continue;
        }
        if job.contains_key("uses") {
            return Err(CiWorkflowIssue::Unknown(
                "Required reusable workflows are not expanded by the local CI parser.",
            ));
        }
        if let Some(continue_on_error) = job.get("continue-on-error") {
            match continue_on_error.as_bool() {
                Some(true) => {
                    return Err(CiWorkflowIssue::Fail(
                        "A required CI job enables continue-on-error.",
                    ));
                }
                Some(false) => {}
                None => {
                    return Err(CiWorkflowIssue::Unknown(
                        "A required job uses a dynamic continue-on-error value.",
                    ));
                }
            }
        }
        if let Some(condition) = job.get("if") {
            let supported_always = condition.as_str().is_some_and(|condition| {
                matches!(condition.trim(), "always()" | "${{ always() }}")
            });
            if !supported_always {
                return Err(CiWorkflowIssue::Unknown(
                    "A required CI job has a conditional execution rule the parser cannot prove.",
                ));
            }
        }
        let Some(steps) = job.get("steps").and_then(serde_json::Value::as_array) else {
            return Err(CiWorkflowIssue::Unknown(
                "A required CI job has no supported steps list.",
            ));
        };
        if steps.is_empty() {
            return Err(CiWorkflowIssue::Fail("A required CI job has no steps."));
        }
        for step in steps {
            let Some(step) = step.as_object() else {
                return Err(CiWorkflowIssue::Unknown(
                    "A required CI job contains an unsupported step shape.",
                ));
            };
            if step.contains_key("if") {
                return Err(CiWorkflowIssue::Unknown(
                    "A required CI step has a conditional execution rule the parser cannot prove.",
                ));
            }
            if let Some(continue_on_error) = step.get("continue-on-error") {
                match continue_on_error.as_bool() {
                    Some(true) => {
                        return Err(CiWorkflowIssue::Fail(
                            "A required CI job has a continue-on-error step.",
                        ));
                    }
                    Some(false) => {}
                    None => {
                        return Err(CiWorkflowIssue::Unknown(
                            "A required CI step uses a dynamic continue-on-error value.",
                        ));
                    }
                }
            }
            let has_run = step.get("run").is_some_and(serde_json::Value::is_string);
            let has_uses = step.get("uses").is_some_and(serde_json::Value::is_string);
            if has_run == has_uses {
                return Err(CiWorkflowIssue::Unknown(
                    "A required CI step must have exactly one run or uses field.",
                ));
            }
        }
    }
    Ok(CiWorkflowFacts {
        push: triggers.push,
        pull_request: triggers.pull_request,
        job_ids,
    })
}

struct CiWorkflowTriggers<'a> {
    push: bool,
    pull_request: bool,
    push_config: Option<&'a serde_json::Value>,
    pull_request_config: Option<&'a serde_json::Value>,
}

fn parse_ci_triggers(on: &serde_json::Value) -> Result<CiWorkflowTriggers<'_>, CiWorkflowIssue> {
    let mut push = false;
    let mut pull_request = false;
    let mut push_config = None;
    let mut pull_request_config = None;
    match on {
        serde_json::Value::String(event) => {
            push = event == "push";
            pull_request = event == "pull_request";
        }
        serde_json::Value::Array(events) => {
            for event in events {
                let Some(event) = event.as_str() else {
                    return Err(CiWorkflowIssue::Unknown(
                        "Workflow event names must be YAML strings.",
                    ));
                };
                push |= event == "push";
                pull_request |= event == "pull_request";
            }
        }
        serde_json::Value::Object(events) => {
            push_config = events.get("push");
            push = push_config.is_some();
            pull_request_config = events.get("pull_request");
            pull_request = pull_request_config.is_some();
            for event in ["push", "pull_request"] {
                if let Some(config) = events.get(event)
                    && !config.is_null()
                    && !config.is_object()
                {
                    return Err(CiWorkflowIssue::Unknown(
                        "A supported workflow event has an unsupported filter shape.",
                    ));
                }
            }
        }
        _ => {
            return Err(CiWorkflowIssue::Unknown(
                "Workflow `on` must be a supported event mapping or list.",
            ));
        }
    }
    Ok(CiWorkflowTriggers {
        push,
        pull_request,
        push_config,
        pull_request_config,
    })
}

fn push_trigger_is_tag_only(config: &serde_json::Value) -> bool {
    config.as_object().is_some_and(|config| {
        (config.contains_key("tags") || config.contains_key("tags-ignore"))
            && !config.contains_key("branches")
            && !config.contains_key("branches-ignore")
    })
}

fn inspect_ci_permissions(permissions: Option<&serde_json::Value>) -> Result<(), CiWorkflowIssue> {
    let Some(permissions) = permissions else {
        return Err(CiWorkflowIssue::Unknown(
            "Workflow token permissions are implicit and cannot be verified locally.",
        ));
    };
    match permissions {
        serde_json::Value::String(value) if value == "read-all" => Ok(()),
        serde_json::Value::String(value) if value == "write-all" => Err(CiWorkflowIssue::Fail(
            "Workflow grants write-all token permissions.",
        )),
        serde_json::Value::Object(permissions) => {
            for (permission, value) in permissions {
                if !REGISTERED_GITHUB_PERMISSIONS.contains(&permission.as_str()) {
                    return Err(CiWorkflowIssue::Unknown(
                        "Workflow uses an unregistered GitHub token permission.",
                    ));
                }
                match value.as_str() {
                    Some("read" | "none") => {}
                    Some("write") => {
                        return Err(CiWorkflowIssue::Fail(
                            "Workflow grants a write token permission.",
                        ));
                    }
                    _ => {
                        return Err(CiWorkflowIssue::Unknown(
                            "Workflow token permission has an unsupported value.",
                        ));
                    }
                }
            }
            Ok(())
        }
        _ => Err(CiWorkflowIssue::Unknown(
            "Workflow token permissions have an unsupported YAML shape.",
        )),
    }
}

fn inspect_job_action_references(
    job: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), CiWorkflowIssue> {
    if let Some(uses) = job.get("uses") {
        inspect_pinned_action_reference(uses)?;
    }
    if let Some(steps) = job.get("steps") {
        let Some(steps) = steps.as_array() else {
            return Err(CiWorkflowIssue::Unknown(
                "A workflow job has an unsupported steps shape.",
            ));
        };
        for step in steps {
            let Some(step) = step.as_object() else {
                return Err(CiWorkflowIssue::Unknown(
                    "A workflow step has an unsupported YAML shape.",
                ));
            };
            if let Some(uses) = step.get("uses") {
                inspect_pinned_action_reference(uses)?;
            }
        }
    }
    Ok(())
}

fn inspect_pinned_action_reference(uses: &serde_json::Value) -> Result<(), CiWorkflowIssue> {
    let Some(uses) = uses.as_str() else {
        return Err(CiWorkflowIssue::Unknown(
            "An Actions `uses` reference is not a YAML string.",
        ));
    };
    if uses.starts_with("./") {
        return Ok(());
    }
    let pinned = if let Some(docker_image) = uses.strip_prefix("docker://") {
        docker_image
            .rsplit_once("@sha256:")
            .is_some_and(|(_, digest)| {
                digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit())
            })
    } else {
        uses.rsplit_once('@').is_some_and(|(_, revision)| {
            revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit())
        })
    };
    if pinned {
        Ok(())
    } else {
        Err(CiWorkflowIssue::Fail(
            "An external GitHub Action is not pinned to a full commit SHA or image digest.",
        ))
    }
}

async fn inspect_effective_commit_hook(
    repository_root: &Path,
    git: &ApprovedRepositoryTool,
    environment: &ApprovedEnv,
    process: &dyn ProcessPort,
) -> AppResult<&'static str> {
    let unknown =
        "Git's effective commit-msg hook could not be verified; only LocalCheck is claimed.";
    let cwd = ApprovedRoot::from_absolute_path(repository_root.to_path_buf())
        .map_err(AppError::Validation)?;
    let spec = ProcessSpec::new(
        ApprovedExecutable::from_absolute_path(git.executable.path().to_path_buf())
            .map_err(AppError::Validation)?,
        ToolId::Git,
        ["rev-parse", "--git-path", "hooks/commit-msg"]
            .into_iter()
            .map(OsString::from)
            .collect(),
        cwd,
        ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?,
        Duration::from_secs(5),
        4096,
        ProcessPermission::ReadOnlyCheck,
        CancellationToken::new(),
    )
    .map_err(AppError::Validation)?
    .with_approved_executable_fingerprint(git.fingerprint);
    let output = match process.run(spec).await {
        Ok(output) if output.exit_code() == Some(0) => output,
        Err(AppError::Cancelled) => return Err(AppError::Cancelled),
        Ok(_) | Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
            return Ok(unknown);
        }
        Err(error) => return Err(error),
    };
    let Some(path_bytes) = output.stdout().strip_suffix(b"\n") else {
        return Ok(unknown);
    };
    let path_bytes = path_bytes.strip_suffix(b"\r").unwrap_or(path_bytes);
    let Ok(path_text) = std::str::from_utf8(path_bytes) else {
        return Ok(unknown);
    };
    if path_text.is_empty() || path_text.chars().any(char::is_control) {
        return Ok(unknown);
    }
    let configured_path = PathBuf::from(path_text);
    let unresolved_path = if configured_path.is_absolute() {
        configured_path
    } else {
        repository_root.join(configured_path)
    };
    let Ok(unresolved_metadata) = std::fs::symlink_metadata(&unresolved_path) else {
        return Ok("Git resolved no readable commit-msg hook file; only LocalCheck is claimed.");
    };
    if unresolved_metadata.file_type().is_symlink() {
        return Ok("The effective commit-msg hook is a symlink; only LocalCheck is claimed.");
    }
    let Ok(hook_path) = std::fs::canonicalize(&unresolved_path) else {
        return Ok("Git resolved no readable commit-msg hook file; only LocalCheck is claimed.");
    };
    if !hook_path.starts_with(repository_root) {
        return Ok(
            "The effective commit-msg hook is outside the approved repository; only LocalCheck is claimed.",
        );
    }
    let Ok(metadata) = std::fs::symlink_metadata(&hook_path) else {
        return Ok(unknown);
    };
    if !metadata.file_type().is_file() || metadata.len() > MAX_COMMIT_HOOK_BYTES as u64 {
        return Ok(
            "The effective commit-msg hook is not a bounded regular file; only LocalCheck is claimed.",
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Ok(
                "The effective commit-msg hook lacks executable permission; only LocalCheck is claimed.",
            );
        }
    }
    let Ok(contents) = std::fs::read(&hook_path) else {
        return Ok(unknown);
    };
    if contents.len() > MAX_COMMIT_HOOK_BYTES {
        return Ok(
            "The effective commit-msg hook exceeds the inspection limit; only LocalCheck is claimed.",
        );
    }
    let observed_hash = Sha256::digest(&contents);
    let Ok(contents_after_read) = std::fs::read(&hook_path) else {
        return Ok(unknown);
    };
    if Sha256::digest(&contents_after_read) != observed_hash {
        return Ok(
            "The effective commit-msg hook changed during inspection; only LocalCheck is claimed.",
        );
    }
    Ok(
        "Git resolved and content-hashed a local commit-msg hook read-only; its argv/driver identity and invocation are unproven, so only LocalCheck is claimed.",
    )
}

fn observation(
    status: CheckStatus,
    enforcement: Option<Enforcement>,
    evidence: Vec<CheckEvidence>,
) -> AppResult<CheckObservation> {
    CheckObservation::new(status, enforcement, evidence).map_err(AppError::Validation)
}

fn readme_has_required_sections(source: &str, required_headings: &[String]) -> bool {
    use markdown::mdast::Node;

    if required_headings.is_empty() {
        return false;
    }
    let Ok(Node::Root(root)) = markdown::to_mdast(source, &markdown::ParseOptions::default())
    else {
        return false;
    };
    let parsed_headings: Vec<_> = root
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            Node::Heading(heading) => Some((index, heading.depth, markdown_text(node))),
            _ => None,
        })
        .collect();

    required_headings.iter().all(|required| {
        let target = normalize_heading(required);
        if target.is_empty() {
            return false;
        }
        let Some((heading_index, depth, _)) = parsed_headings
            .iter()
            .find(|(_, _, heading)| normalize_heading(heading) == target)
        else {
            return false;
        };
        root.children
            .iter()
            .skip(heading_index + 1)
            .take_while(|node| match node {
                Node::Heading(next) => next.depth > *depth,
                _ => true,
            })
            .any(markdown_node_has_content)
    })
}

fn markdown_node_has_content(node: &markdown::mdast::Node) -> bool {
    if matches!(node, markdown::mdast::Node::Heading(_)) {
        return false;
    }
    !normalize_heading(&markdown_text(node)).is_empty()
}

fn markdown_text(node: &markdown::mdast::Node) -> String {
    use markdown::mdast::Node;

    fn append(node: &Node, output: &mut String) {
        match node {
            Node::Text(text) => output.push_str(&text.value),
            Node::InlineCode(code) => output.push_str(&code.value),
            Node::Code(code) => output.push_str(&code.value),
            Node::Html(html) => output.push_str(&html.value),
            Node::Image(image) => output.push_str(&image.alt),
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        append(child, output);
                    }
                }
            }
        }
    }

    let mut text = String::new();
    append(node, &mut text);
    text
}

fn normalize_heading(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl FileSystemPort for LocalFileSystem {
    fn read_bundle_directory(&self, root: &Path) -> Result<BundleFiles, Vec<Diagnostic>> {
        let inventory = self.inspect_bundle(root)?;
        read_bundle_bytes(root, &inventory)
    }
}

/// Reads a `.jskill` archive fully before trusting it: central validation,
/// then stored-entry extraction with checksums and validated totals. Shares
/// validation with directory import so both accept exact canonical bytes.
pub fn read_bundle(bytes: &[u8]) -> Result<(ValidatedInventory, BundleFiles), Vec<Diagnostic>> {
    let inventory = validate_archive_entries(bytes)?;
    let files = extract_archive_files(bytes, &inventory)?;
    Ok((inventory, files))
}

/// Addresses a content blob by canonical hash: a two-hex prefix directory
/// keeps single directories small, mirroring the documented blob layout.
pub fn blob_path(blobs_root: &Path, hash: &ContentHash) -> PathBuf {
    let hex = hash.as_str();
    blobs_root.join(&hex[..2]).join(format!("{hex}.bundle"))
}

/// Stores archive bytes under their canonical hash. Identical bytes are
/// idempotent; different bytes under the same hash fail instead of
/// overwriting published content.
pub fn store_blob_bytes(
    blobs_root: &Path,
    hash: &ContentHash,
    archive: &[u8],
) -> Result<PathBuf, Vec<Diagnostic>> {
    read_verified_blob_archive(hash, archive)?;
    let path = blob_path(blobs_root, hash);
    ensure_private_directory(blobs_root)?;
    ensure_private_directory(&blobs_root.join(&hash.as_str()[..2]))?;
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => return Err(blob_io("Bundle blob path is not a regular file.")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(blob_io("Bundle blob path is not readable.")),
    }
    match std::fs::read(&path) {
        Ok(existing) if existing == archive => {
            restrict_file(&path).map_err(|_| blob_io("Bundle blob is not private."))?;
            return Ok(path);
        }
        Ok(_) => {
            return Err(vec![Diagnostic::error(
                "bundle.blob.conflict",
                "Different bundle bytes share one hash.",
            )]);
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(blob_io("Bundle blob is not readable."));
        }
        Err(_) => {}
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = match options.open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|_| blob_io("Bundle blob path is not readable."))?;
            if !metadata.file_type().is_file() {
                return Err(blob_io("Bundle blob path is not a regular file."));
            }
            let existing =
                std::fs::read(&path).map_err(|_| blob_io("Bundle blob is not readable."))?;
            if existing == archive {
                restrict_file(&path).map_err(|_| blob_io("Bundle blob is not private."))?;
                return Ok(path);
            }
            return Err(vec![Diagnostic::error(
                "bundle.blob.conflict",
                "Different bundle bytes share one hash.",
            )]);
        }
        Err(_) => return Err(blob_io("Bundle blob is not writable.")),
    };
    let result = file
        .write_all(archive)
        .and_then(|()| file.sync_all())
        .and_then(|()| restrict_file(&path));
    drop(file);
    if result.is_err() {
        let _ = std::fs::remove_file(&path);
        return Err(blob_io("Bundle blob could not be durably stored."));
    }
    Ok(path)
}

fn ensure_private_directory(path: &Path) -> Result<(), Vec<Diagnostic>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => {}
        Ok(_) => return Err(blob_io("Bundle blob path is not a regular directory.")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(path)
                .map_err(|_| blob_io("Bundle blob directory is not writable."))?;
        }
        Err(_) => return Err(blob_io("Bundle blob directory is not readable.")),
    }
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
    if !metadata.file_type().is_dir() {
        return Err(blob_io("Bundle blob path is not a regular directory."));
    }
    restrict_directory(path).map_err(|_| blob_io("Bundle blob directory is not private."))
}

fn read_verified_blob_archive(
    expected: &ContentHash,
    archive: &[u8],
) -> Result<BundleFiles, Vec<Diagnostic>> {
    let (inventory, files) = read_bundle(archive)?;
    let actual = hash_bundle(&inventory, &files).map_err(|_| {
        vec![Diagnostic::error(
            "bundle.blob.unverifiable",
            "Bundle blob cannot be hashed.",
        )]
    })?;
    if &actual != expected {
        return Err(vec![Diagnostic::error(
            "bundle.blob.checksum_mismatch",
            "Bundle blob bytes do not match their hash.",
        )]);
    }
    Ok(files)
}

/// Verifies blob bytes end to end: the archive must parse, its canonical
/// hash must match, and the recovered files return for callers that keep
/// going. Anything else fails closed.
pub fn verify_blob_bytes(
    blobs_root: &Path,
    expected: &ContentHash,
) -> Result<BundleFiles, Vec<Diagnostic>> {
    let path = blob_path(blobs_root, expected);
    let prefix = path
        .parent()
        .ok_or_else(|| blob_io("Bundle blob path is invalid."))?;
    for directory in [blobs_root, prefix] {
        let metadata = std::fs::symlink_metadata(directory)
            .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
        if !metadata.file_type().is_dir() {
            return Err(blob_io("Bundle blob path is not a regular directory."));
        }
    }
    let metadata =
        std::fs::symlink_metadata(&path).map_err(|_| blob_io("Bundle blob is not readable."))?;
    if !metadata.file_type().is_file() {
        return Err(blob_io("Bundle blob path is not a regular file."));
    }
    let bytes = std::fs::read(path).map_err(|_| blob_io("Bundle blob is not readable."))?;
    read_verified_blob_archive(expected, &bytes)
}

/// Lists only canonical content-addressed blob files. Unknown filesystem
/// entries are ignored; symlinks/reparse-like entries at the enumerated
/// levels fail closed instead of being followed.
pub fn list_blob_hashes(blobs_root: &Path) -> Result<Vec<ContentHash>, Vec<Diagnostic>> {
    let root_meta = match std::fs::symlink_metadata(blobs_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(blob_io("Bundle blob directory is not readable.")),
    };
    if !root_meta.file_type().is_dir() || root_meta.file_type().is_symlink() {
        return Err(blob_io("Bundle blob directory is not a regular directory."));
    }
    let mut hashes = Vec::new();
    let prefixes = std::fs::read_dir(blobs_root)
        .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
    for prefix in prefixes {
        let prefix = prefix.map_err(|_| blob_io("Bundle blob directory is not readable."))?;
        let prefix_name = prefix.file_name();
        let Some(prefix_name) = prefix_name.to_str() else {
            continue;
        };
        if prefix_name.len() != 2
            || !prefix_name
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            continue;
        }
        let prefix_meta = std::fs::symlink_metadata(prefix.path())
            .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
        if prefix_meta.file_type().is_symlink() {
            return Err(blob_io("Bundle blob path contains a symlink."));
        }
        if !prefix_meta.file_type().is_dir() {
            continue;
        }
        let entries = std::fs::read_dir(prefix.path())
            .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
        for entry in entries {
            let entry = entry.map_err(|_| blob_io("Bundle blob directory is not readable."))?;
            let metadata = std::fs::symlink_metadata(entry.path())
                .map_err(|_| blob_io("Bundle blob is not readable."))?;
            if metadata.file_type().is_symlink() {
                return Err(blob_io("Bundle blob path contains a symlink."));
            }
            if !metadata.file_type().is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(hex) = name.strip_suffix(".bundle") else {
                continue;
            };
            let Ok(hash) = ContentHash::parse_hex(hex) else {
                continue;
            };
            if &hex[..2] == prefix_name {
                hashes.push(hash);
            }
        }
    }
    hashes.sort();
    hashes.dedup();
    Ok(hashes)
}

fn blob_io(message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error("bundle.blob.io", message)]
}

/// Unpacks a validated archive into private staging: nothing is written
/// before the full archive validates, the staging directory must not exist
/// (a pre-existing destination is never touched), and any failure removes
/// what this call created. On Unix, staging dirs are 0700 and files 0600;
/// on Windows the directory inherits the user's ACL. Staging is never the
/// library: install flows revalidate from staging before touching targets.
pub fn unpack_bundle_to_staging(
    bytes: &[u8],
    staging: &Path,
) -> Result<ValidatedInventory, Vec<Diagnostic>> {
    let (inventory, files) = read_bundle(bytes)?;
    if std::fs::symlink_metadata(staging).is_ok() {
        return Err(vec![Diagnostic::error(
            "bundle.staging.exists",
            "Staging directory already exists.",
        )]);
    }
    let failed = |message: &'static str| {
        let _ = std::fs::remove_dir_all(staging);
        vec![Diagnostic::error("bundle.staging.unwritable", message)]
    };
    if std::fs::create_dir_all(staging).is_err() {
        return Err(failed("Staging directory is not writable."));
    }
    restrict_directory(staging).map_err(|_| failed("Staging directory is not private."))?;
    for file in inventory.files() {
        let content = files.get(file.path()).ok_or_else(|| {
            let _ = std::fs::remove_dir_all(staging);
            vec![Diagnostic::error(
                "bundle.archive.mismatch",
                "Archive extraction disagrees with its inventory.",
            )]
        })?;
        let mut path = staging.to_path_buf();
        for component in file.path().as_str().split('/') {
            path.push(component);
        }
        if let Some(parent) = path.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            return Err(failed("Staging directory is not writable."));
        }
        if std::fs::write(&path, content).is_err() {
            return Err(failed("Staging file is not writable."));
        }
        if restrict_file(&path).is_err() {
            return Err(failed("Staging file is not private."));
        }
    }
    Ok(inventory)
}

#[cfg(unix)]
fn restrict_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn restrict_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn restrict_file(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_file(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Reads the raw bytes of every validated file under the canonical root, in
/// inventory order. Reads are capped by the validated totals, so a file that
/// grows between inspection and hashing fails instead of exhausting memory;
/// kind is rechecked at read time, but staging flows must revalidate before
/// writing anywhere. Export and hashing share this single byte source.
pub fn read_bundle_bytes(
    root: &Path,
    inventory: &ValidatedInventory,
) -> Result<BTreeMap<PortablePath, Vec<u8>>, Vec<Diagnostic>> {
    let canonical = root
        .canonicalize()
        .map_err(|_| invalid_root("Bundle root is not accessible."))?;
    let mut budget = inventory.total_uncompressed_bytes();
    let mut files = BTreeMap::new();
    for file in inventory.files() {
        let mut path = canonical.clone();
        for component in file.path().as_str().split('/') {
            path.push(component);
        }
        let kind = std::fs::symlink_metadata(&path)
            .map_err(|_| unreadable())?
            .file_type();
        if !kind.is_file() {
            return Err(vec![Diagnostic::error(
                "bundle.entry.kind.unsupported",
                "Bundle entries must be regular files.",
            )]);
        }
        let handle = std::fs::File::open(&path).map_err(|_| unreadable())?;
        let mut content = Vec::new();
        handle
            .take(budget.saturating_add(1))
            .read_to_end(&mut content)
            .map_err(|_| unreadable())?;
        if content.len() as u64 > budget {
            return Err(vec![Diagnostic::error(
                "bundle.file.changed",
                "Bundle file changed during hashing.",
            )]);
        }
        budget -= content.len() as u64;
        files.insert(file.path().clone(), content);
    }
    Ok(files)
}

fn unreadable() -> Vec<Diagnostic> {
    vec![Diagnostic::error(
        "bundle.file.unreadable",
        "Bundle file is not readable.",
    )]
}

fn invalid_root(message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error("bundle.root.invalid", message)]
}

/// Walks a bundle tree without following symlinks. Every entry is classified
/// with `symlink_metadata`: links, sockets and other non-regular files are
/// rejected, directories are descended but never listed, and each listed
/// path must stay under the canonical root. Hard links with more than one
/// name are rejected on Unix via link count; Windows reparse points that do
/// not present as links need OS APIs and arrive with the Windows port.
pub fn inspect_bundle_tree(root: &Path) -> Result<Vec<BundleEntry>, Vec<Diagnostic>> {
    let canonical = root
        .canonicalize()
        .map_err(|_| invalid_root("Bundle root is not accessible."))?;
    if !canonical.is_dir() {
        return Err(invalid_root("Bundle root is not a directory."));
    }
    let mut entries = Vec::new();
    let mut stack = vec![canonical.clone()];
    while let Some(dir) = stack.pop() {
        let read = std::fs::read_dir(&dir)
            .map_err(|_| invalid_root("Bundle directory is not readable."))?;
        for child in read {
            let child =
                child.map_err(|_| invalid_root("Bundle directory entry is not readable."))?;
            let path = child.path();
            if !path.starts_with(&canonical) {
                return Err(vec![Diagnostic::error(
                    "bundle.path.escape",
                    "Bundle entry escapes its root.",
                )]);
            }
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|_| invalid_root("Bundle entry metadata is not readable."))?;
            let file_type = metadata.file_type();
            if file_type.is_symlink() || (!file_type.is_dir() && !file_type.is_file()) {
                return Err(vec![Diagnostic::error(
                    "bundle.entry.kind.unsupported",
                    "Bundle entries must be regular files.",
                )]);
            }
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            reject_hard_link(&metadata)?;
            let relative = path
                .strip_prefix(&canonical)
                .map_err(|_| {
                    vec![Diagnostic::error(
                        "bundle.path.escape",
                        "Bundle entry escapes its root.",
                    )]
                })?
                .to_path_buf();
            entries.push(tree_entry(&relative, metadata.len())?);
        }
    }
    Ok(entries)
}

fn tree_entry(relative: &Path, size: u64) -> Result<BundleEntry, Vec<Diagnostic>> {
    let mut name = String::new();
    for component in relative.components() {
        let text = component.as_os_str().to_str().ok_or_else(|| {
            vec![Diagnostic::error(
                "bundle.path.invalid_utf8",
                "Bundle path is not valid UTF-8.",
            )]
        })?;
        if !name.is_empty() {
            name.push('/');
        }
        name.push_str(text);
    }
    bundle_entry_from_path(&name, EntryKind::RegularFile, size, size)
}

#[cfg(unix)]
fn reject_hard_link(metadata: &std::fs::Metadata) -> Result<(), Vec<Diagnostic>> {
    use std::os::unix::fs::MetadataExt;
    if metadata.nlink() > 1 {
        return Err(vec![Diagnostic::error(
            "bundle.entry.kind.unsupported",
            "Bundle entries must be regular files.",
        )]);
    }
    Ok(())
}

#[cfg(not(unix))]
fn reject_hard_link(_metadata: &std::fs::Metadata) -> Result<(), Vec<Diagnostic>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jameskills_core::domain::hash_bundle;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static BUNDLE_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempBundle {
        root: PathBuf,
    }

    impl TempBundle {
        fn new(files: &[(&str, &[u8])]) -> Self {
            let id = BUNDLE_COUNTER.fetch_add(1, Ordering::SeqCst);
            let root =
                std::env::temp_dir().join(format!("jameskills-bundle-{}-{id}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            for (name, content) in files {
                let mut path = root.clone();
                for component in name.split('/') {
                    path.push(component);
                }
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, content).unwrap();
            }
            Self { root }
        }

        fn remove(&self, name: &str) {
            let mut path = self.root.clone();
            for component in name.split('/') {
                path.push(component);
            }
            std::fs::remove_file(&path).unwrap();
        }
    }

    impl Drop for TempBundle {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn reads_validated_tree_and_hashes_through_domain() {
        let bundle =
            TempBundle::new(&[("SKILL.md", b"# Skill\n"), ("docs/guide.md", b"# Guide\n")]);
        let system = LocalFileSystem;
        let inventory = system.inspect_bundle(&bundle.root).unwrap();
        let bytes = read_bundle_bytes(&bundle.root, &inventory).unwrap();
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes.values().map(Vec::len).sum::<usize>(), 16);
        assert!(hash_bundle(&inventory, &bytes).is_ok());
    }

    #[test]
    fn rejects_file_grown_after_inspection() {
        let bundle = TempBundle::new(&[("SKILL.md", b"# Skill\n")]);
        let entries = vec![BundleEntry::new(
            PortablePath::new("SKILL.md".to_owned()).unwrap(),
            EntryKind::RegularFile,
            4,
            4,
        )];
        let inventory = validate_bundle_inventory(&entries).unwrap();
        let result = read_bundle_bytes(&bundle.root, &inventory);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err()[0].code(), "bundle.file.changed");
    }

    #[test]
    fn rejects_file_removed_before_reading() {
        let bundle = TempBundle::new(&[("SKILL.md", b"# Skill\n")]);
        let system = LocalFileSystem;
        let inventory = system.inspect_bundle(&bundle.root).unwrap();
        bundle.remove("SKILL.md");
        let result = read_bundle_bytes(&bundle.root, &inventory);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err()[0].code(), "bundle.file.unreadable");
    }
}
