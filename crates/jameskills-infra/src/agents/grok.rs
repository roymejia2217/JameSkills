use crate::{
    agents::{AgentArtifact, find_agent_executable_candidate, plan_file_copy_artifact},
    platform::{grok_home_directory, user_home_directory},
};
use jameskills_core::{
    AppError, AppResult, Diagnostic,
    domain::{AgentId, AgentProfile, CapabilityEvidence, PortablePath, Scope, ValidatedBundle},
    ports::{
        agent::{AgentAvailability, AgentDetection, AgentPort, DetectionContext},
        filesystem::BundleFiles,
        process::{ApprovedExecutable, ApprovedRoot},
    },
};
use std::{
    env,
    path::{Path, PathBuf},
};

pub struct GrokAdapter {
    profile: AgentProfile,
    user_home: PathBuf,
    grok_home: PathBuf,
}

impl GrokAdapter {
    pub fn from_current_user(profile: AgentProfile) -> AppResult<Self> {
        let user_home = user_home_directory().map_err(|error| match error {
            crate::platform::PlatformError::BaseDirectoriesUnavailable => {
                AppError::CapabilityUnavailable {
                    id: "platform.user-home.unavailable".to_owned(),
                    guidance_id: "agent.grok.home.unavailable".to_owned(),
                }
            }
        })?;
        let grok_home_override = env::var_os("GROK_HOME").filter(|value| !value.is_empty());
        Self::with_paths(profile, user_home, grok_home_override.map(PathBuf::from))
    }

    pub fn with_paths(
        profile: AgentProfile,
        user_home: PathBuf,
        grok_home_override: Option<PathBuf>,
    ) -> AppResult<Self> {
        if profile.id() != AgentId::Grok {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "agent.grok.profile.invalid",
                "Grok adapter requires the app-owned Grok Build profile.",
            )]));
        }
        let grok_home = grok_home_directory(&user_home, grok_home_override.as_deref())?;
        Ok(Self {
            profile,
            user_home,
            grok_home,
        })
    }

    pub fn profile(&self) -> &AgentProfile {
        &self.profile
    }

    pub fn plan_artifact(
        &self,
        validated: &ValidatedBundle,
        files: BundleFiles,
        project_root: Option<&ApprovedRoot>,
        scope: Scope,
    ) -> AppResult<AgentArtifact> {
        let target = grok_skill_target(
            scope,
            &self.grok_home,
            project_root,
            validated.manifest().slug(),
        )?;
        plan_file_copy_artifact(target, validated, files)
    }

    pub fn user_home(&self) -> &Path {
        &self.user_home
    }

    fn profile_root(
        &self,
        scope: Scope,
        project_root: Option<&ApprovedRoot>,
    ) -> AppResult<PathBuf> {
        match scope {
            Scope::User => Ok(self.grok_home.join("skills")),
            Scope::Project => Ok(project_root
                .ok_or_else(|| {
                    AppError::Validation(vec![Diagnostic::error(
                        "agent.grok.project-root.required",
                        "Grok project detection requires an approved repository root.",
                    )])
                })?
                .path()
                .join(".grok")
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

#[async_trait::async_trait]
impl AgentPort for GrokAdapter {
    fn profile(&self) -> &AgentProfile {
        &self.profile
    }

    async fn detect(&self, context: DetectionContext) -> AppResult<AgentDetection> {
        let profile_root = self.profile_root(context.scope(), context.project_root())?;
        let evidence = self.capability_evidence();
        let approved = context.approved_executable();
        let availability = if approved.is_some() {
            // `grok version` is documented, but no reviewed output/version fixture
            // is available yet; never elevate a selected binary on name alone.
            AgentAvailability::Blocked
        } else if find_agent_executable_candidate(AgentId::Grok.executable_name()).is_some() {
            AgentAvailability::Candidate
        } else {
            AgentAvailability::Missing
        };
        let executable = approved
            .map(|approval| {
                ApprovedExecutable::from_absolute_path(approval.executable().path().to_path_buf())
                    .map_err(AppError::Validation)
            })
            .transpose()?;
        AgentDetection::new(
            AgentId::Grok,
            executable,
            approved.map(|approval| approval.fingerprint()),
            None,
            Some(profile_root),
            availability,
            self.profile.capabilities().clone(),
            evidence,
        )
        .map_err(AppError::Validation)
    }
}

pub fn grok_skill_target(
    scope: Scope,
    grok_home: &Path,
    project_root: Option<&ApprovedRoot>,
    slug: &str,
) -> AppResult<PathBuf> {
    let slug = PortablePath::new(slug.to_owned()).map_err(|_| {
        AppError::Validation(vec![Diagnostic::error(
            "agent.grok.slug.invalid",
            "Grok skill slug is not a portable path component.",
        )])
    })?;
    if slug.as_str().contains('/') {
        return Err(AppError::Validation(vec![Diagnostic::error(
            "agent.grok.slug.invalid",
            "Grok skill slug must be exactly one portable path component.",
        )]));
    }
    let skills_root = match scope {
        Scope::User => ApprovedRoot::from_absolute_path(grok_home.to_path_buf())
            .map_err(AppError::Validation)?
            .path()
            .join("skills"),
        Scope::Project => project_root
            .ok_or_else(|| {
                AppError::Validation(vec![Diagnostic::error(
                    "agent.grok.project-root.required",
                    "Grok project export requires an approved repository root.",
                )])
            })?
            .path()
            .join(".grok")
            .join("skills"),
    };
    Ok(skills_root.join(slug.as_str()))
}
