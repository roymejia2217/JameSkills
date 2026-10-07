use clap::{Parser, Subcommand, ValueEnum};
use jameskills_core::domain::{
    ToolId, ToolOperation,
    guidance::{ToolAvailability, ToolCapabilitySupport, ToolDetection, ToolVersionStatus},
};
use jameskills_core::{
    SkillId,
    application::PublishDraft,
    ports::{LibraryCursor, LibraryItemState, LibraryQuery},
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
    List {
        #[arg(long)]
        search: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long = "capability")]
        capabilities: Vec<String>,
        #[arg(long, value_enum, default_value_t = LibraryState::Any)]
        state: LibraryState,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long, requires = "after_skill")]
        after_name: Option<String>,
        #[arg(long, requires = "after_name")]
        after_skill: Option<SkillId>,
    },
    /// Create an editable skill and its valid initial draft.
    Create {
        #[arg(long)]
        slug: String,
        #[arg(long)]
        display_name: String,
    },
    /// Publish a saved draft after optimistic head/generation checks.
    Publish {
        #[arg(long)]
        skill: SkillId,
        #[arg(long)]
        draft_generation: u64,
        #[arg(long = "expected-head")]
        expected_heads: Vec<String>,
    },
    /// Preview an import or apply a confirmed, quarantined import.
    Import {
        /// Bundle directory, .jskill archive, or standalone SKILL.md.
        #[arg(long)]
        path: PathBuf,
        /// Apply the choice confirmed by a previous preview.
        #[arg(long, requires_all = ["resolution", "confirmation_digest"])]
        apply: bool,
        /// Resolution selected from the prior preview.
        #[arg(long, value_enum, requires = "apply")]
        resolution: Option<ImportResolutionArg>,
        /// Exact digest returned for the source and selected resolution.
        #[arg(long, requires = "apply")]
        confirmation_digest: Option<String>,
        /// Reuse the UUID shown by a prior standalone SKILL.md preview.
        #[arg(long, requires = "apply")]
        skill_id: Option<String>,
    },
    /// Export a skill to a portable bundle.
    Export {
        #[arg(long)]
        skill: SkillId,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ImportResolutionArg {
    KeepExisting,
    AddConcurrentRoot,
    CreateQuarantinedDraft,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum LibraryState {
    #[default]
    Any,
    Active,
    Deleted,
    Conflicted,
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
                LibraryCommand::List { .. } => "library list",
                LibraryCommand::Create { .. } => "library create",
                LibraryCommand::Publish { .. } => "library publish",
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
        CliCommand::Library { command } => {
            let Some(runtime) = runtime else {
                let name = match &command {
                    LibraryCommand::List { .. } => "library list",
                    LibraryCommand::Create { .. } => "library create",
                    LibraryCommand::Publish { .. } => "library publish",
                    LibraryCommand::Import { .. } => "library import",
                    LibraryCommand::Export { .. } => "library export",
                };
                return super::output::CliResponse::unsupported(name);
            };
            dispatch_library(runtime, command)
        }
        command => super::output::CliResponse::unsupported(command.command_name()),
    }
}

fn dispatch_library(
    runtime: &RuntimeServices,
    command: LibraryCommand,
) -> super::output::CliResponse {
    use super::output::{CliResponse, response_for_app_error};
    use jameskills_core::domain::CreateSkill;

    let command_name = match &command {
        LibraryCommand::List { .. } => "library list",
        LibraryCommand::Create { .. } => "library create",
        LibraryCommand::Publish { .. } => "library publish",
        LibraryCommand::Import { .. } => "library import",
        LibraryCommand::Export { .. } => return CliResponse::unsupported("library export"),
    };
    if let LibraryCommand::Import {
        path,
        apply,
        resolution,
        confirmation_digest,
        skill_id,
    } = &command
    {
        return dispatch_library_import(
            runtime,
            path,
            *apply,
            *resolution,
            confirmation_digest.as_deref(),
            skill_id.as_deref(),
        );
    }
    let executor = match tokio::runtime::Builder::new_current_thread().build() {
        Ok(executor) => executor,
        Err(_) => {
            return CliResponse::error(
                command_name,
                "runtime.unavailable",
                "The library operation runtime is unavailable.",
                3,
            );
        }
    };

    let result = match command {
        LibraryCommand::List {
            search,
            tags,
            capabilities,
            state,
            limit,
            after_name,
            after_skill,
        } => {
            let after = match (after_name, after_skill) {
                (Some(name), Some(skill_id)) => match LibraryCursor::new(&name, skill_id) {
                    Ok(cursor) => Some(cursor),
                    Err(error) => return response_for_app_error(command_name, &error),
                },
                (None, None) => None,
                _ => {
                    return CliResponse::error(
                        command_name,
                        "library.cursor.incomplete",
                        "Both cursor name and skill ID are required.",
                        2,
                    );
                }
            };
            let state = match state {
                LibraryState::Any => LibraryItemState::Any,
                LibraryState::Active => LibraryItemState::Active,
                LibraryState::Deleted => LibraryItemState::Deleted,
                LibraryState::Conflicted => LibraryItemState::Conflicted,
            };
            let query =
                match LibraryQuery::new(search.as_deref(), tags, capabilities, state, after, limit)
                {
                    Ok(query) => query,
                    Err(error) => return response_for_app_error(command_name, &error),
                };
            executor
                .block_on(runtime.library().list_skills(query))
                .map(|page| {
                    let items = page
                        .items()
                        .iter()
                        .map(|item| {
                            json!({
                                "skill_id": item.skill_id().as_uuid().to_string(),
                                "slug": item.slug(),
                                "display_name": item.display_name(),
                                "tags": item.tags(),
                                "capabilities": item.capabilities(),
                                "conflicted": item.conflicted(),
                                "deleted": item.deleted(),
                                "heads": item.heads().iter().map(|head| json!({
                                    "revision_id": head.revision_id().as_str(),
                                    "semantic_version": head.semantic_version(),
                                    "deleted": head.deleted(),
                                })).collect::<Vec<_>>(),
                            })
                        })
                        .collect::<Vec<_>>();
                    let next = page.next().map(|cursor| {
                        json!({
                            "display_name": cursor.normalized_display_name(),
                            "skill_id": cursor.skill_id().as_uuid().to_string(),
                        })
                    });
                    json!({ "items": items, "next": next })
                })
        }
        LibraryCommand::Create { slug, display_name } => {
            let create = match CreateSkill::new(slug, display_name) {
                Ok(create) => create,
                Err(error) => return response_for_app_error(command_name, &error),
            };
            let slug = create.slug().to_owned();
            executor
                .block_on(runtime.library().create_skill(create))
                .map(|draft| {
                    json!({
                        "skill_id": draft.skill_id().as_uuid().to_string(),
                        "slug": slug,
                        "generation": draft.generation(),
                        "files": draft.files().len(),
                        "published": false,
                    })
                })
        }
        LibraryCommand::Publish {
            skill,
            draft_generation,
            expected_heads,
        } => {
            let expected_heads = match expected_heads
                .iter()
                .map(|value| jameskills_core::domain::RevisionId::parse_hex(value))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(heads) => heads,
                Err(_) => {
                    return CliResponse::error(
                        command_name,
                        "library.expected_head.invalid",
                        "Expected head must be a lowercase SHA-256 revision ID.",
                        2,
                    );
                }
            };
            let request = match PublishDraft::new(skill, draft_generation, expected_heads) {
                Ok(request) => request,
                Err(error) => return response_for_app_error(command_name, &error),
            };
            executor
                .block_on(runtime.library().publish(request))
                .map(|result| {
                    json!({
                        "skill_id": result.revision().skill_id().as_uuid().to_string(),
                        "revision_id": result.revision().id().as_str(),
                        "content_hash": result.revision().bundle_hash().as_str(),
                        "semantic_version": result.revision().semantic_version(),
                        "heads": result.new_heads().iter().map(|head| head.as_str()).collect::<Vec<_>>(),
                        "no_op": result.is_no_op(),
                    })
                })
        }
        LibraryCommand::Import { .. } | LibraryCommand::Export { .. } => unreachable!(),
    };
    match result {
        Ok(data) => CliResponse::success(command_name, data),
        Err(error) => response_for_app_error(command_name, &error),
    }
}

