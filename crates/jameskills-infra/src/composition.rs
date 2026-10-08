use crate::sqlite::SqliteStore;
use crate::{
    fs::{
        ApprovedCommitlint, ApprovedNodeNpm, ApprovedRepositoryTool, RepositoryPolicyCheckProvider,
    },
    fs::{GitleaksImportScanner, LocalFileSystem},
    platform::{PlatformFacts, UserDirectories},
    platform::{ToolCandidateKind, find_tool_candidates, load_tool_profiles},
    process::{SystemProcessPort, fingerprint_executable},
};
use chrono::{SecondsFormat, Utc};
use jameskills_core::application::GuidanceFactsProvider;
use jameskills_core::{
    AppError, AppResult, Diagnostic,
    application::{
        GuidanceService, LibraryService, PolicyService,
        policy::{
            CheckContext, PolicyCheckProvider, RepositoryBindingEvaluation,
            RepositoryBindingEvaluationRequest, RepositoryBindingStaleReason,
        },
    },
    domain::{
        GuidanceFactObservation, GuidanceFacts, RepositoryBinding, RepositoryProfile, Requirement,
        RevisionId, SkillId,
        policy::{ApplicabilityFact, CheckEvidence, CheckObservation},
    },
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, ExecutableFingerprint, ProcessPort,
        RepositoryState,
    },
    ports::{ClockPort, ImportScanPort, StoragePort},
};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, path::Path, sync::Arc, time::Instant};

/// Process-local monotonic reference and UTC wall clock.
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockPort for SystemClock {
    fn now_utc(&self) -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    fn monotonic_ms(&self) -> u64 {
        self.origin.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
}

/// Shared process runtime providers currently available to application hosts.
#[derive(Clone)]
pub struct RuntimeServices {
    facts: PlatformFacts,
    directories: UserDirectories,
    clock: Arc<SystemClock>,
    library: Arc<LibraryService>,
    policy: Arc<PolicyService>,
    guidance: Arc<GuidanceService>,
}

struct RepositoryBindingObservationRequest<'a> {
    skill_id: SkillId,
    suite_revision: RevisionId,
    repository: &'a Path,
    profile: RepositoryProfile,
    strict: bool,
    git: &'a ApprovedRepositoryTool,
}

impl RuntimeServices {
    pub fn facts(&self) -> &PlatformFacts {
        &self.facts
    }

    pub fn directories(&self) -> &UserDirectories {
        &self.directories
    }

    pub fn clock(&self) -> &dyn ClockPort {
        self.clock.as_ref()
    }

    pub fn library(&self) -> &LibraryService {
        self.library.as_ref()
    }

    pub fn policy(&self) -> &PolicyService {
        self.policy.as_ref()
    }

    pub fn guidance(&self) -> &GuidanceService {
        self.guidance.as_ref()
    }

