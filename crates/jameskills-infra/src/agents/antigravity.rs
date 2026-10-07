use crate::{
    agents::{AgentArtifact, find_agent_executable_candidate, plan_file_copy_artifact},
    platform::antigravity_cli_install_candidate,
};
use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult, Diagnostic,
    domain::{AgentId, AgentProfile, PortablePath, Scope, ValidatedBundle},
    ports::{
        agent::{AgentAvailability, AgentDetection, AgentPort, DetectionContext},
        filesystem::BundleFiles,
        process::ApprovedExecutable,
    },
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub struct AntigravityAdapter {
    profile: AgentProfile,
    user_home: PathBuf,
}

pub struct AntigravityPluginArtifact {
    plugin_name: String,
    artifact: AgentArtifact,
}

#[derive(Serialize)]
struct PluginManifest<'a> {
    name: &'a str,
    description: &'a str,
}

impl AntigravityAdapter {
    pub fn new(profile: AgentProfile, user_home: PathBuf) -> AppResult<Self> {
        if profile.id() != AgentId::Antigravity || !user_home.is_absolute() {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "agent.antigravity.profile.invalid",
                "Antigravity adapter requires its app-owned profile and an absolute user home.",
            )]));
        }
        Ok(Self { profile, user_home })
    }

    pub fn plan_plugin_artifact(
        &self,
        validated: &ValidatedBundle,
        files: BundleFiles,
        scope: Scope,
    ) -> AppResult<AntigravityPluginArtifact> {
        if scope != Scope::User {
            return Err(AppError::CapabilityUnavailable {
                id: "agent.antigravity.project-scope.unsupported".to_owned(),
                guidance_id: "agent.antigravity.user-plugin-only".to_owned(),
            });
        }
        let slug = portable_slug(validated.manifest().slug())?;
        let plugin_name = format!("jameskills-{slug}");
        let plugin_root = self
            .user_home
            .join(".gemini")
            .join("antigravity-cli")
            .join("plugins")
            .join(&plugin_name);
        let skill_target = plugin_root.join("skills").join(&slug);
        let skill = plan_file_copy_artifact(skill_target, validated, files)?;
        let manifest = PluginManifest {
            name: &plugin_name,
            description: validated.manifest().description(),
        };
        let manifest_bytes = serde_json::to_vec(&manifest).map_err(|_| AppError::Storage {
            code: "agent.antigravity.manifest.serialize.failed".to_owned(),
        })?;
        let manifest_path = PortablePath::new("plugin.json".to_owned()).map_err(|_| {
            AppError::Validation(vec![Diagnostic::error(
                "agent.antigravity.path.invalid",
                "Antigravity plugin artifact path is invalid.",
            )])
        })?;
        let mut artifact_files = BTreeMap::new();
        artifact_files.insert(manifest_path, manifest_bytes);
        for (path, contents) in skill.files() {
            let plugin_path = PortablePath::new(format!("skills/{slug}/{}", path.as_str()))
                .map_err(|_| {
                    AppError::Validation(vec![Diagnostic::error(
                        "agent.antigravity.path.invalid",
                        "Antigravity plugin artifact path is invalid.",
                    )])
                })?;
            artifact_files.insert(plugin_path, contents.clone());
        }
        Ok(AntigravityPluginArtifact {
            plugin_name,
            artifact: AgentArtifact::from_app_owned_files(plugin_root, artifact_files),
        })
    }

    fn profile_root(&self, scope: Scope) -> Option<PathBuf> {
        (scope == Scope::User).then(|| {
            self.user_home
                .join(".gemini")
                .join("antigravity-cli")
                .join("plugins")
        })
    }

    fn candidate_exists(&self) -> bool {
        find_agent_executable_candidate(AgentId::Antigravity.executable_name()).is_some()
            || antigravity_cli_install_candidate(&self.user_home)
                .as_deref()
                .is_some_and(is_regular_non_link)
    }
}

impl AntigravityPluginArtifact {
    pub fn plugin_name(&self) -> &str {
        &self.plugin_name
    }

    pub fn target(&self) -> &Path {
        self.artifact.target()
    }

    pub fn files(&self) -> &BundleFiles {
        self.artifact.files()
    }
}

#[async_trait]
impl AgentPort for AntigravityAdapter {
    fn profile(&self) -> &AgentProfile {
        &self.profile
    }

    async fn detect(&self, context: DetectionContext) -> AppResult<AgentDetection> {
        let availability = if context.approved_executable().is_some() {
            // The current official contract documents plugin commands, but does not
            // provide a reviewed version probe/output fixture for the installed CLI.
            AgentAvailability::Blocked
        } else if self.candidate_exists() {
            AgentAvailability::Candidate
        } else {
            AgentAvailability::Missing
        };
        let approved = context.approved_executable();
        let executable = approved
            .map(|value| {
                ApprovedExecutable::from_absolute_path(value.executable().path().to_path_buf())
                    .map_err(AppError::Validation)
            })
            .transpose()?;
        AgentDetection::new(
            AgentId::Antigravity,
            executable,
            approved.map(|value| value.fingerprint()),
            None,
            self.profile_root(context.scope()),
            availability,
            self.profile.capabilities().clone(),
            self.profile_evidence(),
        )
        .map_err(AppError::Validation)
    }
}

impl AntigravityAdapter {
    fn profile_evidence(&self) -> Vec<jameskills_core::domain::agent::CapabilityEvidence> {
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

fn portable_slug(slug: &str) -> AppResult<String> {
    let portable = PortablePath::new(slug.to_owned()).map_err(|_| {
        AppError::Validation(vec![Diagnostic::error(
            "agent.antigravity.slug.invalid",
            "Antigravity plugin slug must be one portable path component.",
        )])
    })?;
    if portable.as_str().contains('/') {
        return Err(AppError::Validation(vec![Diagnostic::error(
            "agent.antigravity.slug.invalid",
            "Antigravity plugin slug must be one portable path component.",
        )]));
    }
    Ok(portable.as_str().to_owned())
}

fn is_regular_non_link(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 == 0
    }
    #[cfg(not(windows))]
    {
        true
    }
}