fn dispatch_library_import(
    runtime: &RuntimeServices,
    path: &std::path::Path,
    apply: bool,
    resolution_arg: Option<ImportResolutionArg>,
    confirmed_digest: Option<&str>,
    requested_skill_id: Option<&str>,
) -> super::output::CliResponse {
    use super::output::{CliResponse, response_for_app_error};
    use jameskills_core::domain::{
        ImportClassification, ImportResolution, ImportResult, ImportSourceKind, SkillId,
    };

    let command = "library import";
    let resolution = resolution_arg.map(|resolution| match resolution {
        ImportResolutionArg::KeepExisting => ImportResolution::KeepExisting,
        ImportResolutionArg::AddConcurrentRoot => ImportResolution::AddConcurrentRoot,
        ImportResolutionArg::CreateQuarantinedDraft => ImportResolution::CreateQuarantinedDraft,
    });
    if apply && (resolution.is_none() || confirmed_digest.is_none()) {
        return CliResponse::error(
            command,
            "library.import.confirmation.required",
            "Apply requires an explicit resolution and the preview confirmation digest.",
            2,
        );
    }
    let requested_skill_id = match requested_skill_id {
        Some(value) => match SkillId::parse(value) {
            Ok(skill_id) => Some(skill_id),
            Err(_) => {
                return CliResponse::error(
                    command,
                    "library.import.skill_id.invalid",
                    "The supplied skill ID is invalid.",
                    2,
                );
            }
        },
        None => None,
    };
    let source_kind = import_source_kind(path);
    let executor = match tokio::runtime::Builder::new_current_thread().build() {
        Ok(executor) => executor,
        Err(_) => {
            return CliResponse::error(
                command,
                "runtime.unavailable",
                "The library operation runtime is unavailable.",
                3,
            );
        }
    };
    let preview = match executor.block_on(runtime.library().preview_import_with_skill_id(
        path,
        source_kind,
        requested_skill_id,
    )) {
        Ok(preview) => preview,
        Err(error) => return response_for_app_error(command, &error),
    };
    if !apply {
        let confirmations = import_resolutions(&preview)
            .into_iter()
            .filter_map(|resolution| {
                preview.confirmation_digest(resolution).ok().map(|digest| {
                    json!({
                        "resolution": import_resolution_name(resolution),
                        "digest": digest.as_str(),
                    })
                })
            })
            .collect::<Vec<_>>();
        let classification = match preview.classification() {
            ImportClassification::NewSkill => json!({ "kind": "new-skill" }),
            ImportClassification::Identical { revision_id } => json!({
                "kind": "identical",
                "revision_id": revision_id.as_str(),
            }),
            ImportClassification::Conflict { current_heads } => json!({
                "kind": "conflict",
                "current_heads": current_heads.iter().map(|head| head.as_str()).collect::<Vec<_>>(),
            }),
        };
        return CliResponse::success(
            command,
            json!({
                "phase": "preview",
                "applied": false,
                "skill_id": preview.skill_id().as_uuid().to_string(),
                "slug": preview.slug(),
                "display_name": preview.display_name(),
                "version": preview.semantic_version(),
                "content_hash": preview.content_hash().as_str(),
                "source_kind": import_source_kind_name(preview.source_kind()),
                "classification": classification,
                "current_heads": preview.current_heads().iter().map(|head| head.as_str()).collect::<Vec<_>>(),
                "files": preview.files().iter().map(|(path, bytes)| json!({
                    "path": path.as_str(),
                    "size_bytes": bytes.len(),
                })).collect::<Vec<_>>(),
                "scan_status": import_scan_status_name(preview.scan_status()),
                "trust_state": "quarantined",
                "confirmations": confirmations,
                "plain_skill_apply_requires_skill_id": preview.source_kind() == ImportSourceKind::PlainSkill,
            }),
        );
    }

    if preview.source_kind() == ImportSourceKind::PlainSkill && requested_skill_id.is_none() {
        return CliResponse::error(
            command,
            "library.import.skill_id.required",
            "Plain-skill apply requires the skill ID shown by the prior preview.",
            2,
        );
    }
    let resolution = resolution.unwrap_or(ImportResolution::KeepExisting);
    let expected = match preview.confirmation_digest(resolution) {
        Ok(digest) => digest,
        Err(error) => return response_for_app_error(command, &error),
    };
    if Some(expected.as_str()) != confirmed_digest {
        return CliResponse::error(
            command,
            "library.import.confirmation.stale",
            "The import preview changed or the confirmation digest does not match.",
            4,
        );
    }
    match executor.block_on(runtime.library().apply_import(preview, resolution)) {
        Ok(ImportResult::Imported {
            revision,
            heads,
            trust_state,
        }) => CliResponse::success(
            command,
            json!({
                "phase": "applied",
                "applied": true,
                "skill_id": revision.skill_id().as_uuid().to_string(),
                "revision_id": revision.id().as_str(),
                "heads": heads.iter().map(|head| head.as_str()).collect::<Vec<_>>(),
                "trust_state": trust_state_name(trust_state),
                "published": false,
            }),
        ),
        Ok(ImportResult::DraftCreated {
            skill_id,
            generation,
            trust_state,
        }) => CliResponse::success(
            command,
            json!({
                "phase": "applied",
                "applied": true,
                "skill_id": skill_id.as_uuid().to_string(),
                "draft_generation": generation,
                "trust_state": trust_state_name(trust_state),
                "published": false,
            }),
        ),
        Ok(ImportResult::Duplicate {
            revision_id,
            heads,
            trust_state,
        }) => CliResponse::success(
            command,
            json!({
                "phase": "applied",
                "applied": true,
                "duplicate": true,
                "revision_id": revision_id.as_str(),
                "heads": heads.iter().map(|head| head.as_str()).collect::<Vec<_>>(),
                "trust_state": trust_state_name(trust_state),
                "published": false,
            }),
        ),
        Ok(ImportResult::KeptExisting { heads }) => CliResponse::success(
            command,
            json!({
                "phase": "applied",
                "applied": true,
                "kept_existing": true,
                "heads": heads.iter().map(|head| head.as_str()).collect::<Vec<_>>(),
                "published": false,
            }),
        ),
        Err(error) => response_for_app_error(command, &error),
    }
}