    /// Build a check service for one explicitly selected repository. Missing
    /// tool approvals remain absent, so their checks fail closed without spawn.
    pub fn policy_for_repository(
        &self,
        repository: &Path,
        tools: RepositoryCheckTools,
    ) -> AppResult<Arc<PolicyService>> {
        let canonical_root = std::fs::canonicalize(repository).map_err(|_| AppError::NotFound)?;
        if !canonical_root.is_dir() {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "policy.repository.not_directory",
                "Selected repository root is not a directory.",
            )]));
        }
        let root = ApprovedRoot::from_absolute_path(canonical_root.clone())
            .map_err(AppError::Validation)?;
        let environment = approved_repository_check_environment()?;
        let mut digest = Sha256::new();
        digest.update(b"JAMESKILLS-REPOSITORY-CHECK-V1\0");
        digest.update(canonical_root.as_os_str().to_string_lossy().as_bytes());
        for (key, value) in environment.entries() {
            digest.update((key.len() as u64).to_be_bytes());
            digest.update(key.to_string_lossy().as_bytes());
            digest.update((value.len() as u64).to_be_bytes());
            digest.update(value.to_string_lossy().as_bytes());
        }
        let environment_fingerprint = format!("sha256:{:x}", digest.finalize());
        let mut provider = RepositoryPolicyCheckProvider::new(
            root,
            tools.git,
            tools.gitleaks,
            environment,
            Arc::new(SystemProcessPort),
            self.clock.clone(),
            environment_fingerprint,
        );
        if let Some(commitlint) = tools.commitlint {
            provider = provider.with_commitlint(commitlint);
        }
        if let Some(gh) = tools.github {
            provider = provider.with_github_cli(gh);
        }
        if let Some(cargo) = tools.cargo {
            provider = provider.with_cargo(cargo);
        }
        if let Some(node_npm) = tools.node_npm {
            provider = provider.with_node_npm(node_npm);
        }
        Ok(Arc::new(PolicyService::new(
            Arc::new(provider),
            self.clock.clone(),
        )))
    }

    /// Observe stack facts from bounded project manifests. The caller's profile
    /// is only a compatibility request; it never supplies the observed fact.
    pub fn repository_check_context(
        &self,
        repository: &Path,
        requested_profile: &str,
    ) -> AppResult<CheckContext> {
        if !matches!(requested_profile, "rust" | "node" | "generic") {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "policy.profile.invalid",
                "Requested repository profile is not registered.",
            )]));
        }
        let canonical_root = std::fs::canonicalize(repository).map_err(|_| AppError::NotFound)?;
        let root = ApprovedRoot::from_absolute_path(canonical_root.clone())
            .map_err(AppError::Validation)?;
        let facts = crate::platform::inspect_project_manifests(&root)?;
        let (stack_value, compatible) = match facts.stack() {
            crate::platform::ProjectStack::Rust => (Some("rust"), requested_profile != "node"),
            crate::platform::ProjectStack::Node => (Some("node"), requested_profile != "rust"),
            crate::platform::ProjectStack::RustAndNode => (Some("rust-node"), true),
            crate::platform::ProjectStack::Generic => {
                (Some("generic"), requested_profile == "generic")
            }
            crate::platform::ProjectStack::Unknown => (None, true),
        };
        if !compatible {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "policy.profile.stack_mismatch",
                "Requested repository profile does not match the observed project stack.",
            )]));
        }
        let Some(stack_value) = stack_value else {
            return Ok(CheckContext::default());
        };
        let mut digest = Sha256::new();
        digest.update(b"JAMESKILLS-REPOSITORY-STACK-V1\0");
        digest.update(canonical_root.as_os_str().to_string_lossy().as_bytes());
        digest.update(stack_value.as_bytes());
        let environment_fingerprint = format!("sha256:{:x}", digest.finalize());
        let observed_at = self.clock.now_utc();
        let expires_at = Some(self.clock.monotonic_ms().saturating_add(30_000));
        let evidence = CheckEvidence::new(
            "policy.project.stack",
            &observed_at,
            None,
            &environment_fingerprint,
            "Project stack was observed from bounded manifests.",
            expires_at,
        )
        .map_err(AppError::Validation)?;
        CheckContext::default()
            .with_fact(ApplicabilityFact::Stack, stack_value, evidence)
            .map_err(AppError::Validation)
    }

    /// Captures the current selected repository using an explicitly approved
    /// Git executable. The path, HEAD and environment are bound into a local
    /// fingerprint; no candidate is discovered or spawned implicitly.
    pub async fn observe_repository_binding(
        &self,
        skill_id: SkillId,
        suite_revision: RevisionId,
        repository: &Path,
        profile: RepositoryProfile,
        strict: bool,
        git: &ApprovedRepositoryTool,
    ) -> AppResult<RepositoryBinding> {
        self.observe_repository_binding_with_process(
            RepositoryBindingObservationRequest {
                skill_id,
                suite_revision,
                repository,
                profile,
                strict,
                git,
            },
            &SystemProcessPort,
        )
        .await
    }

    /// Revalidates a stored binding against the current local suite and
    /// repository. Missing approvals/offline roots retain the prior report as
    /// stale; only a fresh evaluation replaces the stored result.
    pub async fn evaluate_repository_binding(
        &self,
        binding_id: &str,
        tools: RepositoryCheckTools,
    ) -> AppResult<RepositoryBindingEvaluation> {
        let library = self.library();
        let binding = library
            .load_repository_binding(binding_id)
            .await?
            .ok_or(AppError::NotFound)?;
        let previous_report = library.load_repository_binding_report(binding_id).await?;
        let detail = library.load_skill(binding.skill_id()).await?;
        let current_suite_revision = detail.as_ref().and_then(|detail| {
            (detail.heads().len() == 1 && !detail.heads()[0].summary().deleted())
                .then(|| detail.heads()[0].summary().revision_id().clone())
        });
        let mut observation_failure = None;
        let observed_binding = match tools.git.as_ref() {
            Some(git) => match self
                .observe_repository_binding(
                    binding.skill_id(),
                    binding.suite_revision().clone(),
                    binding.repository_root(),
                    binding.profile(),
                    binding.strict(),
                    git,
                )
                .await
            {
                Ok(observed) => Some(observed),
                Err(error) => {
                    observation_failure = Some(repository_binding_observation_failure(&error));
                    None
                }
            },
            None => {
                observation_failure = Some(RepositoryBindingStaleReason::GitApprovalRequired);
                None
            }
        };

        let mut policies = None;
        let mut context = CheckContext::default();
        let mut policy_service = self.policy.clone();
        if current_suite_revision.as_ref() == Some(binding.suite_revision())
            && let Some(observed) = observed_binding.as_ref()
            && let (Ok(bundle), Ok(repository_context), Ok(repository_policy)) = (
                library
                    .load_validated_revision(binding.skill_id(), binding.suite_revision())
                    .await,
                self.repository_check_context(
                    observed.repository_root(),
                    binding.profile().as_str(),
                ),
                self.policy_for_repository(observed.repository_root(), tools),
            )
        {
            policies = Some(bundle.policies().to_vec());
            context = repository_context;
            policy_service = repository_policy;
        }

        let mut request = RepositoryBindingEvaluationRequest::new(
            binding.clone(),
            observed_binding,
            current_suite_revision,
            policies,
            context,
            previous_report.clone(),
            self.clock.now_utc(),
        );
        if let Some(reason) = observation_failure {
            request = request.with_observation_failure(reason);
        }
        let evaluation = policy_service.evaluate_binding(request).await?;
        if evaluation.is_stale() {
            return Ok(evaluation);
        }
        let refreshed = match library.bind_repository(evaluation.binding().clone()).await {
            Ok(binding) => binding,
            Err(_) => {
                return Ok(RepositoryBindingEvaluation::stale(
                    binding,
                    previous_report,
                    RepositoryBindingStaleReason::BindingChangedDuringEvaluation,
                ));
            }
        };
        let Some(report) = evaluation.report().cloned() else {
            return Ok(RepositoryBindingEvaluation::stale(
                refreshed,
                previous_report,
                RepositoryBindingStaleReason::EvaluationUnavailable,
            ));
        };
        if library
            .save_repository_binding_report(&refreshed, report)
            .await
            .is_err()
        {
            let previous = library
                .load_repository_binding_report(binding_id)
                .await
                .ok()
                .flatten()
                .or(previous_report);
            return Ok(RepositoryBindingEvaluation::stale(
                refreshed,
                previous,
                RepositoryBindingStaleReason::BindingChangedDuringEvaluation,
            ));
        }
        Ok(evaluation)
    }

    async fn observe_repository_binding_with_process(
        &self,
        request: RepositoryBindingObservationRequest<'_>,
        process: &dyn ProcessPort,
    ) -> AppResult<RepositoryBinding> {
        let RepositoryBindingObservationRequest {
            skill_id,
            suite_revision,
            repository,
            profile,
            strict,
            git,
        } = request;
        let observed_before = fingerprint_executable(git.executable_path())?;
        if observed_before != git.fingerprint() {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "repository.binding.git.fingerprint.mismatch",
                "Selected Git executable does not match its approved fingerprint.",
            )]));
        }
        let executable =
            ApprovedExecutable::from_absolute_path(git.executable_path().to_path_buf())
                .map_err(AppError::Validation)?;
        let environment = approved_repository_check_environment()?;
        let facts = crate::process::collect_repository_facts(
            repository,
            &executable,
            git.fingerprint(),
            &environment,
            process,
        )
        .await?;
        let observed_after = fingerprint_executable(git.executable_path())?;
        if observed_after != git.fingerprint() {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "repository.binding.git.fingerprint.changed",
                "Selected Git executable changed during repository inspection.",
            )]));
        }
        if !matches!(
            facts.state(),
            RepositoryState::Attached | RepositoryState::Detached
        ) {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "repository.binding.git.repository.required",
                "A non-bare Git working tree is required for a repository binding.",
            )]));
        }
        let canonical_root = facts.root();
        let top_level = facts.top_level().ok_or_else(|| {
            AppError::Validation(vec![Diagnostic::error(
                "repository.binding.root.invalid",
                "Selected directory is not the Git repository root.",
            )])
        })?;
        let canonical_top_level = std::fs::canonicalize(top_level).map_err(|_| {
            AppError::Validation(vec![Diagnostic::error(
                "repository.binding.root.invalid",
                "Git repository root could not be canonicalized.",
            )])
        })?;
        if canonical_top_level != canonical_root {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "repository.binding.root.mismatch",
                "Select the canonical top-level directory of the Git repository.",
            )]));
        }
        let head = facts.head().cloned().ok_or_else(|| {
            AppError::Validation(vec![Diagnostic::error(
                "repository.binding.head.unavailable",
                "A repository without a committed HEAD cannot be bound.",
            )])
        })?;
        let approved_root = ApprovedRoot::from_absolute_path(canonical_root.to_path_buf())
            .map_err(AppError::Validation)?;
        let project_facts = crate::platform::inspect_project_manifests(&approved_root)?;
        self.repository_check_context(canonical_root, profile.as_str())?;

        let mut digest = Sha256::new();
        digest.update(b"JAMESKILLS-REPOSITORY-BINDING-V1\0");
        update_binding_digest(&mut digest, canonical_root.as_os_str().as_encoded_bytes());
        update_binding_digest(&mut digest, profile.as_str().as_bytes());
        update_binding_digest(
            &mut digest,
            repository_stack_name(project_facts.stack()).as_bytes(),
        );
        update_binding_digest(&mut digest, facts.git_version().as_bytes());
        update_binding_digest(&mut digest, git.fingerprint().as_bytes());
        update_binding_digest(&mut digest, self.facts.architecture.as_bytes());
        update_binding_digest(
            &mut digest,
            host_platform_name(self.facts.platform).as_bytes(),
        );
        digest.update([u8::from(facts.is_linked_worktree())]);
        digest.update([u8::from(facts.is_submodule())]);
        for (key, value) in environment.entries() {
            update_binding_digest(&mut digest, key.to_string_lossy().as_bytes());
            update_binding_digest(&mut digest, value.to_string_lossy().as_bytes());
        }
        let environment_fingerprint = format!("sha256:{:x}", digest.finalize());
        RepositoryBinding::new(
            skill_id,
            suite_revision,
            canonical_root.to_path_buf(),
            profile,
            strict,
            head,
            environment_fingerprint,
        )
    }
}

