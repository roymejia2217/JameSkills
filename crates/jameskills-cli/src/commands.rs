use clap::{Parser, Subcommand, ValueEnum};
use jameskills_core::SkillId;
use jameskills_infra::{
    composition::RuntimeServices,
    platform::{HostPlatform, Observation},
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
            let data = json!({
                "platform": platform_name(facts.platform),
                "architecture": facts.architecture,
                "display_environment": observation_name(facts.display_environment),
                "gpu_device": observation_name(facts.gpu_device),
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
