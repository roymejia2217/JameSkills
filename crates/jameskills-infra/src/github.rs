use crate::fs::ApprovedRepositoryTool;
use jameskills_core::{
    AppError, AppResult,
    domain::policy::{
        CheckEvidence, CheckObservation, CheckStatus, Enforcement, RepositoryHead, ToolId,
    },
    ports::ClockPort,
    ports::process::{
        ApprovedEnv, ApprovedRoot, CancellationToken, ProcessOutput, ProcessPermission,
        ProcessPort, ProcessSpec,
    },
};
use serde::Deserialize;
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf, time::Duration};

const AUTH_OUTPUT_LIMIT: usize = 64 * 1024;
const API_OUTPUT_LIMIT: usize = 256 * 1024;
const GH_VERSION_OUTPUT_LIMIT: usize = 1024;
const GH_CALL_TIMEOUT: Duration = Duration::from_secs(15);

/// A GitHub.com repository coordinate parsed from a configured Git remote.
/// It stores only the path components needed for a fixed REST endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GithubRepository {
    owner: String,
    name: String,
}

impl GithubRepository {
    pub fn from_remote_url(remote: &str) -> Option<Self> {
        let path = if let Some(rest) = remote.strip_prefix("https://github.com/") {
            if rest.contains(['@', '?', '#', ':']) {
                return None;
            }
            rest
        } else if let Some(rest) = remote.strip_prefix("git@github.com:") {
            if rest.contains(['@', '?', '#', ':']) {
                return None;
            }
            rest
        } else {
            return None;
        };
        let path = path.strip_suffix(".git").unwrap_or(path);
        let mut parts = path.split('/');
        let owner = parts.next()?;
        let name = parts.next()?;
        if parts.next().is_some() || !valid_coordinate(owner, 39) || !valid_coordinate(name, 100) {
            return None;
        }
        Some(Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Accepts only bounded `git remote -v` output whose fetch/push URLs all
    /// identify the same GitHub.com repository.
    pub fn from_git_remote_output(output: &[u8]) -> Option<Self> {
        if output.len() > 4096 {
            return None;
        }
        let text = std::str::from_utf8(output).ok()?;
        let mut selected: Option<Self> = None;
        for line in text.lines() {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() {
                continue;
            }
            let (_, details) = line.split_once('\t')?;
            let remote_url = details
                .strip_suffix(" (fetch)")
                .or_else(|| details.strip_suffix(" (push)"))?;
            let repository = Self::from_remote_url(remote_url)?;
            if let Some(previous) = &selected {
                if !previous.owner.eq_ignore_ascii_case(&repository.owner)
                    || !previous.name.eq_ignore_ascii_case(&repository.name)
                {
                    return None;
                }
            } else {
                selected = Some(repository);
            }
        }
        selected
    }
}

fn valid_coordinate(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && value != "."
        && value != ".."
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GithubAuthState {
    Authenticated,
    Unauthenticated,
    Unknown,
}

/// Read-only GitHub CLI driver. The executable fingerprint is rechecked by
/// ProcessPort before each invocation; caller environment is allowlisted.
pub struct GithubEvidenceDriver<'a> {
    gh: &'a ApprovedRepositoryTool,
    root: &'a ApprovedRoot,
    environment: &'a ApprovedEnv,
    process: &'a dyn ProcessPort,
    clock: &'a dyn ClockPort,
    environment_fingerprint: &'a str,
}

impl<'a> GithubEvidenceDriver<'a> {
    pub fn new(
        gh: &'a ApprovedRepositoryTool,
        root: &'a ApprovedRoot,
        environment: &'a ApprovedEnv,
        process: &'a dyn ProcessPort,
        clock: &'a dyn ClockPort,
        environment_fingerprint: &'a str,
    ) -> Self {
        Self {
            gh,
            root,
            environment,
            process,
            clock,
            environment_fingerprint,
        }
    }

    pub async fn inspect_host_auth(&self) -> AppResult<GithubAuthState> {
        if !self.supports_verified_release().await? {
            return Ok(GithubAuthState::Unknown);
        }
        let output = match self
            .run_gh(
                [
                    "auth",
                    "status",
                    "--active",
                    "--hostname",
                    "github.com",
                    "--json",
                    "hosts",
                ],
                AUTH_OUTPUT_LIMIT,
            )
            .await
        {
            Ok(output) => output,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return Ok(GithubAuthState::Unknown);
            }
            Err(error) => return Err(error),
        };
        if output.exit_code() != Some(0) || output.stdout().len() >= AUTH_OUTPUT_LIMIT {
            return Ok(GithubAuthState::Unknown);
        }
        Ok(parse_auth_status(output.stdout()))
    }

    pub async fn identify_repository(
        &self,
        repository: &GithubRepository,
        revision: &RepositoryHead,
    ) -> AppResult<CheckObservation> {
        match self.inspect_host_auth().await? {
            GithubAuthState::Authenticated => {}
            GithubAuthState::Unauthenticated => {
                return self.observation(
                    "github.repository.identity",
                    CheckStatus::Blocked,
                    None,
                    repository,
                    revision,
                    None,
                );
            }
            GithubAuthState::Unknown => {
                return self.observation(
                    "github.repository.identity",
                    CheckStatus::Unknown,
                    None,
                    repository,
                    revision,
                    None,
                );
            }
        }
        let endpoint = format!("repos/{}/{}", repository.owner(), repository.name());
        let output = match self
            .run_gh(
                [
                    "api",
                    "--hostname",
                    "github.com",
                    "--method",
                    "GET",
                    "--include",
                    endpoint.as_str(),
                ],
                API_OUTPUT_LIMIT,
            )
            .await
        {
            Ok(output) => output,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return self.observation(
                    "github.repository.identity",
                    CheckStatus::Unknown,
                    None,
                    repository,
                    revision,
                    None,
                );
            }
            Err(error) => return Err(error),
        };
        if output.stdout().len() >= API_OUTPUT_LIMIT {
            return self.observation(
                "github.repository.identity",
                CheckStatus::Unknown,
                None,
                repository,
                revision,
                None,
            );
        }
        let parsed = parse_repository_response(output.stdout(), output.exit_code(), repository);
        let status = match parsed {
            RepositoryResponse::Match => CheckStatus::Pass,
            RepositoryResponse::Mismatch => CheckStatus::Fail,
            RepositoryResponse::NotFoundOrDenied | RepositoryResponse::Unavailable => {
                CheckStatus::Unknown
            }
            RepositoryResponse::RateLimited
            | RepositoryResponse::Denied
            | RepositoryResponse::AuthenticationRequired => CheckStatus::Blocked,
        };
        let enforcement = match status {
            CheckStatus::Pass | CheckStatus::Fail => Some(Enforcement::LocalCheck),
            _ => None,
        };
        self.observation(
            "github.repository.identity",
            status,
            enforcement,
            repository,
            revision,
            (status == CheckStatus::Pass).then_some("repo-read"),
        )
    }