fn update_binding_digest(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

fn host_platform_name(platform: crate::platform::HostPlatform) -> &'static str {
    match platform {
        crate::platform::HostPlatform::Linux => "linux",
        crate::platform::HostPlatform::Windows => "windows",
        crate::platform::HostPlatform::Other => "other",
    }
}

fn repository_binding_observation_failure(error: &AppError) -> RepositoryBindingStaleReason {
    match error {
        AppError::NotFound => RepositoryBindingStaleReason::RepositoryUnavailable,
        AppError::Validation(diagnostics) => {
            if diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code() == "policy.profile.stack_mismatch")
            {
                RepositoryBindingStaleReason::ProfileChanged
            } else if diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code().starts_with("repository.binding.root."))
            {
                RepositoryBindingStaleReason::RepositoryMoved
            } else if diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .code()
                    .starts_with("repository.binding.git.fingerprint.")
            }) {
                RepositoryBindingStaleReason::GitApprovalRequired
            } else {
                RepositoryBindingStaleReason::EvaluationUnavailable
            }
        }
        _ => RepositoryBindingStaleReason::EvaluationUnavailable,
    }
}

fn repository_stack_name(stack: crate::platform::ProjectStack) -> &'static str {
    match stack {
        crate::platform::ProjectStack::Rust => "rust",
        crate::platform::ProjectStack::Node => "node",
        crate::platform::ProjectStack::RustAndNode => "rust-node",
        crate::platform::ProjectStack::Generic => "generic",
        crate::platform::ProjectStack::Unknown => "unknown",
    }
}

