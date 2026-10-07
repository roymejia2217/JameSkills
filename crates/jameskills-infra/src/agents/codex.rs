use super::{AgentArtifact, find_agent_executable_candidate, plan_file_copy_artifact};
use crate::platform::{PlatformError, user_home_directory};
use async_trait::async_trait;
use chrono::{SecondsFormat, Utc};
use jameskills_core::{
    AppError, AppResult, Diagnostic,
    domain::{AgentId, AgentProfile, CapabilityEvidence, PortablePath, Scope, ValidatedBundle},
    ports::{
        agent::{AgentAvailability, AgentDetection, AgentPort, DetectionContext},
        filesystem::BundleFiles,
        process::{
            ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken, ProcessPermission,
            ProcessPort, ProcessSpec,
        },
    },
};
use semver::Version;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

const CODEX_VERSION_ARGS: &[&str] = &["--version"];
const CODEX_VERSION_TIMEOUT: Duration = Duration::from_secs(3);
const CODEX_VERSION_OUTPUT_LIMIT: usize = 4096;

pub struct CodexAdapter {
    profile: AgentProfile,
    process: std::sync::Arc<dyn ProcessPort>,
    user_home: PathBuf,
}

impl CodexAdapter {
    pub fn new(
        profile: AgentProfile,
        process: std::sync::Arc<dyn ProcessPort>,
        user_home: PathBuf,
    ) -> AppResult<Self> {
        if profile.id() != AgentId::Codex || !user_home.is_absolute() {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "agent.codex.profile.invalid",
                "Codex adapter requires its app-owned profile and an absolute user home.",
            )]));
        }
        Ok(Self {
            profile,
            process,
            user_home,
        })
    }

    pub fn profile(&self) -> &AgentProfile {
        &self.profile
    }

    pub fn plan_artifact(
        &self,
        validated: &ValidatedBundle,
        files: BundleFiles,
        user_home: &Path,
        project_root: Option<&ApprovedRoot>,
        scope: Scope,
    ) -> AppResult<AgentArtifact> {
        let target =
            codex_skill_target(scope, user_home, project_root, validated.manifest().slug())?;
        plan_file_copy_artifact(target, validated, files)
    }

    pub fn plan_artifact_for_current_user(
        &self,
        validated: &ValidatedBundle,
        files: BundleFiles,
        project_root: Option<&ApprovedRoot>,
        scope: Scope,
    ) -> AppResult<AgentArtifact> {
        let home = match scope {
            Scope::User => user_home_directory().map_err(|error| match error {
                PlatformError::BaseDirectoriesUnavailable => AppError::CapabilityUnavailable {
                    id: "platform.user-home.unavailable".to_owned(),
                    guidance_id: "agent.codex.home.unavailable".to_owned(),
                },
            })?,
            Scope::Project => project_root
                .ok_or_else(|| {
                    AppError::Validation(vec![Diagnostic::error(
                        "agent.codex.project-root.required",
                        "Codex project export requires an approved repository root.",
                    )])
                })?
                .path()
                .to_path_buf(),
        };
        self.plan_artifact(validated, files, &home, project_root, scope)
    }
}

#[async_trait]
impl AgentPort for CodexAdapter {
    fn profile(&self) -> &AgentProfile {
        &self.profile
    }