    pub async fn check_branch_policy(
        &self,
        repository: &GithubRepository,
        revision: &RepositoryHead,
        branch: &str,
        require_pull_request: bool,
        required_checks: &[String],
        require_no_bypass: bool,
    ) -> AppResult<CheckObservation> {
        if !supported_branch_name(branch) {
            return self.observation(
                "github.branch-policy",
                CheckStatus::Unsupported,
                None,
                repository,
                revision,
                None,
            );
        }
        match self.inspect_host_auth().await? {
            GithubAuthState::Authenticated => {}
            GithubAuthState::Unauthenticated => {
                return self.observation(
                    "github.branch-policy",
                    CheckStatus::Blocked,
                    None,
                    repository,
                    revision,
                    None,
                );
            }
            GithubAuthState::Unknown => {
                return self.observation(
                    "github.branch-policy",
                    CheckStatus::Unknown,
                    None,
                    repository,
                    revision,
                    None,
                );
            }
        }
        let branch_component = encode_path_component(branch);
        let effective_endpoint = format!(
            "repos/{}/{}/rules/branches/{branch_component}?per_page=100",
            repository.owner(),
            repository.name(),
        );
        let effective = self
            .run_api_get(&effective_endpoint, API_OUTPUT_LIMIT)
            .await?;
        let rules = match effective {
            ApiResponse::Json(value) => match value.as_array() {
                Some(raw_rules) if raw_rules.len() < 100 => raw_rules
                    .iter()
                    .map(parse_effective_rule)
                    .collect::<Option<Vec<_>>>(),
                _ => None,
            },
            ApiResponse::Denied | ApiResponse::Unauthorized | ApiResponse::RateLimited => {
                return self.observation(
                    "github.branch-policy",
                    CheckStatus::Blocked,
                    None,
                    repository,
                    revision,
                    None,
                );
            }
            ApiResponse::NotFound | ApiResponse::Unknown => {
                return self.observation(
                    "github.branch-policy",
                    CheckStatus::Unknown,
                    None,
                    repository,
                    revision,
                    None,
                );
            }
        };
        let Some(rules) = rules else {
            return self.observation(
                "github.branch-policy",
                CheckStatus::Unknown,
                None,
                repository,
                revision,
                None,
            );
        };

        let classic_endpoint = format!(
            "repos/{}/{}/branches/{branch_component}/protection",
            repository.owner(),
            repository.name(),
        );
        let classic = self
            .run_api_get(&classic_endpoint, API_OUTPUT_LIMIT)
            .await?;
        let classic_protection = match classic {
            ApiResponse::Json(value) => match value.as_object() {
                Some(protection) => Some(protection.clone()),
                None => {
                    return self.observation(
                        "github.branch-policy",
                        CheckStatus::Unknown,
                        None,
                        repository,
                        revision,
                        None,
                    );
                }
            },
            ApiResponse::Denied | ApiResponse::Unauthorized | ApiResponse::RateLimited => {
                return self.observation(
                    "github.branch-policy",
                    CheckStatus::Blocked,
                    None,
                    repository,
                    revision,
                    None,
                );
            }
            // A 404 may mean no classic protection, inaccessible private
            // settings, or plan limitation. Effective active rules can still
            // prove requested positive rules; absent classic data stays unknown.
            ApiResponse::NotFound => None,
            ApiResponse::Unknown => {
                return self.observation(
                    "github.branch-policy",
                    CheckStatus::Unknown,
                    None,
                    repository,
                    revision,
                    None,
                );
            }
        };
        let classic = classic_protection.as_ref();
        let classic_pr =
            classic.and_then(
                |protection| match protection.get("required_pull_request_reviews") {
                    Some(serde_json::Value::Null) => Some(false),
                    Some(serde_json::Value::Object(_)) => Some(true),
                    None | Some(_) => None,
                },
            );
        let active_pr = rules.iter().any(|rule| rule.pull_request);
        let pr_ok = if !require_pull_request || active_pr {
            Some(true)
        } else {
            classic_pr
        };
        let mut host_checks = BTreeMap::new();
        for rule in &rules {
            for (context, app_id) in &rule.required_checks {
                if !merge_check_source(&mut host_checks, context.clone(), *app_id) {
                    return self.observation(
                        "github.branch-policy",
                        CheckStatus::Unknown,
                        None,
                        repository,
                        revision,
                        None,
                    );
                }
            }
        }
        let classic_checks = match classic {
            Some(protection) => match parse_classic_required_checks(protection) {
                Some(checks) => Some(checks),
                None => {
                    return self.observation(
                        "github.branch-policy",
                        CheckStatus::Unknown,
                        None,
                        repository,
                        revision,
                        None,
                    );
                }
            },
            None => None,
        };
        if let Some(classic_checks) = &classic_checks {
            for (context, app_id) in classic_checks {
                if !merge_check_source(&mut host_checks, context.clone(), *app_id) {
                    return self.observation(
                        "github.branch-policy",
                        CheckStatus::Unknown,
                        None,
                        repository,
                        revision,
                        None,
                    );
                }
            }
        }
        let checks_ok = if required_checks
            .iter()
            .all(|required| host_checks.contains_key(required))
        {
            Some(true)
        } else if classic_checks.is_some() {
            Some(false)
        } else {
            None
        };
        let rulesets_endpoint = format!(
            "repos/{}/{}/rulesets?includes_parents=true&targets=branch&per_page=100",
            repository.owner(),
            repository.name(),
        );
        let ruleset_bypass = match self
            .run_api_get(&rulesets_endpoint, API_OUTPUT_LIMIT)
            .await?
        {
            ApiResponse::Json(value) => match value.as_array() {
                Some(rulesets) if rulesets.len() < 100 => {
                    ruleset_bypass_state(rulesets, branch, !rules.is_empty())
                }
                _ => BypassState::Unknown,
            },
            ApiResponse::Denied
            | ApiResponse::Unauthorized
            | ApiResponse::RateLimited
            | ApiResponse::NotFound
            | ApiResponse::Unknown => BypassState::Unknown,
        };
        let classic_bypass = classic.map_or(BypassState::Unknown, classic_bypass_state);
        let bypass = combine_bypass_state(classic_bypass, ruleset_bypass);
        let condition_status = match (pr_ok, checks_ok) {
            (Some(false), _) | (_, Some(false)) => CheckStatus::Fail,
            (Some(true), Some(true)) => CheckStatus::Pass,
            _ => CheckStatus::Unknown,
        };
        let status = if require_no_bypass {
            match bypass {
                BypassState::NoneVisible => condition_status,
                BypassState::Present => CheckStatus::Fail,
                BypassState::Unknown if condition_status == CheckStatus::Fail => CheckStatus::Fail,
                BypassState::Unknown => CheckStatus::Unknown,
            }
        } else {
            condition_status
        };
        let observed_rule =
            !rules.is_empty() || classic_pr == Some(true) || !host_checks.is_empty();
        let enforcement = if status == CheckStatus::Unknown {
            None
        } else {
            observed_rule.then_some(Enforcement::HostRule)
        };
        self.observation_with_detail(
            "github.branch-policy",
            status,
            enforcement,
            repository,
            revision,
            bypass_evidence(bypass),
        )
    }