fn approved_repository_check_environment() -> AppResult<ApprovedEnv> {
    const ALLOWED_KEYS: &[&str] = &[
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "SYSTEMROOT",
        "WINDIR",
        "TEMP",
        "TMP",
        "LANG",
        "LC_ALL",
        "LANGUAGE",
    ];
    let environment = ALLOWED_KEYS
        .iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value)))
        .collect();
    ApprovedEnv::new(environment).map_err(AppError::Validation)
}

/// Native repository tools selected and fingerprint-approved by the caller.
/// An omitted tool is never discovered-and-executed implicitly.
#[derive(Default)]
pub struct RepositoryCheckTools {
    git: Option<ApprovedRepositoryTool>,
    gitleaks: Option<ApprovedRepositoryTool>,
    commitlint: Option<ApprovedCommitlint>,
    github: Option<ApprovedRepositoryTool>,
    cargo: Option<ApprovedRepositoryTool>,
    node_npm: Option<ApprovedNodeNpm>,
}

impl RepositoryCheckTools {
    pub fn with_git(mut self, tool: ApprovedRepositoryTool) -> Self {
        self.git = Some(tool);
        self
    }

    pub fn with_gitleaks(mut self, tool: ApprovedRepositoryTool) -> Self {
        self.gitleaks = Some(tool);
        self
    }