fn import_source_kind(path: &std::path::Path) -> jameskills_core::domain::ImportSourceKind {
    use jameskills_core::domain::ImportSourceKind;
    if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
        ImportSourceKind::PlainSkill
    } else if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jskill"))
    {
        ImportSourceKind::Archive
    } else {
        ImportSourceKind::Directory
    }
}

fn import_resolutions(
    preview: &jameskills_core::domain::ImportPreview,
) -> Vec<jameskills_core::domain::ImportResolution> {
    use jameskills_core::domain::{ImportClassification, ImportResolution, ImportSourceKind};
    match preview.classification() {
        ImportClassification::NewSkill if preview.source_kind() == ImportSourceKind::PlainSkill => {
            vec![ImportResolution::CreateQuarantinedDraft]
        }
        ImportClassification::NewSkill => vec![ImportResolution::AddConcurrentRoot],
        ImportClassification::Identical { .. } => vec![ImportResolution::KeepExisting],
        ImportClassification::Conflict { .. } => vec![
            ImportResolution::KeepExisting,
            ImportResolution::AddConcurrentRoot,
        ],
    }
}

fn import_resolution_name(resolution: jameskills_core::domain::ImportResolution) -> &'static str {
    use jameskills_core::domain::ImportResolution;
    match resolution {
        ImportResolution::KeepExisting => "keep-existing",
        ImportResolution::AddConcurrentRoot => "add-concurrent-root",
        ImportResolution::CreateQuarantinedDraft => "create-quarantined-draft",
    }
}