    pub async fn check_ci_evidence(
        &self,
        repository: &GithubRepository,
        revision: &RepositoryHead,
        required_checks: &[String],
    ) -> AppResult<CheckObservation> {
        if required_checks.is_empty() {
            return self.observation(
                "github.ci-evidence",
                CheckStatus::Unsupported,
                None,
                repository,
                revision,
                None,
            );
        }
        let branch_policy = self
            .check_branch_policy(repository, revision, "main", false, required_checks, false)
            .await?;
        if branch_policy.status() != CheckStatus::Pass
            || branch_policy.enforcement() != Some(Enforcement::HostRule)
        {
            return self.observation(
                "github.ci-evidence",
                branch_policy.status(),
                branch_policy.enforcement(),
                repository,
                revision,
                None,
            );
        }

        let Some(expected_sources) = self.required_check_sources(repository, "main").await? else {
            return self.observation(
                "github.ci-evidence",
                CheckStatus::Unknown,
                Some(Enforcement::HostRule),
                repository,
                revision,
                None,
            );
        };
        if required_checks
            .iter()
            .any(|name| !expected_sources.contains_key(name))
        {
            return self.observation(
                "github.ci-evidence",
                CheckStatus::Fail,
                Some(Enforcement::HostRule),
                repository,
                revision,
                None,
            );
        }

        let checks_endpoint = format!(
            "repos/{}/{}/commits/{}/check-runs?filter=latest&per_page=100",
            repository.owner(),
            repository.name(),
            revision.as_str(),
        );
        let check_runs = match self.run_api_get(&checks_endpoint, API_OUTPUT_LIMIT).await? {
            ApiResponse::Json(value) => parse_check_runs(&value, revision.as_str()),
            ApiResponse::RateLimited | ApiResponse::Denied | ApiResponse::Unauthorized => {
                return self.observation(
                    "github.ci-evidence",
                    CheckStatus::Blocked,
                    Some(Enforcement::HostRule),
                    repository,
                    revision,
                    None,
                );
            }
            ApiResponse::NotFound | ApiResponse::Unknown => {
                return self.observation(
                    "github.ci-evidence",
                    CheckStatus::Unknown,
                    Some(Enforcement::HostRule),
                    repository,
                    revision,
                    None,
                );
            }
        };
        let Some(check_runs) = check_runs else {
            return self.observation(
                "github.ci-evidence",
                CheckStatus::Unknown,
                Some(Enforcement::HostRule),
                repository,
                revision,
                None,
            );
        };

        let needs_status_fallback = required_checks
            .iter()
            .any(|name| expected_sources.get(name).is_some_and(Option::is_none));
        let statuses = if needs_status_fallback {
            let statuses_endpoint = format!(
                "repos/{}/{}/commits/{}/status?per_page=100",
                repository.owner(),
                repository.name(),
                revision.as_str(),
            );
            match self
                .run_api_get(&statuses_endpoint, API_OUTPUT_LIMIT)
                .await?
            {
                ApiResponse::Json(value) => {
                    match parse_commit_statuses(&value, revision.as_str()) {
                        Some(statuses) => Some(statuses),
                        None => {
                            return self.observation(
                                "github.ci-evidence",
                                CheckStatus::Unknown,
                                Some(Enforcement::HostRule),
                                repository,
                                revision,
                                None,
                            );
                        }
                    }
                }
                ApiResponse::RateLimited | ApiResponse::Denied | ApiResponse::Unauthorized => {
                    return self.observation(
                        "github.ci-evidence",
                        CheckStatus::Blocked,
                        Some(Enforcement::HostRule),
                        repository,
                        revision,
                        None,
                    );
                }
                ApiResponse::NotFound | ApiResponse::Unknown => {
                    return self.observation(
                        "github.ci-evidence",
                        CheckStatus::Unknown,
                        Some(Enforcement::HostRule),
                        repository,
                        revision,
                        None,
                    );
                }
            }
        } else {
            None
        };

        let mut outcomes = Vec::with_capacity(required_checks.len());
        for name in required_checks {
            let Some(app_id) = expected_sources.get(name).copied() else {
                outcomes.push(CheckOutcome::Unknown);
                continue;
            };
            let run_outcome = check_run_outcome(&check_runs, revision.as_str(), name, app_id);
            let outcome = if app_id.is_some() {
                run_outcome
            } else if let Some(statuses) = statuses.as_ref() {
                combine_check_sources(run_outcome, commit_status_outcome(statuses, name))
            } else {
                CheckOutcome::Unknown
            };
            outcomes.push(outcome);
        }
        let status = aggregate_check_outcomes(&outcomes);
        let enforcement = Some(Enforcement::RequiredCi);
        self.observation(
            "github.ci-evidence",
            status,
            enforcement,
            repository,
            revision,
            (status == CheckStatus::Pass).then_some("current-sha-checks"),
        )
    }