    pub fn with_commitlint(mut self, tool: ApprovedCommitlint) -> Self {
        self.commitlint = Some(tool);
        self
    }

    pub fn with_github_cli(mut self, tool: ApprovedRepositoryTool) -> Self {
        self.github = Some(tool);
        self
    }

    pub fn with_cargo(mut self, tool: ApprovedRepositoryTool) -> Self {
        self.cargo = Some(tool);
        self
    }

    pub fn with_node_npm(mut self, tool: ApprovedNodeNpm) -> Self {
        self.node_npm = Some(tool);
        self
    }
}

struct UnavailablePolicyCheckProvider;

#[async_trait::async_trait]
impl PolicyCheckProvider for UnavailablePolicyCheckProvider {
    async fn observe(&self, _requirement: &Requirement) -> AppResult<CheckObservation> {
        Ok(CheckObservation::unknown())
    }
}

struct SystemGuidanceFactsProvider {
    facts: PlatformFacts,
    clock: Arc<SystemClock>,
}

#[async_trait::async_trait]
impl GuidanceFactsProvider for SystemGuidanceFactsProvider {
    async fn observe_facts(&self) -> AppResult<GuidanceFacts> {
        let operating_system = match self.facts.platform {
            crate::platform::HostPlatform::Linux => "linux",
            crate::platform::HostPlatform::Windows => "windows",
            crate::platform::HostPlatform::Other => "other",
        };
        let architecture = ApplicabilityFact::Architecture
            .accepts_value(&self.facts.architecture)
            .then_some(self.facts.architecture.as_str());
        let fingerprint_source = format!(
            "guidance-platform-v1;os={operating_system};arch={}",
            architecture.unwrap_or("unknown"),
        );
        let digest = Sha256::digest(fingerprint_source.as_bytes());
        let mut environment_fingerprint = String::with_capacity(71);
        environment_fingerprint.push_str("sha256:");
        for byte in digest {
            let _ = write!(environment_fingerprint, "{byte:02x}");
        }
        let observed_at = self.clock.now_utc();
        let expires_at = Some(self.clock.monotonic_ms().saturating_add(30_000));
        let mut observations = Vec::with_capacity(2);
        for (fact, value) in [
            (ApplicabilityFact::Os, Some(operating_system)),
            (ApplicabilityFact::Architecture, architecture),
        ] {
            if let Some(value) = value {
                let evidence = CheckEvidence::new(
                    "guidance.platform.facts",
                    &observed_at,
                    None,
                    &environment_fingerprint,
                    "Observed host platform facts.",
                    expires_at,
                )
                .map_err(AppError::Validation)?;
                observations.push(
                    GuidanceFactObservation::new(fact, value, evidence)
                        .map_err(AppError::Validation)?,
                );
            }
        }
        GuidanceFacts::new(&environment_fingerprint, observations).map_err(AppError::Validation)
    }
}

