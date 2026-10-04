use crate::platform::{
    PlatformFacts, ToolCandidate, ToolCandidateKind, ToolProfile, find_tool_candidates,
    load_tool_profiles, probe_registered_tool_version,
};
use jameskills_core::{
    AppError, AppResult, Diagnostic,
    application::policy::PolicyCheckProvider,
    domain::{
        BundleEntry, Check, ContentHash, EntryKind, PortablePath, Requirement, ToolId,
        ValidatedInventory,
        guidance::{ToolAvailability, ToolVersionStatus},
        hash_bundle,
        policy::{CheckEvidence, CheckObservation, CheckStatus, Enforcement},
        validate_bundle_inventory,
    },
    ports::ClockPort,
    ports::filesystem::{
        BundleFiles, FileSystemPort, bundle_entry_from_path, extract_archive_files,
        validate_archive_entries,
    },
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken, ExecutableFingerprint,
        ProcessPermission, ProcessPort, ProcessSpec,
    },
};
use std::collections::BTreeMap;
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
const GITLEAKS_CONFIG_PLACEHOLDER: &str = "{APP_GITLEAKS_CONFIG}";
const GITLEAKS_DEFAULT_CONFIG: &str = "[extend]\nuseDefault = true\n";
static NEXT_GITLEAKS_CONFIG_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_COMMIT_MESSAGE_ID: AtomicU64 = AtomicU64::new(0);

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
        git: &ApprovedRepositoryTool,
        commitlint: &ApprovedRepositoryTool,
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
        let Some(commitlint_candidate) =
            candidate_for_approved_tool(commitlint_profile, commitlint, platform)
        else {
            return report(
                CheckStatus::Blocked,
                None,
                "The approved Commitlint executable is not a registered candidate.",
            );
        };
        if git_candidate.kind() != ToolCandidateKind::NativeExecutable
            || commitlint_candidate.kind() != ToolCandidateKind::NativeExecutable
        {
            return report(
                CheckStatus::Blocked,
                None,
                "Git and Commitlint command shims are not executed by this driver.",
            );
        }
        let environment =
            ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?;
        for (profile, candidate, tool) in [
            (git_profile, &git_candidate, git),
            (commitlint_profile, &commitlint_candidate, commitlint),
        ] {
            let version = probe_registered_tool_version(
                profile,
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
                    "Git or Commitlint version is outside the verified app-owned profile.",
                );
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
        let commitlint_args = ["--default-config", "--config"]
            .into_iter()
            .map(OsString::from)
            .chain([message_file.config_path.as_os_str().to_os_string()])
            .chain([OsString::from("--edit")])
            .chain([message_file.path.as_os_str().to_os_string()])
            .chain([OsString::from("--quiet"), OsString::from("--color=false")])
            .collect();
        let commitlint_spec = ProcessSpec::new(
            ApprovedExecutable::from_absolute_path(commitlint.executable.path().to_path_buf())
                .map_err(AppError::Validation)?,
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
        .with_approved_executable_fingerprint(commitlint.fingerprint);
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
            Some(0) => report(
                CheckStatus::Pass,
                Some(Enforcement::LocalCheck),
                "Commitlint 21.2.2 accepted the current commit using built-in Conventional Commits rules.",
            ),
            Some(1) => report(
                CheckStatus::Fail,
                Some(Enforcement::LocalCheck),
                "Commitlint 21.2.2 rejected the current commit; message details are withheld.",
            ),
            _ => report(
                CheckStatus::Blocked,
                None,
                "Commitlint returned an unregistered exit status; output is withheld.",
            ),
        }
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
}

/// Per-repository provider. It stores only approved executable identities and
/// returns Unknown for check kinds that do not have an implemented driver.
pub struct RepositoryPolicyCheckProvider {
    root: PathBuf,
    git: Option<ApprovedRepositoryTool>,
    gitleaks: Option<ApprovedRepositoryTool>,
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
            environment,
            process,
            clock,
            environment_fingerprint,
        }
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
            _ => Ok(CheckObservation::unknown()),
        }
    }
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