    async fn required_check_sources(
        &self,
        repository: &GithubRepository,
        branch: &str,
    ) -> AppResult<Option<BTreeMap<String, Option<i64>>>> {
        let branch_component = encode_path_component(branch);
        let effective_endpoint = format!(
            "repos/{}/{}/rules/branches/{branch_component}?per_page=100",
            repository.owner(),
            repository.name(),
        );
        let effective = match self
            .run_api_get(&effective_endpoint, API_OUTPUT_LIMIT)
            .await?
        {
            ApiResponse::Json(value) => value,
            _ => return Ok(None),
        };
        let Some(rules) = effective.as_array().filter(|rules| rules.len() < 100) else {
            return Ok(None);
        };
        let mut sources = BTreeMap::new();
        for rule in rules {
            let Some(rule) = parse_effective_rule(rule) else {
                return Ok(None);
            };
            for (context, app_id) in rule.required_checks {
                if !merge_check_source(&mut sources, context, app_id) {
                    return Ok(None);
                }
            }
        }

        let classic_endpoint = format!(
            "repos/{}/{}/branches/{branch_component}/protection",
            repository.owner(),
            repository.name(),
        );
        let classic = match self
            .run_api_get(&classic_endpoint, API_OUTPUT_LIMIT)
            .await?
        {
            ApiResponse::Json(value) => value,
            ApiResponse::NotFound => return Ok(Some(sources)),
            _ => return Ok(None),
        };
        let Some(protection) = classic.as_object() else {
            return Ok(None);
        };
        let Some(classic_checks) = parse_classic_required_checks(protection) else {
            return Ok(None);
        };
        for (context, app_id) in classic_checks {
            if !merge_check_source(&mut sources, context, app_id) {
                return Ok(None);
            }
        }
        Ok(Some(sources))
    }

    async fn supports_verified_release(&self) -> AppResult<bool> {
        let output = match self.run_gh(["--version"], GH_VERSION_OUTPUT_LIMIT).await {
            Ok(output) => output,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(AppError::ExternalTool { .. } | AppError::PermissionDenied { .. }) => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        if output.exit_code() != Some(0) || output.stdout().len() >= GH_VERSION_OUTPUT_LIMIT {
            return Ok(false);
        }
        let Ok(output) = std::str::from_utf8(output.stdout()) else {
            return Ok(false);
        };
        let Some(version) = output
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("gh version "))
            .and_then(|line| line.split_whitespace().next())
            .and_then(|value| semver::Version::parse(value).ok())
        else {
            return Ok(false);
        };
        Ok(version == semver::Version::new(2, 102, 0))
    }

    async fn run_gh<const N: usize>(
        &self,
        arguments: [&str; N],
        output_limit: usize,
    ) -> AppResult<ProcessOutput> {
        let executable_path: PathBuf = self.gh.executable_path().to_path_buf();
        let fingerprint = self.gh.fingerprint();
        let executable = jameskills_core::ports::process::ApprovedExecutable::from_absolute_path(
            executable_path,
        )
        .map_err(AppError::Validation)?;
        let root = ApprovedRoot::from_absolute_path(self.root.path().to_path_buf())
            .map_err(AppError::Validation)?;
        let environment =
            ApprovedEnv::new(self.environment.entries().clone()).map_err(AppError::Validation)?;
        let spec = ProcessSpec::new(
            executable,
            ToolId::Gh,
            arguments.into_iter().map(OsString::from).collect(),
            root,
            environment,
            GH_CALL_TIMEOUT,
            output_limit,
            ProcessPermission::ReadOnlyCheck,
            CancellationToken::new(),
        )
        .map_err(AppError::Validation)?
        .with_approved_executable_fingerprint(fingerprint);
        self.process.run(spec).await
    }

    async fn run_api_get(&self, endpoint: &str, output_limit: usize) -> AppResult<ApiResponse> {
        let output = self
            .run_gh(
                [
                    "api",
                    "--hostname",
                    "github.com",
                    "--method",
                    "GET",
                    "--include",
                    endpoint,
                ],
                output_limit,
            )
            .await?;
        Ok(parse_api_response(&output, output_limit))
    }

    fn observation(
        &self,
        source_id: &'static str,
        status: CheckStatus,
        enforcement: Option<Enforcement>,
        repository: &GithubRepository,
        revision: &RepositoryHead,
        capability: Option<&str>,
    ) -> AppResult<CheckObservation> {
        let capability = capability.unwrap_or("unconfirmed");
        let summary = format!(
            "r={}/{};sha={};cap={};result={}",
            repository.owner(),
            repository.name(),
            revision.as_str(),
            capability,
            match status {
                CheckStatus::Pass => "pass",
                CheckStatus::Fail => "fail",
                CheckStatus::Blocked => "blocked",
                CheckStatus::Unknown => "unknown",
                CheckStatus::Unsupported => "unsupported",
                CheckStatus::NotApplicable => "not-applicable",
            },
        );
        self.create_observation(source_id, status, enforcement, summary)
    }

    fn observation_with_detail(
        &self,
        source_id: &'static str,
        status: CheckStatus,
        enforcement: Option<Enforcement>,
        repository: &GithubRepository,
        revision: &RepositoryHead,
        detail: &'static str,
    ) -> AppResult<CheckObservation> {
        let summary = format!(
            "r={}/{};sha={};{};result={}",
            repository.owner(),
            repository.name(),
            revision.as_str(),
            detail,
            status_name(status),
        );
        self.create_observation(source_id, status, enforcement, summary)
    }

    fn create_observation(
        &self,
        source_id: &'static str,
        status: CheckStatus,
        enforcement: Option<Enforcement>,
        summary: String,
    ) -> AppResult<CheckObservation> {
        let evidence = CheckEvidence::new(
            source_id,
            &self.clock.now_utc(),
            None,
            self.environment_fingerprint,
            summary,
            Some(self.clock.monotonic_ms().saturating_add(30_000)),
        )
        .map_err(AppError::Validation)?;
        CheckObservation::new(status, enforcement, vec![evidence]).map_err(AppError::Validation)
    }
}

fn status_name(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "pass",
        CheckStatus::Fail => "fail",
        CheckStatus::Blocked => "blocked",
        CheckStatus::Unknown => "unknown",
        CheckStatus::Unsupported => "unsupported",
        CheckStatus::NotApplicable => "not-applicable",
    }
}