/// Build the available runtime adapters after validating caller-supplied paths.
/// Opening the persistent library creates the data directory/database as needed.
pub fn build_services(directories: UserDirectories) -> AppResult<RuntimeServices> {
    build_services_with_import_scanner(directories, None)
}

/// Build the standard runtime with an explicitly selected and fingerprinted
/// Gitleaks executable for import scanning.
pub fn build_services_with_gitleaks(
    directories: UserDirectories,
    executable_path: &Path,
    approved_fingerprint: ExecutableFingerprint,
) -> AppResult<RuntimeServices> {
    validate_directories(&directories)?;
    let executable = ApprovedExecutable::from_absolute_path(executable_path.to_path_buf())
        .map_err(AppError::Validation)?;
    let observed_fingerprint = fingerprint_executable(executable.path())?;
    if observed_fingerprint.as_bytes() != approved_fingerprint.as_bytes() {
        return Err(AppError::Validation(vec![Diagnostic::error(
            "tool.gitleaks.fingerprint.mismatch",
            "Selected Gitleaks executable does not match its approved SHA-256 fingerprint.",
        )]));
    }

    let profiles = load_tool_profiles().map_err(AppError::Validation)?;
    let profile = profiles
        .into_iter()
        .find(|profile| profile.tool_id() == jameskills_core::domain::ToolId::Gitleaks)
        .ok_or_else(|| AppError::CapabilityUnavailable {
            id: "tool.gitleaks.profile.unavailable".to_owned(),
            guidance_id: "tool.gitleaks.setup".to_owned(),
        })?;
    let parent = executable.path().parent().ok_or_else(|| {
        AppError::Validation(vec![Diagnostic::error(
            "tool.gitleaks.candidate.invalid",
            "Selected Gitleaks executable is not a registered native candidate.",
        )])
    })?;
    let canonical_selected = std::fs::canonicalize(executable.path()).map_err(|_| {
        AppError::Validation(vec![Diagnostic::error(
            "tool.gitleaks.candidate.invalid",
            "Selected Gitleaks executable is not a registered native candidate.",
        )])
    })?;
    let candidate = find_tool_candidates(
        std::slice::from_ref(&profile),
        &[parent.to_path_buf()],
        PlatformFacts::detect().platform,
    )
    .into_iter()
    .find(|candidate| {
        candidate.kind() == ToolCandidateKind::NativeExecutable
            && candidate
                .path()
                .and_then(|path| std::fs::canonicalize(path).ok())
                .is_some_and(|path| path == canonical_selected)
    })
    .ok_or_else(|| {
        AppError::Validation(vec![Diagnostic::error(
            "tool.gitleaks.candidate.invalid",
            "Selected Gitleaks executable is not a registered native candidate.",
        )])
    })?;
    let environment = ApprovedEnv::new(Default::default()).map_err(AppError::Validation)?;
    let process: Arc<dyn ProcessPort> = Arc::new(SystemProcessPort);
    let scanner: Arc<dyn ImportScanPort> = Arc::new(GitleaksImportScanner::new(
        profile,
        candidate,
        Some(approved_fingerprint),
        &environment,
        process,
        directories.cache.join("import-scans"),
    ));
    build_services_with_import_scanner(directories, Some(scanner))
}

fn build_services_with_import_scanner(
    directories: UserDirectories,
    import_scanner: Option<Arc<dyn ImportScanPort>>,
) -> AppResult<RuntimeServices> {
    validate_directories(&directories)?;
    let storage: Arc<dyn StoragePort> = Arc::new(SqliteStore::open(
        &directories.data.join("library.sqlite3"),
    )?);
    let facts = PlatformFacts::detect();
    let clock = Arc::new(SystemClock::new());
    let policy_clock: Arc<dyn ClockPort> = clock.clone();
    let policy = Arc::new(PolicyService::new(
        Arc::new(UnavailablePolicyCheckProvider),
        policy_clock,
    ));
    let guidance_facts = Arc::new(SystemGuidanceFactsProvider {
        facts: facts.clone(),
        clock: clock.clone(),
    });
    let guidance = Arc::new(GuidanceService::new(
        policy.clone(),
        guidance_facts,
        clock.clone(),
    ));
    Ok(RuntimeServices {
        facts,
        directories,
        clock: clock.clone(),
        library: Arc::new({
            let library = LibraryService::new(Arc::new(LocalFileSystem)).with_storage(storage);
            match import_scanner {
                Some(scanner) => library.with_import_scanner(scanner),
                None => library,
            }
        }),
        policy,
        guidance,
    })
}