fn import_source_kind_name(source: jameskills_core::domain::ImportSourceKind) -> &'static str {
    use jameskills_core::domain::ImportSourceKind;
    match source {
        ImportSourceKind::Directory => "directory",
        ImportSourceKind::Archive => "archive",
        ImportSourceKind::PlainSkill => "plain-skill",
    }
}

fn import_scan_status_name(status: jameskills_core::domain::ImportScanStatus) -> &'static str {
    use jameskills_core::domain::ImportScanStatus;
    match status {
        ImportScanStatus::Unavailable => "unavailable",
        ImportScanStatus::NoFindings => "no-findings",
        ImportScanStatus::Findings => "findings",
        ImportScanStatus::Unknown => "unknown",
        ImportScanStatus::Blocked => "blocked",
    }
}

fn trust_state_name(status: jameskills_core::domain::TrustState) -> &'static str {
    match status {
        jameskills_core::domain::TrustState::Quarantined => "quarantined",
        jameskills_core::domain::TrustState::Reviewed => "reviewed",
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

#[cfg(test)]
mod library_command_tests {
    use super::{Cli, CliCommand, ImportResolutionArg, LibraryCommand, LibraryState, dispatch_cli};
    use jameskills_core::SkillId;
    use jameskills_infra::{composition::build_services, platform::UserDirectories};
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    static CASE_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct RuntimeCase {
        root: std::path::PathBuf,
        services: Option<jameskills_infra::composition::RuntimeServices>,
    }

    impl Drop for RuntimeCase {
        fn drop(&mut self) {
            self.services.take();
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn runtime_case() -> RuntimeCase {
        let number = CASE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "jameskills-cli-library-{}-{number}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let services = build_services(UserDirectories {
            config: root.join("config"),
            data: root.join("data"),
            cache: root.join("cache"),
        })
        .unwrap();
        RuntimeCase {
            root,
            services: Some(services),
        }
    }

    fn write_import_bundle(root: &std::path::Path) -> std::path::PathBuf {
        let source = root.join("import-source");
        std::fs::create_dir_all(source.join("policies")).unwrap();
        std::fs::create_dir_all(source.join("guidance")).unwrap();
        for (name, bytes) in [
            (
                "SKILL.md",
                include_bytes!("../../../docs/examples/repository-foundation/SKILL.md").as_slice(),
            ),
            (
                "jameskills.toml",
                include_bytes!("../../../docs/examples/repository-foundation/jameskills.toml")
                    .as_slice(),
            ),
            (
                "policies/repository.toml",
                include_bytes!(
                    "../../../docs/examples/repository-foundation/policies/repository.toml"
                )
                .as_slice(),
            ),
            (
                "guidance/repository.toml",
                include_bytes!(
                    "../../../docs/examples/repository-foundation/guidance/repository.toml"
                )
                .as_slice(),
            ),
        ] {
            std::fs::write(source.join(name), bytes).unwrap();
        }
        source
    }

    fn import_command(
        path: std::path::PathBuf,
        apply: bool,
        resolution: Option<ImportResolutionArg>,
        confirmation_digest: Option<String>,
        skill_id: Option<String>,
    ) -> LibraryCommand {
        LibraryCommand::Import {
            path,
            apply,
            resolution,
            confirmation_digest,
            skill_id,
        }
    }

    fn dispatch_library(
        command: LibraryCommand,
        services: Option<&jameskills_infra::composition::RuntimeServices>,
    ) -> super::super::output::CliResponse {
        dispatch_cli(
            Cli {
                json: true,
                command: Some(CliCommand::Library { command }),
            },
            services,
        )
    }

    #[test]
    fn library_create_list_and_publish_use_the_shared_runtime_services() {
        let case = runtime_case();
        let created = dispatch_library(
            LibraryCommand::Create {
                slug: "cli-library".to_owned(),
                display_name: "CLI library".to_owned(),
            },
            case.services.as_ref(),
        );
        assert_eq!(created.exit_code, 0);
        let skill_id =
            SkillId::parse(created.data.as_ref().unwrap()["skill_id"].as_str().unwrap()).unwrap();
        assert_eq!(created.data.as_ref().unwrap()["published"], false);

        let published = dispatch_library(
            LibraryCommand::Publish {
                skill: skill_id,
                draft_generation: 1,
                expected_heads: vec![],
            },
            case.services.as_ref(),
        );
        assert_eq!(published.exit_code, 0);
        assert_eq!(
            published.data.as_ref().unwrap()["semantic_version"],
            "0.1.0"
        );

        let listed = dispatch_library(
            LibraryCommand::List {
                search: Some("CLI".to_owned()),
                tags: vec![],
                capabilities: vec![],
                state: LibraryState::Active,
                limit: 10,
                after_name: None,
                after_skill: None,
            },
            case.services.as_ref(),
        );
        assert_eq!(listed.exit_code, 0);
        assert_eq!(
            listed.data.as_ref().unwrap()["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            listed.data.as_ref().unwrap()["items"][0]["skill_id"],
            skill_id.as_uuid().to_string()
        );
    }

    #[test]
    fn library_commands_without_a_runtime_remain_unsupported() {
        let response = dispatch_library(
            LibraryCommand::List {
                search: None,
                tags: vec![],
                capabilities: vec![],
                state: LibraryState::Any,
                limit: 50,
                after_name: None,
                after_skill: None,
            },
            None,
        );
        assert_eq!(response.exit_code, 3);
        assert_eq!(response.error.unwrap().code, "capability.unsupported");
    }

    #[test]
    fn library_list_exposes_bounded_search_state_and_cursor_flags() {
        let cli = Cli::parse([
            "jameskills",
            "library",
            "list",
            "--search",
            "agent",
            "--state",
            "conflicted",
            "--limit",
            "12",
            "--after-name",
            "Alpha",
            "--after-skill",
            "f9c0199f-c4ce-4b04-85dd-ae12a7db292b",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(CliCommand::Library {
                command: LibraryCommand::List {
                    state: LibraryState::Conflicted,
                    limit: 12,
                    ..
                }
            })
        ));
    }

    #[test]
    fn library_import_preview_requires_matching_confirmation_and_reports_quarantine() {
        let case = runtime_case();
        let source = write_import_bundle(&case.root);
        let preview = dispatch_library(
            import_command(source.clone(), false, None, None, None),
            Some(case.services.as_ref().unwrap()),
        );
        assert_eq!(preview.exit_code, 0);
        let data = preview.data.unwrap();
        assert_eq!(data["phase"], "preview");
        assert_eq!(data["applied"], false);
        assert_eq!(data["scan_status"], "unavailable");
        assert_eq!(data["trust_state"], "quarantined");
        assert_eq!(data["files"].as_array().unwrap().len(), 4);
        let digest = data["confirmations"][0]["digest"]
            .as_str()
            .unwrap()
            .to_owned();

        std::fs::OpenOptions::new()
            .append(true)
            .open(source.join("SKILL.md"))
            .unwrap()
            .write_all(b"\nChanged after preview.\n")
            .unwrap();
        let stale = dispatch_library(
            import_command(
                source.clone(),
                true,
                Some(ImportResolutionArg::AddConcurrentRoot),
                Some(digest),
                None,
            ),
            Some(case.services.as_ref().unwrap()),
        );
        assert_eq!(stale.exit_code, 4);
        assert_eq!(
            stale.error.unwrap().code,
            "library.import.confirmation.stale"
        );

        let current_preview = dispatch_library(
            import_command(source.clone(), false, None, None, None),
            Some(case.services.as_ref().unwrap()),
        );
        let data = current_preview.data.unwrap();
        let current_digest = data["confirmations"][0]["digest"]
            .as_str()
            .unwrap()
            .to_owned();
        let applied = dispatch_library(
            import_command(
                source,
                true,
                Some(ImportResolutionArg::AddConcurrentRoot),
                Some(current_digest),
                None,
            ),
            Some(case.services.as_ref().unwrap()),
        );
        assert_eq!(applied.exit_code, 0);
        assert_eq!(applied.data.as_ref().unwrap()["phase"], "applied");
        assert_eq!(applied.data.as_ref().unwrap()["trust_state"], "quarantined");
        assert_eq!(applied.data.as_ref().unwrap()["published"], false);
    }

    #[test]
    fn plain_skill_cli_import_reuses_preview_id_and_only_creates_quarantined_draft() {
        let case = runtime_case();
        let source = case.root.join("SKILL.md");
        std::fs::write(
            &source,
            b"---\nname: cli-instructions\ndescription: Imported guidance.\n---\n# Inert text\n",
        )
        .unwrap();
        let preview = dispatch_library(
            import_command(source.clone(), false, None, None, None),
            Some(case.services.as_ref().unwrap()),
        );
        assert_eq!(preview.exit_code, 0);
        let data = preview.data.unwrap();
        assert_eq!(data["source_kind"], "plain-skill");
        assert_eq!(data["trust_state"], "quarantined");
        let skill_id = data["skill_id"].as_str().unwrap().to_owned();
        let digest = data["confirmations"][0]["digest"]
            .as_str()
            .unwrap()
            .to_owned();
        let applied = dispatch_library(
            import_command(
                source,
                true,
                Some(ImportResolutionArg::CreateQuarantinedDraft),
                Some(digest),
                Some(skill_id.clone()),
            ),
            Some(case.services.as_ref().unwrap()),
        );
        assert_eq!(applied.exit_code, 0);
        let applied = applied.data.unwrap();
        assert_eq!(applied["skill_id"], skill_id);
        assert_eq!(applied["trust_state"], "quarantined");
        assert_eq!(applied["published"], false);
        assert_eq!(applied["draft_generation"], 1);
    }

    #[test]
    fn library_import_apply_flags_require_resolution_and_digest() {
        let preview = Cli::parse(["jameskills", "library", "import", "--path", "bundle"]).unwrap();
        assert!(matches!(
            preview.command,
            Some(CliCommand::Library {
                command: LibraryCommand::Import { apply: false, .. }
            })
        ));
        assert!(
            Cli::parse([
                "jameskills",
                "library",
                "import",
                "--path",
                "bundle",
                "--apply",
            ])
            .is_err()
        );
    }
}