fn parse_auth_status(bytes: &[u8]) -> GithubAuthState {
    #[derive(Deserialize)]
    struct HostEntry {
        host: String,
        active: bool,
        state: String,
    }
    #[derive(Deserialize)]
    struct AuthStatus {
        hosts: std::collections::BTreeMap<String, Vec<HostEntry>>,
    }
    let Ok(document) = serde_json::from_slice::<AuthStatus>(bytes) else {
        return GithubAuthState::Unknown;
    };
    let Some(entries) = document.hosts.get("github.com") else {
        return GithubAuthState::Unauthenticated;
    };
    let Some(active) = entries
        .iter()
        .find(|entry| entry.active && entry.host == "github.com")
    else {
        return GithubAuthState::Unauthenticated;
    };
    match active.state.as_str() {
        "success" => GithubAuthState::Authenticated,
        "error" | "timeout" => GithubAuthState::Unauthenticated,
        _ => GithubAuthState::Unknown,
    }
}

enum ApiResponse {
    Json(serde_json::Value),
    NotFound,
    RateLimited,
    Denied,
    Unauthorized,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BypassState {
    NoneVisible,
    Present,
    Unknown,
}

fn parse_api_response(output: &ProcessOutput, output_limit: usize) -> ApiResponse {
    if output.stdout().len() >= output_limit {
        return ApiResponse::Unknown;
    }
    let Ok(text) = std::str::from_utf8(output.stdout()) else {
        return ApiResponse::Unknown;
    };
    let Some((headers, body)) = text.split_once("\r\n\r\n") else {
        return ApiResponse::Unknown;
    };
    let status_line = headers.lines().next().unwrap_or_default();
    if status_line.contains(" 401 ") {
        return ApiResponse::Unauthorized;
    }
    if status_line.contains(" 429 ") {
        return ApiResponse::RateLimited;
    }
    if status_line.contains(" 403 ") {
        let rate_limited = headers.lines().any(|line| {
            let (name, value) = line.split_once(':').unwrap_or_default();
            (name.eq_ignore_ascii_case("retry-after") && !value.trim().is_empty())
                || (name.eq_ignore_ascii_case("x-ratelimit-remaining") && value.trim() == "0")
        });
        return if rate_limited {
            ApiResponse::RateLimited
        } else {
            ApiResponse::Denied
        };
    }
    if status_line.contains(" 404 ") {
        return ApiResponse::NotFound;
    }
    if !status_line.contains(" 200 ") || output.exit_code() != Some(0) {
        return ApiResponse::Unknown;
    }
    serde_json::from_str(body).map_or(ApiResponse::Unknown, ApiResponse::Json)
}

fn parse_check_runs(value: &serde_json::Value, revision: &str) -> Option<CheckRunList> {
    let list = serde_json::from_value::<CheckRunList>(value.clone()).ok()?;
    if list.check_runs.len() >= 100 || list.total_count > list.check_runs.len() as u64 {
        return None;
    }
    if list.check_runs.iter().any(|run| {
        run.head_sha.len() != revision.len()
            || !run.head_sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return None;
    }
    Some(list)
}

fn parse_commit_statuses(value: &serde_json::Value, revision: &str) -> Option<CommitStatusList> {
    let list = serde_json::from_value::<CommitStatusList>(value.clone()).ok()?;
    if !list.sha.eq_ignore_ascii_case(revision)
        || list.statuses.len() >= 100
        || list.total_count > list.statuses.len() as u64
    {
        return None;
    }
    Some(list)
}

fn check_run_outcome(
    list: &CheckRunList,
    revision: &str,
    required_name: &str,
    required_app: Option<i64>,
) -> CheckOutcome {
    let candidates = list
        .check_runs
        .iter()
        .filter(|run| run.name == required_name)
        .filter(|run| {
            required_app.is_none_or(|app_id| {
                run.app
                    .as_ref()
                    .and_then(|app| app.id)
                    .is_some_and(|observed| observed == app_id)
            })
        })
        .filter(|run| run.head_sha.eq_ignore_ascii_case(revision))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        if required_app.is_some()
            && list
                .check_runs
                .iter()
                .any(|run| run.name == required_name && run.head_sha.eq_ignore_ascii_case(revision))
        {
            return CheckOutcome::Failed;
        }
        if list
            .check_runs
            .iter()
            .any(|run| run.name == required_name && !run.head_sha.eq_ignore_ascii_case(revision))
        {
            return CheckOutcome::Stale;
        }
        return CheckOutcome::Missing;
    }
    let latest = if candidates.len() == 1 {
        candidates[0]
    } else {
        let Some(latest_started_at) = candidates
            .iter()
            .filter_map(|run| run.started_at.as_deref())
            .max()
        else {
            return CheckOutcome::Unknown;
        };
        let latest = candidates
            .into_iter()
            .filter(|run| run.started_at.as_deref() == Some(latest_started_at))
            .collect::<Vec<_>>();
        if latest.len() != 1 {
            return CheckOutcome::Unknown;
        }
        latest[0]
    };
    match latest.status.as_str() {
        "queued" | "in_progress" | "waiting" | "requested" | "pending" => CheckOutcome::Pending,
        "completed" if latest.conclusion.as_deref() == Some("success") => CheckOutcome::Passed,
        "completed" if latest.conclusion.is_some() => CheckOutcome::Failed,
        _ => CheckOutcome::Unknown,
    }
}

fn commit_status_outcome(list: &CommitStatusList, required_name: &str) -> CheckOutcome {
    // GitHub returns commit statuses in reverse chronological order.
    let Some(latest) = list
        .statuses
        .iter()
        .find(|status| status.context == required_name)
    else {
        return CheckOutcome::Missing;
    };
    match latest.state.as_str() {
        "success" => CheckOutcome::Passed,
        "pending" => CheckOutcome::Pending,
        "failure" | "error" => CheckOutcome::Failed,
        _ => CheckOutcome::Unknown,
    }
}

fn combine_check_sources(runs: CheckOutcome, statuses: CheckOutcome) -> CheckOutcome {
    match (runs, statuses) {
        (CheckOutcome::Failed | CheckOutcome::Stale, _)
        | (_, CheckOutcome::Failed | CheckOutcome::Stale) => CheckOutcome::Failed,
        (CheckOutcome::Unknown, _) | (_, CheckOutcome::Unknown) => CheckOutcome::Unknown,
        (CheckOutcome::Pending, _) | (_, CheckOutcome::Pending) => CheckOutcome::Pending,
        (CheckOutcome::Passed, CheckOutcome::Missing)
        | (CheckOutcome::Missing, CheckOutcome::Passed)
        | (CheckOutcome::Passed, CheckOutcome::Passed) => CheckOutcome::Passed,
        (CheckOutcome::Missing, CheckOutcome::Missing) => CheckOutcome::Missing,
    }
}

fn aggregate_check_outcomes(outcomes: &[CheckOutcome]) -> CheckStatus {
    if outcomes
        .iter()
        .any(|outcome| matches!(outcome, CheckOutcome::Failed | CheckOutcome::Stale))
    {
        CheckStatus::Fail
    } else if outcomes.contains(&CheckOutcome::Unknown) {
        CheckStatus::Unknown
    } else if outcomes
        .iter()
        .any(|outcome| matches!(outcome, CheckOutcome::Pending | CheckOutcome::Missing))
    {
        CheckStatus::Blocked
    } else {
        CheckStatus::Pass
    }
}

struct EffectiveRule {
    pull_request: bool,
    required_checks: BTreeMap<String, Option<i64>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckOutcome {
    Passed,
    Failed,
    Pending,
    Missing,
    Stale,
    Unknown,
}

#[derive(Deserialize)]
struct CheckRunList {
    total_count: u64,
    check_runs: Vec<CheckRunItem>,
}

#[derive(Deserialize)]
struct CheckRunItem {
    head_sha: String,
    name: String,
    status: String,
    conclusion: Option<String>,
    started_at: Option<String>,
    app: Option<CheckRunApp>,
}

#[derive(Deserialize)]
struct CheckRunApp {
    id: Option<i64>,
}

#[derive(Deserialize)]
struct CommitStatusList {
    sha: String,
    total_count: u64,
    statuses: Vec<CommitStatusItem>,
}

#[derive(Deserialize)]
struct CommitStatusItem {
    context: String,
    state: String,
}

fn parse_effective_rule(value: &serde_json::Value) -> Option<EffectiveRule> {
    let object = value.as_object()?;
    let rule_type = object.get("type")?.as_str()?;
    if rule_type == "pull_request" {
        return Some(EffectiveRule {
            pull_request: true,
            required_checks: BTreeMap::new(),
        });
    }
    if rule_type != "required_status_checks" {
        return Some(EffectiveRule {
            pull_request: false,
            required_checks: BTreeMap::new(),
        });
    }
    let parameters = object.get("parameters")?.as_object()?;
    let checks = parameters.get("required_status_checks")?.as_array()?;
    let mut required_checks = BTreeMap::new();
    for check in checks {
        let check = check.as_object()?;
        let context = check.get("context")?.as_str()?.to_owned();
        let integration_id = optional_i64(check.get("integration_id"))?;
        if !merge_check_source(&mut required_checks, context, integration_id) {
            return None;
        }
    }
    Some(EffectiveRule {
        pull_request: false,
        required_checks,
    })
}

fn parse_classic_required_checks(
    protection: &serde_json::Map<String, serde_json::Value>,
) -> Option<BTreeMap<String, Option<i64>>> {
    let value = protection.get("required_status_checks")?;
    if value.is_null() {
        return Some(BTreeMap::new());
    }
    let checks = value.as_object()?;
    let mut sources = BTreeMap::new();
    if let Some(contexts) = checks.get("contexts") {
        for context in contexts.as_array()? {
            if !merge_check_source(&mut sources, context.as_str()?.to_owned(), None) {
                return None;
            }
        }
    }
    if let Some(contexts) = checks.get("checks") {
        for check in contexts.as_array()? {
            let check = check.as_object()?;
            let context = check.get("context")?.as_str()?.to_owned();
            let app_id = optional_i64(check.get("app_id"))?;
            if !merge_check_source(&mut sources, context, app_id) {
                return None;
            }
        }
    }
    Some(sources)
}

fn optional_i64(value: Option<&serde_json::Value>) -> Option<Option<i64>> {
    match value {
        None | Some(serde_json::Value::Null) => Some(None),
        Some(serde_json::Value::Number(value)) => value.as_i64().and_then(|value| match value {
            -1 => Some(None), // Classic branch rules explicitly allow any check provider.
            1.. => Some(Some(value)),
            _ => None,
        }),
        _ => None,
    }
}

fn merge_check_source(
    checks: &mut BTreeMap<String, Option<i64>>,
    context: String,
    app_id: Option<i64>,
) -> bool {
    match checks.get_mut(&context) {
        Some(current) => match (*current, app_id) {
            (Some(current), Some(next)) if current != next => false,
            (None, Some(next)) => {
                *current = Some(next);
                true
            }
            _ => true,
        },
        None => {
            checks.insert(context, app_id);
            true
        }
    }
}

fn classic_bypass_state(protection: &serde_json::Map<String, serde_json::Value>) -> BypassState {
    let Some(enforce_admins) = protection
        .get("enforce_admins")
        .and_then(serde_json::Value::as_object)
        .and_then(|value| value.get("enabled"))
        .and_then(serde_json::Value::as_bool)
    else {
        return BypassState::Unknown;
    };
    if !enforce_admins {
        return BypassState::Present;
    }
    let Some(reviews) = protection.get("required_pull_request_reviews") else {
        return BypassState::NoneVisible;
    };
    if reviews.is_null() {
        return BypassState::NoneVisible;
    }
    let Some(allowances) = reviews
        .as_object()
        .and_then(|value| value.get("bypass_pull_request_allowances"))
        .and_then(serde_json::Value::as_object)
    else {
        return BypassState::Unknown;
    };
    let Some(users) = allowances
        .get("users")
        .and_then(serde_json::Value::as_array)
    else {
        return BypassState::Unknown;
    };
    let Some(teams) = allowances
        .get("teams")
        .and_then(serde_json::Value::as_array)
    else {
        return BypassState::Unknown;
    };
    let Some(apps) = allowances.get("apps").and_then(serde_json::Value::as_array) else {
        return BypassState::Unknown;
    };
    if users.is_empty() && teams.is_empty() && apps.is_empty() {
        BypassState::NoneVisible
    } else {
        BypassState::Present
    }
}

fn ruleset_bypass_state(
    rulesets: &[serde_json::Value],
    branch: &str,
    effective_rules_observed: bool,
) -> BypassState {
    let mut state = BypassState::NoneVisible;
    let mut applicable_ruleset_observed = false;
    for ruleset in rulesets {
        let Some(object) = ruleset.as_object() else {
            return BypassState::Unknown;
        };
        let Some(target) = object.get("target").and_then(serde_json::Value::as_str) else {
            return BypassState::Unknown;
        };
        if target != "branch" {
            continue;
        }
        let Some(enforcement) = object
            .get("enforcement")
            .and_then(serde_json::Value::as_str)
        else {
            return BypassState::Unknown;
        };
        if enforcement == "disabled" || enforcement == "evaluate" {
            continue;
        }
        if enforcement != "active" {
            return BypassState::Unknown;
        }
        match ruleset_applies_to_branch(object.get("conditions"), branch) {
            Some(false) => continue,
            Some(true) => {}
            None => return BypassState::Unknown,
        }
        applicable_ruleset_observed = true;
        let Some(actors) = object
            .get("bypass_actors")
            .and_then(serde_json::Value::as_array)
        else {
            // GitHub withholds this field for callers lacking ruleset write
            // access. Absence therefore cannot prove that no actor can bypass.
            return BypassState::Unknown;
        };
        if !actors.is_empty() {
            return BypassState::Present;
        }
        match object
            .get("current_user_can_bypass")
            .and_then(serde_json::Value::as_str)
        {
            Some("never") => {}
            Some("always" | "pull_requests_only" | "exempt") => {
                state = BypassState::Present;
            }
            _ => return BypassState::Unknown,
        }
    }
    if effective_rules_observed && !applicable_ruleset_observed {
        BypassState::Unknown
    } else {
        state
    }
}

fn ruleset_applies_to_branch(conditions: Option<&serde_json::Value>, branch: &str) -> Option<bool> {
    let Some(conditions) = conditions else {
        return Some(true);
    };
    if conditions.is_null() {
        return Some(true);
    }
    let conditions = conditions.as_object()?;
    let Some(ref_name) = conditions.get("ref_name") else {
        return Some(true);
    };
    let ref_name = ref_name.as_object()?;
    let includes = ref_name.get("include")?.as_array()?;
    let excludes = ref_name.get("exclude")?.as_array()?;
    let expected = format!("refs/heads/{branch}");
    let mut included = false;
    for pattern in includes {
        let pattern = pattern.as_str()?;
        match pattern {
            "~ALL" => included = true,
            "~DEFAULT_BRANCH" => return None,
            value if value == expected => included = true,
            value if value.contains(['*', '?', '[', ']']) => return None,
            _ => {}
        }
    }
    if !included {
        return Some(false);
    }
    for pattern in excludes {
        let pattern = pattern.as_str()?;
        if pattern == expected || pattern == "~ALL" {
            return Some(false);
        }
        if pattern == "~DEFAULT_BRANCH" || pattern.contains(['*', '?', '[', ']']) {
            return None;
        }
    }
    Some(true)
}

fn combine_bypass_state(first: BypassState, second: BypassState) -> BypassState {
    match (first, second) {
        (BypassState::Present, _) | (_, BypassState::Present) => BypassState::Present,
        (BypassState::Unknown, _) | (_, BypassState::Unknown) => BypassState::Unknown,
        _ => BypassState::NoneVisible,
    }
}

fn bypass_evidence(state: BypassState) -> &'static str {
    match state {
        BypassState::NoneVisible => "bypass=none-visible",
        BypassState::Present => "bypass=present",
        BypassState::Unknown => "bypass=unknown",
    }
}

fn supported_branch_name(branch: &str) -> bool {
    !branch.is_empty()
        && branch.len() <= 128
        && !branch.contains('*')
        && !branch.bytes().any(|byte| byte.is_ascii_control())
}

fn encode_path_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

enum RepositoryResponse {
    Match,
    Mismatch,
    NotFoundOrDenied,
    RateLimited,
    Denied,
    AuthenticationRequired,
    Unavailable,
}

fn parse_repository_response(
    bytes: &[u8],
    exit_code: Option<i32>,
    expected: &GithubRepository,
) -> RepositoryResponse {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return RepositoryResponse::Unavailable;
    };
    let Some((headers, body)) = text.split_once("\r\n\r\n") else {
        return RepositoryResponse::Unavailable;
    };
    let status_line = headers.lines().next().unwrap_or_default();
    if status_line.contains(" 401 ") {
        return RepositoryResponse::AuthenticationRequired;
    }
    if status_line.contains(" 429 ") {
        return RepositoryResponse::RateLimited;
    }
    if status_line.contains(" 403 ") {
        let rate_limited = headers.lines().any(|line| {
            let (name, value) = line.split_once(':').unwrap_or_default();
            (name.eq_ignore_ascii_case("retry-after") && !value.trim().is_empty())
                || (name.eq_ignore_ascii_case("x-ratelimit-remaining") && value.trim() == "0")
        });
        return if rate_limited {
            RepositoryResponse::RateLimited
        } else {
            RepositoryResponse::Denied
        };
    }
    if status_line.contains(" 404 ") {
        return RepositoryResponse::NotFoundOrDenied;
    }
    if !status_line.contains(" 200 ") {
        return RepositoryResponse::Unavailable;
    }
    if exit_code != Some(0) {
        return RepositoryResponse::Unavailable;
    }
    #[derive(Deserialize)]
    struct Owner {
        login: String,
    }
    #[derive(Deserialize)]
    struct Repository {
        full_name: String,
        name: String,
        owner: Owner,
    }
    let Ok(repo) = serde_json::from_str::<Repository>(body) else {
        return RepositoryResponse::Unavailable;
    };
    let expected_name = format!("{}/{}", expected.owner(), expected.name());
    if repo.full_name.eq_ignore_ascii_case(&expected_name)
        && repo.name.eq_ignore_ascii_case(expected.name())
        && repo.owner.login.eq_ignore_ascii_case(expected.owner())
    {
        RepositoryResponse::Match
    } else {
        RepositoryResponse::Mismatch
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BypassState, GithubAuthState, GithubRepository, RepositoryResponse, classic_bypass_state,
        combine_bypass_state, parse_auth_status, parse_repository_response,
        ruleset_applies_to_branch, ruleset_bypass_state,
    };

