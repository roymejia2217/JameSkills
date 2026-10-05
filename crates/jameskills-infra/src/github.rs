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
use std::{ffi::OsString, path::PathBuf, time::Duration};

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
                return self.observation(CheckStatus::Blocked, None, repository, revision);
            }
            GithubAuthState::Unknown => {
                return self.observation(CheckStatus::Unknown, None, repository, revision);
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
                return self.observation(CheckStatus::Unknown, None, repository, revision);
            }
            Err(error) => return Err(error),
        };
        if output.stdout().len() >= API_OUTPUT_LIMIT {
            return self.observation(CheckStatus::Unknown, None, repository, revision);
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
        self.observation(status, enforcement, repository, revision)
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

    fn observation(
        &self,
        status: CheckStatus,
        enforcement: Option<Enforcement>,
        repository: &GithubRepository,
        revision: &RepositoryHead,
    ) -> AppResult<CheckObservation> {
        let capability = if status == CheckStatus::Pass {
            "repo-read"
        } else {
            "unconfirmed"
        };
        let summary = format!(
            "r={}/{};sha={};check=repo;cap={};result={}",
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
        let evidence = CheckEvidence::new(
            "github.repository.identity",
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
        GithubAuthState, GithubRepository, RepositoryResponse, parse_auth_status,
        parse_repository_response,
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
}