fn validate_directories(directories: &UserDirectories) -> AppResult<()> {
    let paths = [&directories.config, &directories.data, &directories.cache];
    let invalid = paths.iter().any(|path| {
        !path.is_absolute()
            || path.parent().is_none()
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::CurDir | std::path::Component::ParentDir
                )
            })
    });
    let overlapping = paths.iter().enumerate().any(|(index, path)| {
        paths
            .iter()
            .skip(index + 1)
            .any(|other| paths_overlap(path, other, cfg!(windows)))
    });

    if invalid || overlapping {
        return Err(AppError::Validation(vec![Diagnostic::error(
            "config.directories.invalid",
            "Application directories must be absolute, distinct, and non-overlapping",
        )]));
    }
    Ok(())
}

fn paths_overlap(left: &std::path::Path, right: &std::path::Path, case_insensitive: bool) -> bool {
    path_is_within(left, right, case_insensitive) || path_is_within(right, left, case_insensitive)
}

fn path_is_within(path: &std::path::Path, root: &std::path::Path, case_insensitive: bool) -> bool {
    let path_components = path.components().collect::<Vec<_>>();
    let root_components = root.components().collect::<Vec<_>>();
    path_components.len() >= root_components.len()
        && path_components
            .iter()
            .zip(root_components.iter())
            .all(|(left, right)| {
                if case_insensitive {
                    left.as_os_str().to_string_lossy().to_lowercase()
                        == right.as_os_str().to_string_lossy().to_lowercase()
                } else {
                    left == right
                }
            })
}

#[cfg(test)]
mod tests {
    use super::paths_overlap;
    use super::{
        ApprovedRepositoryTool, RepositoryBindingObservationRequest, UserDirectories,
        build_services,
    };
    use async_trait::async_trait;
    use jameskills_core::{
        AppError,
        application::policy::CheckRequest,
        domain::{RepositoryProfile, RevisionId, SkillId, parse_policy, policy::CheckStatus},
        ports::process::{
            ApprovedExecutable, ExecutableFingerprint, ProcessOutput, ProcessPort, ProcessSpec,
        },
    };
    use std::{
        collections::VecDeque,
        path::{Path, PathBuf},
        sync::Mutex,
    };

    struct FixedProcess {
        responses: Mutex<VecDeque<ProcessOutput>>,
        fingerprints: Mutex<Vec<Option<ExecutableFingerprint>>>,
    }