    #[test]
    fn remote_parser_accepts_only_github_repository_coordinates() {
        assert_eq!(
            GithubRepository::from_remote_url("https://github.com/owner/repo.git"),
            Some(GithubRepository {
                owner: "owner".into(),
                name: "repo".into()
            })
        );
        assert_eq!(
            GithubRepository::from_remote_url("git@github.com:owner/repo.git"),
            Some(GithubRepository {
                owner: "owner".into(),
                name: "repo".into()
            })
        );
        for remote in [
            "https://evil.example/owner/repo.git",
            "https://github.com@evil.example/owner/repo.git",
            "https://github.com/owner/repo/extra.git",
            "https://github.com/owner/repo?redirect=evil.example",
            "ssh://github.com/owner/repo.git",
        ] {
            assert!(
                GithubRepository::from_remote_url(remote).is_none(),
                "{remote}"
            );
        }
        assert_eq!(
            GithubRepository::from_git_remote_output(
                b"origin\thttps://github.com/owner/repo.git (fetch)\norigin\tgit@github.com:owner/repo.git (push)\n"
            ),
            Some(GithubRepository {
                owner: "owner".into(),
                name: "repo".into()
            })
        );
        assert!(GithubRepository::from_git_remote_output(
            b"origin\thttps://github.com/owner/repo.git (fetch)\nmirror\thttps://github.com/other/repo.git (push)\n"
        )
        .is_none());
    }