    async fn detect(&self, context: DetectionContext) -> AppResult<AgentDetection> {
        let scope = context.scope();
        let profile_root = match scope {
            Scope::User => self.user_home.join(".agents").join("skills"),
            Scope::Project => context
                .project_root()
                .ok_or_else(|| {
                    AppError::Validation(vec![Diagnostic::error(
                        "agent.codex.project-root.required",
                        "Codex project detection requires an approved repository root.",
                    )])
                })?
                .path()
                .join(".agents")
                .join("skills"),
        };
        let mut evidence = self.capability_evidence();
        let Some(approved) = context.approved_executable() else {
            let availability =
                if find_agent_executable_candidate(AgentId::Codex.executable_name()).is_some() {
                    AgentAvailability::Candidate
                } else {
                    AgentAvailability::Missing
                };
            return AgentDetection::new(
                AgentId::Codex,
                None,
                None,
                None,
                Some(profile_root),
                availability,
                self.profile.capabilities().clone(),
                evidence,
            )
            .map_err(AppError::Validation);
        };

        let executable =
            ApprovedExecutable::from_absolute_path(approved.executable().path().to_path_buf())
                .map_err(AppError::Validation)?;
        let cwd = ApprovedRoot::from_absolute_path(self.user_home.clone())
            .map_err(AppError::Validation)?;
        let spec = ProcessSpec::new_for_agent(
            executable,
            AgentId::Codex,
            CODEX_VERSION_ARGS.iter().map(OsString::from).collect(),
            cwd,
            ApprovedEnv::new(BTreeMap::new()).map_err(AppError::Validation)?,
            CODEX_VERSION_TIMEOUT,
            CODEX_VERSION_OUTPUT_LIMIT,
            ProcessPermission::ReadOnlyCheck,
            CancellationToken::new(),
        )
        .map_err(AppError::Validation)?
        .with_approved_executable_fingerprint(approved.fingerprint());
        let output = self.process.run(spec).await?;
        let parsed_version = if output.exit_code() == Some(0) {
            parse_codex_version(output.stdout())
        } else {
            None
        };
        let availability = if parsed_version.is_some() {
            AgentAvailability::Verified
        } else {
            AgentAvailability::Blocked
        };
        if let Some(version) = parsed_version.as_ref() {
            evidence.push(
                CapabilityEvidence::new(
                    "codex-cli-source",
                    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
                    Some(version.clone()),
                    None,
                )
                .map_err(AppError::Validation)?,
            );
        }
        let executable =
            ApprovedExecutable::from_absolute_path(approved.executable().path().to_path_buf())
                .map_err(AppError::Validation)?;
        AgentDetection::new(
            AgentId::Codex,
            Some(executable),
            Some(approved.fingerprint()),
            parsed_version,
            Some(profile_root),
            availability,
            self.profile.capabilities().clone(),
            evidence,
        )
        .map_err(AppError::Validation)
    }
}

impl CodexAdapter {
    fn capability_evidence(&self) -> Vec<CapabilityEvidence> {
        jameskills_core::domain::agent::AgentCapabilityId::ALL
            .into_iter()
            .filter_map(|id| {
                self.profile
                    .capabilities()
                    .assessment(id)
                    .evidence()
                    .cloned()
            })
            .collect()
    }
}

pub fn codex_skill_target(
    scope: Scope,
    user_home: &Path,
    project_root: Option<&ApprovedRoot>,
    slug: &str,
) -> AppResult<PathBuf> {
    let slug = PortablePath::new(slug.to_owned()).map_err(|_| {
        AppError::Validation(vec![Diagnostic::error(
            "agent.codex.slug.invalid",
            "Codex skill slug is not a portable path component.",
        )])
    })?;
    if slug.as_str().contains('/') {
        return Err(AppError::Validation(vec![Diagnostic::error(
            "agent.codex.slug.invalid",
            "Codex skill slug must be exactly one portable path component.",
        )]));
    }
    let skills_root = match scope {
        Scope::User => ApprovedRoot::from_absolute_path(user_home.to_path_buf())
            .map_err(AppError::Validation)?
            .path()
            .to_path_buf(),
        Scope::Project => project_root
            .ok_or_else(|| {
                AppError::Validation(vec![Diagnostic::error(
                    "agent.codex.project-root.required",
                    "Codex project export requires an approved repository root.",
                )])
            })?
            .path()
            .to_path_buf(),
    };
    Ok(skills_root
        .join(".agents")
        .join("skills")
        .join(slug.as_str()))
}

fn parse_codex_version(output: &[u8]) -> Option<Version> {
    if output.is_empty() || output.len() > CODEX_VERSION_OUTPUT_LIMIT || output.contains(&0) {
        return None;
    }
    let text = std::str::from_utf8(output).ok()?;
    let line = text.trim();
    let version = line.strip_prefix("codex ")?;
    if version.is_empty() || version.contains('\r') || version.contains('\n') {
        return None;
    }
    Version::parse(version).ok()
}
