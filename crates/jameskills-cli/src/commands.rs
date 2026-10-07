use clap::{Parser, Subcommand, ValueEnum};
use jameskills_core::SkillId;
use jameskills_core::domain::{
    ToolId, ToolOperation,
    guidance::{ToolAvailability, ToolCapabilitySupport, ToolDetection, ToolVersionStatus},
};
use jameskills_infra::{
    composition::RuntimeServices,
    platform::{
        HostPlatform, Observation, ToolCandidateKind, find_tool_candidates, load_tool_profiles,
    },
};
use serde_json::json;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
#[command(
    name = "jameskills",
    version,
    about = "Portable, verifiable coding-agent skills"
)]
pub struct Cli {
    /// Emit the stable JSON response envelope.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

impl Cli {
    pub fn parse<I, T>(args: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Self::try_parse_from(args)
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum CliCommand {
    /// Inspect platform facts observed by this process.
    Doctor,
    /// Validate a portable skill bundle.
    Validate {
        #[arg(long)]
        path: PathBuf,
    },
    /// Evaluate a skill against a repository.
    Check {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        skill: SkillId,
        #[arg(long, value_enum, default_value_t = Profile::Generic)]
        profile: Profile,
        /// Return failure unless every required check passes.
        #[arg(long)]
        strict: bool,
    },
    /// Manage the local skill library.
    Library {
        #[command(subcommand)]
        command: LibraryCommand,
    },
    /// Detect supported code agents.
    Agents {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// Plan and apply skill installations.
    Install {
        #[command(subcommand)]
        command: InstallCommand,
    },
    /// Export or restore an encrypted library backup.
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    /// Inspect or synchronize the encrypted cloud vault.
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Profile {
    Rust,
    Node,
    #[default]
    Generic,
}

#[derive(Clone, Debug, Subcommand)]
pub enum LibraryCommand {
    /// List locally stored skills.
    List,
    /// Import a reviewed portable bundle.
    Import {
        #[arg(long)]
        path: PathBuf,
    },
    /// Export a skill to a portable bundle.
    Export {
        #[arg(long)]
        skill: SkillId,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum AgentCommand {
    /// Detect installed code agents and verified capabilities.
    Detect,
}

#[derive(Clone, Debug, Subcommand)]
pub enum InstallCommand {
    /// Preview an installation plan.
    Plan {
        #[arg(long)]
        skill: SkillId,
        #[arg(long)]
        agent: String,
        #[arg(long, value_enum)]
        scope: InstallScope,
        #[arg(long)]
        repo: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Apply a plan after explicit digest confirmation.
    Apply {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        confirm_digest: String,
    },
    /// Remove a previously recorded installation.
    Remove {
        #[arg(long)]
        receipt: String,
        #[arg(long)]
        confirm_digest: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum InstallScope {
    User,
    Project,
}

#[derive(Clone, Debug, Subcommand)]
pub enum BackupCommand {
    /// Export a passphrase-encrypted offline backup.
    Export {
        #[arg(long)]
        output: PathBuf,
    },
    /// Preview or apply a restore; passphrases are prompted by a secure provider.
    Restore {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, conflicts_with = "apply")]
        preview: bool,
        #[arg(long, conflicts_with = "preview", requires = "confirm_digest")]
        apply: bool,
        #[arg(long, requires = "apply")]
        confirm_digest: Option<String>,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum SyncCommand {
    /// Show local vault and synchronization state.
    Status,
    /// Synchronize with the configured cloud vault.
    Run,
}

impl CliCommand {
    pub fn command_name(&self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::Validate { .. } => "validate",
            Self::Check { .. } => "check",
            Self::Library { command } => match command {
                LibraryCommand::List => "library list",
                LibraryCommand::Import { .. } => "library import",
                LibraryCommand::Export { .. } => "library export",
            },
            Self::Agents { command } => match command {
                AgentCommand::Detect => "agents detect",
            },
            Self::Install { command } => match command {
                InstallCommand::Plan { .. } => "install plan",
                InstallCommand::Apply { .. } => "install apply",
                InstallCommand::Remove { .. } => "install remove",
            },
            Self::Backup { command } => match command {
                BackupCommand::Export { .. } => "backup export",
                BackupCommand::Restore { .. } => "backup restore",
            },
            Self::Sync { command } => match command {
                SyncCommand::Status => "sync status",
                SyncCommand::Run => "sync run",
            },
        }
    }
}

pub fn dispatch_cli(cli: Cli, runtime: Option<&RuntimeServices>) -> super::output::CliResponse {
    let Some(command) = cli.command else {
        return super::output::CliResponse::error(
            "help",
            "command.required",
            "Choose a command from the help output.",
            2,
        );
    };

    match command {
        CliCommand::Doctor => {
            let Some(runtime) = runtime else {
                return super::output::CliResponse::unsupported("doctor");
            };
            let facts = runtime.facts();
            let (tools, guidance) = match doctor_tool_inventory(runtime) {
                Ok(inventory) => inventory,
                Err(()) => {
                    return super::output::CliResponse::error(
                        "doctor",
                        "tool.registry.unavailable",
                        "The application tool registry is unavailable.",
                        3,
                    );
                }
            };
            let data = json!({
                "platform": platform_name(facts.platform),
                "architecture": facts.architecture,
                "display_environment": observation_name(facts.display_environment),
                "gpu_device": observation_name(facts.gpu_device),
                "tools": tools,
                "guidance": guidance,
            });
            super::output::CliResponse::success("doctor", data)
        }
        CliCommand::Validate { path } => {
            let Some(runtime) = runtime else {
                return super::output::CliResponse::unsupported("validate");
            };
            match runtime.library().validate_import(&path) {
                Ok(bundle) => {
                    let manifest = bundle.manifest();
                    let data = json!({
                        "valid": true,
                        "skill_id": manifest.id().as_uuid().to_string(),
                        "slug": manifest.slug(),
                        "version": manifest.version().to_string(),
                        "file_count": bundle.file_count(),
                        "content_hash": bundle.content_hash().as_str(),
                        "warnings": [],
                    });
                    super::output::CliResponse::success("validate", data)
                }
                Err(diagnostics) => super::output::CliResponse::validation_failure(diagnostics),
            }
        }
        command => super::output::CliResponse::unsupported(command.command_name()),
    }
}

fn doctor_tool_inventory(
    runtime: &RuntimeServices,
) -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>), ()> {
    let profiles = load_tool_profiles().map_err(|_| ())?;
    let facts = runtime.facts();
    let search_paths = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    let candidates = find_tool_candidates(&profiles, &search_paths, facts.platform);
    let observed_at = runtime.clock().now_utc();
    let mut tools = Vec::with_capacity(profiles.len());
    let mut guidance = Vec::new();

    for (profile, candidate) in profiles.iter().zip(candidates) {
        let (availability, summary) = match candidate.kind() {
            ToolCandidateKind::Missing => (
                ToolAvailability::Missing,
                "No registered executable candidate was found.",
            ),
            ToolCandidateKind::NativeExecutable => (
                ToolAvailability::Candidate,
                "Native candidate requires an approved fingerprint before version probing.",
            ),
            ToolCandidateKind::CommandShim => (
                ToolAvailability::Blocked,
                "Command shim is not executed by the registered version probe.",
            ),
        };
        let detection = ToolDetection::from_probe(
            profile.tool_id(),
            availability,
            None,
            profile.version_range(),
            profile.operations(),
            profile.source_id(),
            &observed_at,
            summary,
        )
        .map_err(|_| ())?;
        let capabilities = detection
            .capabilities()
            .iter()
            .map(|(operation, support)| {
                (
                    operation_name(*operation).to_owned(),
                    serde_json::Value::String(capability_name(*support).to_owned()),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let installation_guide =
            profile
                .install_guides()
                .for_platform(facts.platform)
                .map(|guide| {
                    json!({
                        "source_id": guide.source_id(),
                        "url": guide.url(),
                    })
                });
        let next_step = match detection.availability() {
            ToolAvailability::Missing => Some(json!({
                "tool_id": tool_name(detection.tool_id()),
                "prompt_es": "No se encontró esta herramienta. Consulta la guía oficial, instala manualmente y vuelve a ejecutar doctor.",
                "action": installation_guide.clone().map(|guide| json!({
                    "kind": "open-official-url",
                    "source_id": guide["source_id"],
                    "url": guide["url"],
                })).unwrap_or_else(|| json!({"kind":"manual-instruction"})),
            })),
            ToolAvailability::Candidate => Some(json!({
                "tool_id": tool_name(detection.tool_id()),
                "prompt_es": "Se encontró un candidato nativo; requiere aprobación explícita de su fingerprint antes de comprobar versión o capacidades.",
                "action": {"kind":"manual-instruction"},
            })),
            ToolAvailability::Blocked => Some(json!({
                "tool_id": tool_name(detection.tool_id()),
                "prompt_es": "El candidato es un shim bloqueado; selecciona una instalación nativa desde la guía oficial.",
                "action": installation_guide.clone().map(|guide| json!({
                    "kind": "open-official-url",
                    "source_id": guide["source_id"],
                    "url": guide["url"],
                })).unwrap_or_else(|| json!({"kind":"manual-instruction"})),
            })),
            ToolAvailability::Unknown => Some(json!({
                "tool_id": tool_name(detection.tool_id()),
                "prompt_es": "No hay evidencia suficiente para determinar la disponibilidad de esta herramienta.",
                "action": {"kind":"manual-instruction"},
            })),
            ToolAvailability::Verified => None,
        };
        if let Some(next_step) = next_step {
            guidance.push(next_step);
        }
        tools.push(json!({
            "id": tool_name(detection.tool_id()),
            "availability": availability_name(detection.availability()),
            "version": detection.version().map(ToString::to_string),
            "version_status": version_status_name(detection.version_status()),
            "capabilities": capabilities,
            "evidence": {
                "source_id": detection.evidence().source_id(),
                "summary": detection.evidence().summary(),
            },
        }));
    }
    Ok((tools, guidance))
}

fn tool_name(tool: ToolId) -> &'static str {
    match tool {
        ToolId::Git => "git",
        ToolId::Gitleaks => "gitleaks",
        ToolId::Commitlint => "commitlint",
        ToolId::Gh => "gh",
        ToolId::Cargo => "cargo",
        ToolId::Npm => "npm",
        ToolId::Node => "node",
        ToolId::Rustc => "rustc",
        ToolId::CargoAudit => "cargo-audit",
        ToolId::CargoDeny => "cargo-deny",
    }
}

fn operation_name(operation: ToolOperation) -> &'static str {
    match operation {
        ToolOperation::RepositoryRoot => "repository-root",
        ToolOperation::IgnoreCheck => "ignore-check",
        ToolOperation::ScanTracked => "scan-tracked",
        ToolOperation::LintMessage => "lint-message",
        ToolOperation::BranchRules => "branch-rules",
        ToolOperation::RepositoryRead => "repository-read",
        ToolOperation::CheckRuns => "check-runs",
        ToolOperation::QualitySuite => "quality-suite",
        ToolOperation::Version => "version",
        ToolOperation::Audit => "audit",
        ToolOperation::Deny => "deny",
    }
}

fn availability_name(availability: ToolAvailability) -> &'static str {
    match availability {
        ToolAvailability::Missing => "missing",
        ToolAvailability::Candidate => "candidate",
        ToolAvailability::Verified => "verified",
        ToolAvailability::Blocked => "blocked",
        ToolAvailability::Unknown => "unknown",
    }
}

fn version_status_name(status: ToolVersionStatus) -> &'static str {
    match status {
        ToolVersionStatus::Compatible => "compatible",
        ToolVersionStatus::Incompatible => "incompatible",
        ToolVersionStatus::Unknown => "unknown",
        ToolVersionStatus::NotApplicable => "not-applicable",
    }
}

fn capability_name(support: ToolCapabilitySupport) -> &'static str {
    match support {
        ToolCapabilitySupport::Supported => "supported",
        ToolCapabilitySupport::NeedsVerification => "needs-verification",
        ToolCapabilitySupport::Unsupported => "unsupported",
    }
}

fn platform_name(platform: HostPlatform) -> &'static str {
    match platform {
        HostPlatform::Linux => "linux",
        HostPlatform::Windows => "windows",
        HostPlatform::Other => "other",
    }
}

fn observation_name(observation: Observation) -> &'static str {
    match observation {
        Observation::Present => "present",
        Observation::Absent => "absent",
        Observation::Unknown => "unknown",
    }
}