    #[test]
    fn auth_parser_requires_active_github_entry_and_ignores_token_fields() {
        let valid = br#"{"hosts":{"github.com":[{"host":"github.com","active":true,"state":"success","token":"must-not-be-returned","login":"private-user"}]}}"#;
        assert_eq!(parse_auth_status(valid), GithubAuthState::Authenticated);
        assert_eq!(
            parse_auth_status(br#"{"hosts":{"github.com":[{"host":"github.com","active":true,"state":"error"}]}}"#),
            GithubAuthState::Unauthenticated
        );
        assert_eq!(
            parse_auth_status(br#"{"hosts":{}}"#),
            GithubAuthState::Unauthenticated
        );
        assert_eq!(parse_auth_status(b"{}"), GithubAuthState::Unknown);
        assert_eq!(parse_auth_status(b"not json"), GithubAuthState::Unknown);
    }

    #[test]
    fn repository_response_classifies_private_404_and_rate_limits_conservatively() {
        assert!(matches!(
            parse_repository_response(
                b"HTTP/2 404 Not Found\r\ncontent-type: application/json\r\n\r\n{}",
                Some(1),
                &GithubRepository {
                    owner: "owner".into(),
                    name: "repo".into()
                }
            ),
            RepositoryResponse::NotFoundOrDenied
        ));
        assert!(matches!(
            parse_repository_response(
                b"HTTP/2 403 Forbidden\r\n\r\n{}",
                Some(1),
                &GithubRepository {
                    owner: "owner".into(),
                    name: "repo".into()
                }
            ),
            RepositoryResponse::Denied
        ));
        assert!(matches!(
            parse_repository_response(
                b"HTTP/2 429 Too Many Requests\r\n\r\n{}",
                Some(1),
                &GithubRepository {
                    owner: "owner".into(),
                    name: "repo".into()
                }
            ),
            RepositoryResponse::RateLimited
        ));
        assert!(matches!(
            parse_repository_response(
                b"HTTP/2 403 Forbidden\r\nretry-after: 30\r\n\r\n{}",
                Some(1),
                &GithubRepository {
                    owner: "owner".into(),
                    name: "repo".into()
                }
            ),
            RepositoryResponse::RateLimited
        ));
        assert!(matches!(
            parse_repository_response(
                b"HTTP/2 401 Unauthorized\r\n\r\n{}",
                Some(1),
                &GithubRepository {
                    owner: "owner".into(),
                    name: "repo".into()
                }
            ),
            RepositoryResponse::AuthenticationRequired
        ));
        assert!(matches!(
            parse_repository_response(b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n{\"full_name\":\"owner/repo\",\"name\":\"repo\",\"owner\":{\"login\":\"owner\"}}", Some(0), &GithubRepository { owner: "owner".into(), name: "repo".into() }),
            RepositoryResponse::Match
        ));
        assert!(matches!(
            parse_repository_response(b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n{\"full_name\":\"different/repo\",\"name\":\"repo\",\"owner\":{\"login\":\"different\"}}", Some(0), &GithubRepository { owner: "owner".into(), name: "repo".into() }),
            RepositoryResponse::Mismatch
        ));
    }

    #[test]
    fn bypass_data_is_tri_state_and_branch_scoped() {
        let no_bypass = serde_json::json!({
            "target": "branch",
            "enforcement": "active",
            "conditions": { "ref_name": { "include": ["refs/heads/main"], "exclude": [] } },
            "bypass_actors": [],
            "current_user_can_bypass": "never"
        });
        assert_eq!(
            ruleset_bypass_state(std::slice::from_ref(&no_bypass), "main", true),
            BypassState::NoneVisible
        );
        assert_eq!(
            ruleset_applies_to_branch(no_bypass.get("conditions"), "main"),
            Some(true)
        );

        let bypass = serde_json::json!({
            "target": "branch",
            "enforcement": "active",
            "conditions": { "ref_name": { "include": ["refs/heads/main"], "exclude": [] } },
            "bypass_actors": [{ "actor_type": "OrganizationAdmin", "bypass_mode": "always" }],
            "current_user_can_bypass": "always"
        });
        assert_eq!(
            ruleset_bypass_state(std::slice::from_ref(&bypass), "main", true),
            BypassState::Present
        );

        let hidden_bypass = serde_json::json!({
            "target": "branch",
            "enforcement": "active",
            "conditions": { "ref_name": { "include": ["refs/heads/main"], "exclude": [] } },
            "current_user_can_bypass": "never"
        });
        assert_eq!(
            ruleset_bypass_state(std::slice::from_ref(&hidden_bypass), "main", true),
            BypassState::Unknown
        );
        assert_eq!(
            ruleset_bypass_state(std::slice::from_ref(&bypass), "release", false),
            BypassState::NoneVisible
        );
        assert_eq!(
            ruleset_bypass_state(&[], "main", true),
            BypassState::Unknown
        );

        let classic_no_bypass = serde_json::json!({
            "enforce_admins": { "enabled": true },
            "required_pull_request_reviews": { "bypass_pull_request_allowances": { "users": [], "teams": [], "apps": [] } }
        });
        let classic_admin_bypass = serde_json::json!({
            "enforce_admins": { "enabled": false },
            "required_pull_request_reviews": null
        });
        assert_eq!(
            classic_bypass_state(classic_no_bypass.as_object().unwrap()),
            BypassState::NoneVisible
        );
        assert_eq!(
            classic_bypass_state(classic_admin_bypass.as_object().unwrap()),
            BypassState::Present
        );
        assert_eq!(
            combine_bypass_state(BypassState::NoneVisible, BypassState::Unknown),
            BypassState::Unknown
        );
    }
}
