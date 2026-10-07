use crate::{
    agents::{AgentArtifact, find_agent_executable_candidate, plan_file_copy_artifact},
    platform::{PlatformError, pi_agent_directory, user_home_directory},
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

const PI_VERSION_ARGS: &[&str] = &["--version"];
const PI_VERSION_FIXTURE: &str = "1.0.4";
const PI_VERSION_TIMEOUT: Duration = Duration::from_secs(3);
const PI_VERSION_OUTPUT_LIMIT: usize = 4096;

pub struct PiAdapter {
    profile: AgentProfile,
    process: std::sync::Arc<dyn ProcessPort>,
    user_home: PathBuf,
    agent_directory: PathBuf,
}

impl PiAdapter {
    pub fn from_current_user(
        profile: AgentProfile,
        process: std::sync::Arc<dyn ProcessPort>,
    ) -> AppResult<Self> {
        let user_home = user_home_directory().map_err(|error| match error {
            PlatformError::BaseDirectoriesUnavailable => AppError::CapabilityUnavailable {
                id: "platform.user-home.unavailable".to_owned(),
                guidance_id: "agent.pi.home.unavailable".to_owned(),
            },
        })?;
        let agent_directory_override = env::var_os("PI_CODING_AGENT_DIR").map(PathBuf::from);
        Self::with_paths(profile, process, user_home, agent_directory_override)
    }

    pub fn with_paths(
        profile: AgentProfile,
        process: std::sync::Arc<dyn ProcessPort>,
        user_home: PathBuf,
        agent_directory_override: Option<PathBuf>,
    ) -> AppResult<Self> {
        if profile.id() != AgentId::Pi {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "agent.pi.profile.invalid",
                "Pi adapter requires the app-owned Pi profile.",
            )]));
        }
        let agent_directory = pi_agent_directory(&user_home, agent_directory_override.as_deref())?;
        Ok(Self {
            profile,
            process,
            user_home,
            agent_directory,
        })
    }

    pub fn plan_artifact(
        &self,
        validated: &ValidatedBundle,
        files: BundleFiles,
        project_root: Option<&ApprovedRoot>,
        scope: Scope,
    ) -> AppResult<AgentArtifact> {
        let target = pi_skill_target(
            scope,
            &self.agent_directory,
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
            Scope::User => Ok(self.agent_directory.join("skills")),
            Scope::Project => Ok(project_root
                .ok_or_else(|| {
                    AppError::Validation(vec![Diagnostic::error(
                        "agent.pi.project-root.required",
                        "Pi project detection requires an approved repository root.",
                    )])
                })?
                .path()
                .join(".pi")
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
impl AgentPort for PiAdapter {
    fn profile(&self) -> &AgentProfile {
        &self.profile
    }

    async fn detect(&self, context: DetectionContext) -> AppResult<AgentDetection> {
        let scope = context.scope();
        let profile_root = self.profile_root(scope, context.project_root())?;
        let mut evidence = self.capability_evidence();
        let Some(approved) = context.approved_executable() else {
            let availability =
                if find_agent_executable_candidate(AgentId::Pi.executable_name()).is_some() {
                    AgentAvailability::Candidate
                } else {
                    AgentAvailability::Missing
                };
            return AgentDetection::new(
                AgentId::Pi,
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
            AgentId::Pi,
            PI_VERSION_ARGS.iter().map(OsString::from).collect(),
            cwd,
            ApprovedEnv::new(BTreeMap::new()).map_err(AppError::Validation)?,
            PI_VERSION_TIMEOUT,
            PI_VERSION_OUTPUT_LIMIT,
            ProcessPermission::ReadOnlyCheck,
            CancellationToken::new(),
        )
        .map_err(AppError::Validation)?
        .with_approved_executable_fingerprint(approved.fingerprint());
        let output = self.process.run(spec).await?;
        let parsed_version = if output.exit_code() == Some(0) {
            parse_pi_version(output.stdout())
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
                    "pi-cli-source",
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
            AgentId::Pi,
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

pub fn pi_skill_target(
    scope: Scope,
    agent_directory: &Path,
    project_root: Option<&ApprovedRoot>,
    slug: &str,
) -> AppResult<PathBuf> {
    let slug = PortablePath::new(slug.to_owned()).map_err(|_| {
        AppError::Validation(vec![Diagnostic::error(
            "agent.pi.slug.invalid",
            "Pi skill slug is not a portable path component.",
        )])
    })?;
    if slug.as_str().contains('/') {
        return Err(AppError::Validation(vec![Diagnostic::error(
            "agent.pi.slug.invalid",
            "Pi skill slug must be exactly one portable path component.",
        )]));
    }
    let skills_root = match scope {
        Scope::User => ApprovedRoot::from_absolute_path(agent_directory.to_path_buf())
            .map_err(AppError::Validation)?
            .path()
            .join("skills"),
        Scope::Project => project_root
            .ok_or_else(|| {
                AppError::Validation(vec![Diagnostic::error(
                    "agent.pi.project-root.required",
                    "Pi project export requires an approved repository root.",
                )])
            })?
            .path()
            .join(".pi")
            .join("skills"),
    };
    Ok(skills_root.join(slug.as_str()))
}

fn parse_pi_version(output: &[u8]) -> Option<Version> {
    if output.is_empty() || output.len() > PI_VERSION_OUTPUT_LIMIT || output.contains(&0) {
        return None;
    }
    let text = std::str::from_utf8(output).ok()?.trim();
    if text.is_empty() || text.contains('\r') || text.contains('\n') {
        return None;
    }
    let version = Version::parse(text).ok()?;
    (version.to_string() == PI_VERSION_FIXTURE).then_some(version)
}
