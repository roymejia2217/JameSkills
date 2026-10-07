use crate::{
    agents::{AgentArtifact, find_agent_executable_candidate, plan_file_copy_artifact},
    platform::{PlatformError, opencode_config_directory, user_home_directory},
};
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
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

const OPENCODE_VERSION_ARGS: &[&str] = &["--version"];
const OPENCODE_VERSION_TIMEOUT: Duration = Duration::from_secs(3);
const OPENCODE_VERSION_OUTPUT_LIMIT: usize = 4096;

pub struct OpenCodeAdapter {
    profile: AgentProfile,
    process: std::sync::Arc<dyn ProcessPort>,
    user_home: PathBuf,
    config_directory: PathBuf,
}

impl OpenCodeAdapter {
    pub fn for_current_user(
        profile: AgentProfile,
        process: std::sync::Arc<dyn ProcessPort>,
    ) -> AppResult<Self> {
        let user_home = user_home_directory().map_err(|error| match error {
            PlatformError::BaseDirectoriesUnavailable => AppError::CapabilityUnavailable {
                id: "platform.user-home.unavailable".to_owned(),
                guidance_id: "agent.opencode.home.unavailable".to_owned(),
            },
        })?;
        let xdg_config_home = optional_environment_path("XDG_CONFIG_HOME");
        let config_directory_override = optional_environment_path("OPENCODE_CONFIG_DIR");
        Self::with_paths(
            profile,
            process,
            user_home,
            xdg_config_home,
            config_directory_override,
        )
    }

    pub fn with_paths(
        profile: AgentProfile,
        process: std::sync::Arc<dyn ProcessPort>,
        user_home: PathBuf,
        xdg_config_home: Option<PathBuf>,
        config_directory_override: Option<PathBuf>,
    ) -> AppResult<Self> {
        if profile.id() != AgentId::OpenCode {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "agent.opencode.profile.invalid",
                "OpenCode adapter requires the app-owned OpenCode profile.",
            )]));
        }
        let config_directory = opencode_config_directory(
            &user_home,
            xdg_config_home.as_deref(),
            config_directory_override.as_deref(),
        )?;
        Ok(Self {
            profile,
            process,
            user_home,
            config_directory,
        })
    }

    pub fn from_current_user(
        profile: AgentProfile,
        process: std::sync::Arc<dyn ProcessPort>,
    ) -> AppResult<Self> {
        let user_home = user_home_directory().map_err(|error| match error {
            PlatformError::BaseDirectoriesUnavailable => AppError::CapabilityUnavailable {
                id: "platform.user-home.unavailable".to_owned(),
                guidance_id: "agent.opencode.home.unavailable".to_owned(),
            },
        })?;
        let xdg_config_home = optional_environment_path("XDG_CONFIG_HOME");
        let config_directory_override = optional_environment_path("OPENCODE_CONFIG_DIR");
        Self::with_paths(
            profile,
            process,
            user_home,
            xdg_config_home,
            config_directory_override,
        )
    }

    pub fn plan_artifact(
        &self,
        validated: &ValidatedBundle,
        files: BundleFiles,
        project_root: Option<&ApprovedRoot>,
        scope: Scope,
    ) -> AppResult<AgentArtifact> {
        let target = opencode_skill_target(
            scope,
            &self.config_directory,
            project_root,
            validated.manifest().slug(),
        )?;
        plan_file_copy_artifact(target, validated, files)
    }

    fn profile_root(
        &self,
        scope: Scope,
        project_root: Option<&ApprovedRoot>,
    ) -> AppResult<PathBuf> {
        match scope {
            Scope::User => Ok(self.config_directory.join("skills")),
            Scope::Project => Ok(project_root
                .ok_or_else(|| {
                    AppError::Validation(vec![Diagnostic::error(
                        "agent.opencode.project-root.required",
                        "OpenCode project detection requires an approved repository root.",
                    )])
                })?
                .path()
                .join(".opencode")
                .join("skills")),
        }
    }

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

#[async_trait]
impl AgentPort for OpenCodeAdapter {
    fn profile(&self) -> &AgentProfile {
        &self.profile
    }

    async fn detect(&self, context: DetectionContext) -> AppResult<AgentDetection> {
        let scope = context.scope();
        let profile_root = self.profile_root(scope, context.project_root())?;
        let mut evidence = self.capability_evidence();
        let Some(approved) = context.approved_executable() else {
            let availability =
                if find_agent_executable_candidate(AgentId::OpenCode.executable_name()).is_some() {
                    AgentAvailability::Candidate
                } else {
                    AgentAvailability::Missing
                };
            return AgentDetection::new(
                AgentId::OpenCode,
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
            AgentId::OpenCode,
            OPENCODE_VERSION_ARGS.iter().map(OsString::from).collect(),
            cwd,
            ApprovedEnv::new(BTreeMap::new()).map_err(AppError::Validation)?,
            OPENCODE_VERSION_TIMEOUT,
            OPENCODE_VERSION_OUTPUT_LIMIT,
            ProcessPermission::ReadOnlyCheck,
            CancellationToken::new(),
        )
        .map_err(AppError::Validation)?
        .with_approved_executable_fingerprint(approved.fingerprint());
        let output = self.process.run(spec).await?;
        let parsed_version = if output.exit_code() == Some(0) {
            parse_opencode_version(output.stdout())
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
                    "opencode-cli-source",
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
            AgentId::OpenCode,
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

pub fn opencode_skill_target(
    scope: Scope,
    config_directory: &Path,
    project_root: Option<&ApprovedRoot>,
    slug: &str,
) -> AppResult<PathBuf> {
    let slug = PortablePath::new(slug.to_owned()).map_err(|_| {
        AppError::Validation(vec![Diagnostic::error(
            "agent.opencode.slug.invalid",
            "OpenCode skill slug is not a portable path component.",
        )])
    })?;
    if slug.as_str().contains('/') {
        return Err(AppError::Validation(vec![Diagnostic::error(
            "agent.opencode.slug.invalid",
            "OpenCode skill slug must be exactly one portable path component.",
        )]));
    }
    let skills_root = match scope {
        Scope::User => ApprovedRoot::from_absolute_path(config_directory.to_path_buf())
            .map_err(AppError::Validation)?
            .path()
            .join("skills"),
        Scope::Project => project_root
            .ok_or_else(|| {
                AppError::Validation(vec![Diagnostic::error(
                    "agent.opencode.project-root.required",
                    "OpenCode project export requires an approved repository root.",
                )])
            })?
            .path()
            .join(".opencode")
            .join("skills"),
    };
    Ok(skills_root.join(slug.as_str()))
}

fn parse_opencode_version(output: &[u8]) -> Option<Version> {
    if output.is_empty() || output.len() > OPENCODE_VERSION_OUTPUT_LIMIT || output.contains(&0) {
        return None;
    }
    let line = std::str::from_utf8(output).ok()?.trim();
    let version = line.strip_prefix("opencode ")?;
    if version.is_empty() || version.contains('\r') || version.contains('\n') {
        return None;
    }
    Version::parse(version).ok()
}

fn optional_environment_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}