    impl FixedProcess {
        fn new(responses: Vec<ProcessOutput>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                fingerprints: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ProcessPort for FixedProcess {
        async fn run(&self, spec: ProcessSpec) -> Result<ProcessOutput, AppError> {
            self.fingerprints
                .lock()
                .unwrap()
                .push(spec.approved_executable_fingerprint().copied());
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

    fn git_responses(root: &Path, head: char) -> Vec<ProcessOutput> {
        vec![
            ProcessOutput::new(Some(0), b"git version 2.55.0\n".to_vec(), vec![]),
            ProcessOutput::new(Some(0), b"true\n".to_vec(), vec![]),
            ProcessOutput::new(Some(0), b"false\n".to_vec(), vec![]),
            ProcessOutput::new(
                Some(0),
                format!("{}\n", root.display()).into_bytes(),
                vec![],
            ),
            ProcessOutput::new(Some(0), b"main\n".to_vec(), vec![]),
            ProcessOutput::new(
                Some(0),
                format!("{}\n", head.to_string().repeat(40)).into_bytes(),
                vec![],
            ),
            ProcessOutput::new(Some(0), b".git\n".to_vec(), vec![]),
            ProcessOutput::new(Some(0), b".git\n".to_vec(), vec![]),
            ProcessOutput::new(Some(0), b"\n".to_vec(), vec![]),
        ]
    }

    fn temporary_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "jameskills-binding-observation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".git")).unwrap();
        root
    }

    #[test]
    fn windows_directory_overlap_check_ignores_case() {
        assert!(paths_overlap(
            Path::new("C:/Users/Ada/AppData/Local/JameSkills"),
            Path::new("c:/users/ada/appdata/local/jameskills/config"),
            true,
        ));
    }

    #[test]
    fn repository_binding_observer_binds_current_head_and_rejects_incompatible_profile() {
        let repository = std::fs::canonicalize(temporary_root()).unwrap();
        let data_root = repository.join("app-data");
        let services = build_services(UserDirectories {
            config: data_root.join("config"),
            data: data_root.join("data"),
            cache: data_root.join("cache"),
        })
        .unwrap();
        let executable =
            ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap();
        let fingerprint = crate::process::fingerprint_executable(executable.path()).unwrap();
        let git = ApprovedRepositoryTool::new(executable, fingerprint);
        let executor = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let process_a = FixedProcess::new(git_responses(&repository, 'a'));
        let skill_id = SkillId::new();
        let suite_revision = RevisionId::from_digest([1; 32]);
        let binding_a = executor
            .block_on(services.observe_repository_binding_with_process(
                RepositoryBindingObservationRequest {
                    skill_id,
                    suite_revision: suite_revision.clone(),
                    repository: &repository,
                    profile: RepositoryProfile::Generic,
                    strict: true,
                    git: &git,
                },
                &process_a,
            ))
            .unwrap();
        assert_eq!(binding_a.repository_root(), repository);
        assert_eq!(binding_a.repository_head().as_str(), "a".repeat(40));
        assert_eq!(binding_a.profile(), RepositoryProfile::Generic);
        assert_eq!(binding_a.skill_id(), skill_id);
        assert_eq!(binding_a.suite_revision(), &suite_revision);
        assert!(
            process_a
                .fingerprints
                .lock()
                .unwrap()
                .iter()
                .all(|observed| *observed == Some(fingerprint))
        );

        let process_b = FixedProcess::new(git_responses(&repository, 'b'));
        let binding_b = executor
            .block_on(services.observe_repository_binding_with_process(
                RepositoryBindingObservationRequest {
                    skill_id,
                    suite_revision,
                    repository: &repository,
                    profile: RepositoryProfile::Generic,
                    strict: true,
                    git: &git,
                },
                &process_b,
            ))
            .unwrap();
        assert_ne!(
            binding_a.repository_head().as_str(),
            binding_b.repository_head().as_str()
        );
        assert_eq!(
            binding_a.environment_fingerprint(),
            binding_b.environment_fingerprint()
        );

        let incompatible_process = FixedProcess::new(git_responses(&repository, 'c'));
        let incompatible = executor.block_on(services.observe_repository_binding_with_process(
            RepositoryBindingObservationRequest {
                skill_id,
                suite_revision: RevisionId::from_digest([1; 32]),
                repository: &repository,
                profile: RepositoryProfile::Rust,
                strict: true,
                git: &git,
            },
            &incompatible_process,
        ));
        assert!(matches!(incompatible, Err(AppError::Validation(_))));
        drop(services);
        let _ = std::fs::remove_dir_all(repository);
    }

    #[test]
    fn unavailable_runtime_policy_provider_returns_unknown_not_pass() {
        let root =
            std::env::temp_dir().join(format!("jameskills policy runtime-{}", std::process::id()));
        let directories = UserDirectories {
            config: root.join("config"),
            data: root.join("data"),
            cache: root.join("cache"),
        };
        let runtime = build_services(directories).unwrap();
        let policy = parse_policy(
            include_str!("../../../tests/fixtures/valid-suite/policies/repository.toml").as_bytes(),
        )
        .unwrap();
        let report = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(
                runtime
                    .policy()
                    .check(CheckRequest::new(policy, Default::default())),
            )
            .unwrap();

        assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
        assert_eq!(report.strict_exit(), 1);
    }
}
