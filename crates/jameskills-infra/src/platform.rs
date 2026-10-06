use directories::BaseDirs;
use jameskills_core::{
    AppError, AppResult, Diagnostic,
    domain::{
        GuidanceAction, OfficialGuidanceSource, ToolId, ToolOperation,
        guidance::{ToolAvailability, ToolDetection},
    },
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken, ExecutableFingerprint,
        ProcessPermission, ProcessPort, ProcessSpec,
    },
};
use semver::VersionReq;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsString,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

const TOOL_PROFILE_SOURCE: &str = include_str!("../../../profiles/tools.toml");
const GITLEAKS_CONFIG_PLACEHOLDER: &str = "{APP_GITLEAKS_CONFIG}";
const MAX_TOOL_PROFILES: usize = 32;
const MAX_PROFILE_ARGUMENTS: usize = 8;
const MAX_PROFILE_ARGUMENT_BYTES: usize = 128;
const MAX_PROFILE_OPERATIONS: usize = 16;
const MAX_VERSION_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_PROJECT_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_PROJECT_SCRIPT_NAMES: usize = 64;
const MAX_PROJECT_SCRIPT_NAME_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostPlatform {
    Linux,
    Windows,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observation {
    Present,
    Absent,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformFacts {
    pub platform: HostPlatform,
    pub architecture: String,
    pub display_environment: Observation,
    pub gpu_device: Observation,
}

#[derive(Clone, PartialEq, Eq)]
pub struct UserDirectories {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformError {
    BaseDirectoriesUnavailable,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ToolProfile {
    tool_id: ToolId,
    executable_name: String,
    source_id: String,
    version_range: VersionReq,
    version_args: Vec<String>,
    version_prefix: String,
    operations: Vec<ToolOperation>,
    install_guides: ToolInstallGuides,
    scan_probe: Option<ToolScanSpec>,
}

impl ToolProfile {
    pub fn tool_id(&self) -> ToolId {
        self.tool_id
    }

    pub fn executable_name(&self) -> &str {
        &self.executable_name
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn version_range(&self) -> &VersionReq {
        &self.version_range
    }

    pub fn version_args(&self) -> &[String] {
        &self.version_args
    }

    pub fn version_prefix(&self) -> &str {
        &self.version_prefix
    }

    pub fn operations(&self) -> &[ToolOperation] {
        &self.operations
    }

    pub fn install_guides(&self) -> &ToolInstallGuides {
        &self.install_guides
    }

    pub fn scan_probe(&self) -> Option<&ToolScanSpec> {
        self.scan_probe.as_ref()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ToolScanSpec {
    args: Vec<String>,
    findings_exit_code: i32,
    output_limit_bytes: usize,
}

impl ToolScanSpec {
    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn clean_exit_code(&self) -> i32 {
        0
    }

    pub fn findings_exit_code(&self) -> i32 {
        self.findings_exit_code
    }

    pub fn output_schema(&self) -> &'static str {
        "gitleaks-json-array-v1"
    }

    pub fn output_limit_bytes(&self) -> usize {
        self.output_limit_bytes
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct OfficialInstallGuide {
    source_id: String,
    url: &'static str,
}

impl OfficialInstallGuide {
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn url(&self) -> &'static str {
        self.url
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ToolInstallGuides {
    windows: OfficialInstallGuide,
    linux: OfficialInstallGuide,
}

impl ToolInstallGuides {
    pub fn for_platform(&self, platform: HostPlatform) -> Option<&OfficialInstallGuide> {
        match platform {
            HostPlatform::Windows => Some(&self.windows),
            HostPlatform::Linux => Some(&self.linux),
            HostPlatform::Other => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawToolRegistry {
    schema_version: u32,
    tools: Vec<RawToolProfile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawToolProfile {
    tool_id: String,
    executable: String,
    source_id: String,
    version_range: String,
    version_args: Vec<String>,
    version_prefix: String,
    operations: Vec<String>,
    install_guides: RawToolInstallGuides,
    #[serde(default)]
    scan_probe: Option<RawToolScanSpec>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawToolScanSpec {
    args: Vec<String>,
    findings_exit_code: i32,
    output_limit_bytes: usize,
    output_schema: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawToolInstallGuides {
    windows: String,
    linux: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolCandidateKind {
    Missing,
    NativeExecutable,
    CommandShim,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ToolCandidate {
    tool_id: ToolId,
    path: Option<PathBuf>,
    kind: ToolCandidateKind,
}

#[derive(Clone, PartialEq, Eq)]
pub enum RenderedGuidanceAction {
    ManualInstruction,
    OpenOfficialUrl {
        source_id: &'static str,
        url: &'static str,
    },
    CopyApprovedCommand {
        tool_id: ToolId,
        operation: ToolOperation,
        text: &'static str,
    },
    SelectLocalPath {
        purpose: String,
    },
    AnswerChoice {
        choices: Vec<String>,
    },
    Recheck,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectStack {
    Rust,
    Node,
    RustAndNode,
    Generic,
    Unknown,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProjectManifestFacts {
    stack: ProjectStack,
    node_script_names: Vec<String>,
}

impl ProjectManifestFacts {
    pub fn stack(&self) -> ProjectStack {
        self.stack
    }

    /// Bounded script names only. Script bodies are neither returned nor run.
    pub fn node_script_names(&self) -> &[String] {
        &self.node_script_names
    }
}

enum ManifestFile {
    Missing,
    Present(Vec<u8>),
    Invalid,
}

impl ToolCandidate {
    pub fn tool_id(&self) -> ToolId {
        self.tool_id
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn kind(&self) -> ToolCandidateKind {
        self.kind
    }
}

pub fn load_tool_profiles() -> Result<Vec<ToolProfile>, Vec<Diagnostic>> {
    parse_tool_profiles(TOOL_PROFILE_SOURCE)
}

/// Resolves a validated guidance action to app-owned presentation data only.
/// Returned command text is never passed to a process port.
pub fn render_guidance_action(action: &GuidanceAction) -> RenderedGuidanceAction {
    match action {
        GuidanceAction::ManualInstruction => RenderedGuidanceAction::ManualInstruction,
        GuidanceAction::OpenOfficialUrl { source } => match source {
            OfficialGuidanceSource::GitInstall => RenderedGuidanceAction::OpenOfficialUrl {
                source_id: "git-install",
                url: "https://git-scm.com/downloads",
            },
            OfficialGuidanceSource::Gitleaks => RenderedGuidanceAction::OpenOfficialUrl {
                source_id: "gitleaks",
                url: "https://github.com/gitleaks/gitleaks",
            },
            OfficialGuidanceSource::ConventionalCommits => {
                RenderedGuidanceAction::OpenOfficialUrl {
                    source_id: "conventional-commits",
                    url: "https://www.conventionalcommits.org/en/v1.0.0/",
                }
            }
            OfficialGuidanceSource::GithubCli => RenderedGuidanceAction::OpenOfficialUrl {
                source_id: "github-cli",
                url: "https://cli.github.com/manual/gh_auth_login",
            },
            OfficialGuidanceSource::GithubRulesets => RenderedGuidanceAction::OpenOfficialUrl {
                source_id: "github-rulesets",
                url: "https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets",
            },
        },
        GuidanceAction::CopyApprovedCommand { tool_id, operation }
            if *tool_id == ToolId::Git && *operation == ToolOperation::RepositoryRoot =>
        {
            RenderedGuidanceAction::CopyApprovedCommand {
                tool_id: *tool_id,
                operation: *operation,
                text: "git rev-parse --show-toplevel",
            }
        }
        GuidanceAction::CopyApprovedCommand { .. } => RenderedGuidanceAction::Unsupported,
        GuidanceAction::SelectLocalPath { purpose } => RenderedGuidanceAction::SelectLocalPath {
            purpose: purpose.clone(),
        },
        GuidanceAction::AnswerChoice { choices } => RenderedGuidanceAction::AnswerChoice {
            choices: choices.clone(),
        },
        GuidanceAction::Recheck => RenderedGuidanceAction::Recheck,
    }
}

pub async fn detect_tools(
    search_paths: &[PathBuf],
    platform: HostPlatform,
    approved_fingerprints: &BTreeMap<ToolId, ExecutableFingerprint>,
    cwd: &ApprovedRoot,
    environment: &ApprovedEnv,
    process: &dyn ProcessPort,
    observed_at: &str,
) -> AppResult<Vec<ToolDetection>> {
    let profiles = load_tool_profiles().map_err(AppError::Validation)?;
    let candidates = find_tool_candidates(&profiles, search_paths, platform);
    let mut detections = Vec::with_capacity(profiles.len());
    for (profile, candidate) in profiles.iter().zip(&candidates) {
        let fingerprint = approved_fingerprints.get(&profile.tool_id).copied();
        detections.push(
            probe_registered_tool_version(
                profile,
                candidate,
                fingerprint,
                cwd,
                environment,
                process,
                observed_at,
            )
            .await?,
        );
    }
    Ok(detections)
}

pub fn inspect_project_manifests(root: &ApprovedRoot) -> AppResult<ProjectManifestFacts> {
    let root = std::fs::canonicalize(root.path()).map_err(|_| AppError::NotFound)?;
    if !root.is_dir() {
        return Err(AppError::NotFound);
    }
    let cargo = read_project_manifest(&root, "Cargo.toml");
    let node = read_project_manifest(&root, "package.json");
    if matches!(&cargo, ManifestFile::Invalid) || matches!(&node, ManifestFile::Invalid) {
        return Ok(unknown_project_facts());
    }
    let has_node = matches!(&node, ManifestFile::Present(_));

    let has_rust = match cargo {
        ManifestFile::Missing => false,
        ManifestFile::Present(bytes) => {
            if !valid_cargo_manifest(&bytes) {
                return Ok(unknown_project_facts());
            }
            true
        }
        ManifestFile::Invalid => unreachable!(),
    };
    let node_script_names = match node {
        ManifestFile::Missing => Vec::new(),
        ManifestFile::Present(bytes) => match parse_node_manifest(&bytes) {
            Some(script_names) => script_names,
            None => return Ok(unknown_project_facts()),
        },
        ManifestFile::Invalid => unreachable!(),
    };
    let stack = match (has_rust, has_node) {
        (true, true) => ProjectStack::RustAndNode,
        (true, false) => ProjectStack::Rust,
        (false, true) => ProjectStack::Node,
        (false, false) => ProjectStack::Generic,
    };
    Ok(ProjectManifestFacts {
        stack,
        node_script_names,
    })
}

fn read_project_manifest(root: &Path, filename: &str) -> ManifestFile {
    let path = root.join(filename);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ManifestFile::Missing;
        }
        Err(_) => return ManifestFile::Invalid,
    };
    if !metadata.file_type().is_file() || metadata.len() > MAX_PROJECT_MANIFEST_BYTES {
        return ManifestFile::Invalid;
    }
    let Ok(canonical_path) = std::fs::canonicalize(&path) else {
        return ManifestFile::Invalid;
    };
    if canonical_path != path {
        return ManifestFile::Invalid;
    }
    let Ok(mut file) = File::open(canonical_path) else {
        return ManifestFile::Invalid;
    };
    let Ok(opened_metadata) = file.metadata() else {
        return ManifestFile::Invalid;
    };
    if !opened_metadata.file_type().is_file()
        || opened_metadata.len() > MAX_PROJECT_MANIFEST_BYTES
        || opened_metadata.len() != metadata.len()
    {
        return ManifestFile::Invalid;
    }
    let mut bytes = Vec::with_capacity(opened_metadata.len() as usize);
    if file
        .by_ref()
        .take(MAX_PROJECT_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > MAX_PROJECT_MANIFEST_BYTES
    {
        return ManifestFile::Invalid;
    }
    let Ok(after_metadata) = file.metadata() else {
        return ManifestFile::Invalid;
    };
    if bytes.len() as u64 != after_metadata.len()
        || opened_metadata.modified().ok() != after_metadata.modified().ok()
    {
        return ManifestFile::Invalid;
    }
    ManifestFile::Present(bytes)
}

fn valid_cargo_manifest(bytes: &[u8]) -> bool {
    let Ok(manifest) = toml::from_slice::<toml::Value>(bytes) else {
        return false;
    };
    manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .is_some_and(|name| !name.is_empty())
        || manifest.get("workspace").is_some_and(toml::Value::is_table)
}

fn parse_node_manifest(bytes: &[u8]) -> Option<Vec<String>> {
    let manifest: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let root = manifest.as_object()?;
    let Some(scripts) = root.get("scripts") else {
        return Some(Vec::new());
    };
    let scripts = scripts.as_object()?;
    if scripts.len() > MAX_PROJECT_SCRIPT_NAMES {
        return None;
    }
    let mut names = Vec::with_capacity(scripts.len());
    for (name, value) in scripts {
        if name.is_empty()
            || name.len() > MAX_PROJECT_SCRIPT_NAME_BYTES
            || name.chars().any(char::is_control)
            || !value.is_string()
        {
            return None;
        }
        names.push(name.clone());
    }
    names.sort();
    Some(names)
}

fn unknown_project_facts() -> ProjectManifestFacts {
    ProjectManifestFacts {
        stack: ProjectStack::Unknown,
        node_script_names: Vec::new(),
    }
}

fn parse_tool_profiles(source: &str) -> Result<Vec<ToolProfile>, Vec<Diagnostic>> {
    let raw: RawToolRegistry = toml::from_str(source).map_err(|_| profile_diagnostic())?;
    if raw.schema_version != 1 || raw.tools.is_empty() || raw.tools.len() > MAX_TOOL_PROFILES {
        return Err(profile_diagnostic());
    }

    let mut registered_tools = BTreeSet::new();
    let mut profiles = Vec::with_capacity(raw.tools.len());
    for raw_profile in raw.tools {
        let tool_id = parse_tool_id(&raw_profile.tool_id).ok_or_else(profile_diagnostic)?;
        if !registered_tools.insert(tool_id)
            || !valid_executable_name(&raw_profile.executable)
            || !valid_source_id(&raw_profile.source_id)
            || raw_profile.version_prefix.len() > MAX_PROFILE_ARGUMENT_BYTES
            || raw_profile.version_prefix.chars().any(char::is_control)
            || raw_profile.version_args.is_empty()
            || raw_profile.version_args.len() > MAX_PROFILE_ARGUMENTS
            || raw_profile.version_args.iter().any(|argument| {
                argument.is_empty()
                    || argument.len() > MAX_PROFILE_ARGUMENT_BYTES
                    || argument.chars().any(char::is_control)
            })
        {
            return Err(profile_diagnostic());
        }
        let version_range =
            VersionReq::parse(&raw_profile.version_range).map_err(|_| profile_diagnostic())?;
        let windows_guide =
            official_install_guide(&raw_profile.install_guides.windows, HostPlatform::Windows)
                .ok_or_else(profile_diagnostic)?;
        let linux_guide =
            official_install_guide(&raw_profile.install_guides.linux, HostPlatform::Linux)
                .ok_or_else(profile_diagnostic)?;
        let operations: Vec<_> = raw_profile
            .operations
            .iter()
            .map(|operation| parse_tool_operation(operation).ok_or_else(profile_diagnostic))
            .collect::<Result<_, _>>()?;
        let scan_probe = parse_scan_probe(tool_id, raw_profile.scan_probe)?;
        let unique_operations: BTreeSet<_> = operations.iter().copied().collect();
        if operations.is_empty()
            || operations.len() > MAX_PROFILE_OPERATIONS
            || operations.len() != unique_operations.len()
            || !operations.contains(&ToolOperation::Version)
            || operations.contains(&ToolOperation::ScanTracked) != scan_probe.is_some()
            || (tool_id == ToolId::Gitleaks && !operations.contains(&ToolOperation::ScanTracked))
            || operations
                .iter()
                .any(|operation| !profile_operation_allowed(tool_id, *operation))
        {
            return Err(profile_diagnostic());
        }
        profiles.push(ToolProfile {
            tool_id,
            executable_name: raw_profile.executable,
            source_id: raw_profile.source_id,
            version_range,
            version_args: raw_profile.version_args,
            version_prefix: raw_profile.version_prefix,
            operations,
            install_guides: ToolInstallGuides {
                windows: windows_guide,
                linux: linux_guide,
            },
            scan_probe,
        });
    }
    Ok(profiles)
}

fn parse_scan_probe(
    tool_id: ToolId,
    raw: Option<RawToolScanSpec>,
) -> Result<Option<ToolScanSpec>, Vec<Diagnostic>> {
    match (tool_id, raw) {
        (ToolId::Gitleaks, Some(raw)) => {
            let expected_args = [
                "dir",
                "--config",
                GITLEAKS_CONFIG_PLACEHOLDER,
                "--redact",
                "--no-banner",
                "--no-color",
                "--report-format",
                "json",
                "--exit-code",
                "3",
            ];
            if raw.args.iter().map(String::as_str).ne(expected_args)
                || raw.findings_exit_code != 3
                || raw.output_schema != "gitleaks-json-array-v1"
                || raw.output_limit_bytes == 0
                || raw.output_limit_bytes > MAX_VERSION_OUTPUT_BYTES
            {
                return Err(profile_diagnostic());
            }
            Ok(Some(ToolScanSpec {
                args: raw.args,
                findings_exit_code: raw.findings_exit_code,
                output_limit_bytes: raw.output_limit_bytes,
            }))
        }
        (ToolId::Gitleaks, None) | (_, Some(_)) => Err(profile_diagnostic()),
        (_, None) => Ok(None),
    }
}

fn official_install_guide(source_id: &str, platform: HostPlatform) -> Option<OfficialInstallGuide> {
    let (expected_host, url) = match source_id {
        "git-install-windows" => ("windows", "https://git-scm.com/install/windows"),
        "git-install-linux" => ("linux", "https://git-scm.com/install/linux"),
        "node-install-windows" => ("windows", "https://nodejs.org/en/download"),
        "node-install-linux" => ("linux", "https://nodejs.org/en/download"),
        "npm-install-windows" => (
            "windows",
            "https://docs.npmjs.com/downloading-and-installing-node-js-and-npm",
        ),
        "npm-install-linux" => (
            "linux",
            "https://docs.npmjs.com/downloading-and-installing-node-js-and-npm",
        ),
        "rust-install-windows" => (
            "windows",
            "https://rust-lang.github.io/rustup/installation/windows-msvc.html",
        ),
        "rust-install-linux" => (
            "linux",
            "https://rust-lang.github.io/rustup/installation/other.html",
        ),
        "gh-install-windows" => (
            "windows",
            "https://github.com/cli/cli/blob/trunk/docs/install_windows.md",
        ),
        "gh-install-linux" => (
            "linux",
            "https://github.com/cli/cli/blob/trunk/docs/install_linux.md",
        ),
        "gitleaks-install-windows" => {
            ("windows", "https://github.com/gitleaks/gitleaks#installing")
        }
        "gitleaks-install-linux" => ("linux", "https://github.com/gitleaks/gitleaks#installing"),
        "commitlint-install-windows" => (
            "windows",
            "https://commitlint.js.org/guides/getting-started.html",
        ),
        "commitlint-install-linux" => (
            "linux",
            "https://commitlint.js.org/guides/getting-started.html",
        ),
        "cargo-audit-install-windows" => (
            "windows",
            "https://github.com/rustsec/rustsec/tree/main/cargo-audit",
        ),
        "cargo-audit-install-linux" => (
            "linux",
            "https://github.com/rustsec/rustsec/tree/main/cargo-audit",
        ),
        "cargo-deny-install-windows" => ("windows", "https://embarkstudios.github.io/cargo-deny/"),
        "cargo-deny-install-linux" => ("linux", "https://embarkstudios.github.io/cargo-deny/"),
        _ => return None,
    };
    let correct_platform = match platform {
        HostPlatform::Windows => expected_host == "windows",
        HostPlatform::Linux => expected_host == "linux",
        HostPlatform::Other => false,
    };
    if !correct_platform {
        return None;
    }
    Some(OfficialInstallGuide {
        source_id: source_id.to_owned(),
        url,
    })
}

pub fn find_tool_candidates(
    profiles: &[ToolProfile],
    search_paths: &[PathBuf],
    platform: HostPlatform,
) -> Vec<ToolCandidate> {
    profiles
        .iter()
        .map(|profile| find_tool_candidate(profile, search_paths, platform))
        .collect()
}

fn find_tool_candidate(
    profile: &ToolProfile,
    search_paths: &[PathBuf],
    platform: HostPlatform,
) -> ToolCandidate {
    for directory in search_paths.iter().filter(|path| path.is_absolute()) {
        for (filename, kind) in candidate_filenames(profile, platform) {
            let candidate_path = directory.join(filename);
            if !std::fs::metadata(&candidate_path).is_ok_and(|metadata| metadata.is_file()) {
                continue;
            }
            let Ok(path) = std::fs::canonicalize(candidate_path) else {
                continue;
            };
            return ToolCandidate {
                tool_id: profile.tool_id,
                path: Some(path),
                kind,
            };
        }
    }
    ToolCandidate {
        tool_id: profile.tool_id,
        path: None,
        kind: ToolCandidateKind::Missing,
    }
}

fn candidate_filenames(
    profile: &ToolProfile,
    platform: HostPlatform,
) -> Vec<(String, ToolCandidateKind)> {
    if platform != HostPlatform::Windows {
        return vec![(
            profile.executable_name.clone(),
            ToolCandidateKind::NativeExecutable,
        )];
    }
    let mut candidates = vec![(
        format!("{}.exe", profile.executable_name),
        ToolCandidateKind::NativeExecutable,
    )];
    if matches!(profile.tool_id, ToolId::Npm | ToolId::Commitlint) {
        candidates.push((
            format!("{}.cmd", profile.executable_name),
            ToolCandidateKind::CommandShim,
        ));
    }
    candidates
}

pub fn parse_tool_version_output(profile: &ToolProfile, output: &[u8]) -> Option<semver::Version> {
    if output.is_empty() || output.len() > MAX_VERSION_OUTPUT_BYTES || output.contains(&0) {
        return None;
    }
    let output = std::str::from_utf8(output).ok()?;
    let line = output.lines().next()?;
    let version_text = line.strip_prefix(&profile.version_prefix)?;
    let token = version_text.split_whitespace().next()?;
    parse_version_token(token)
}

pub async fn probe_registered_tool_version(
    profile: &ToolProfile,
    candidate: &ToolCandidate,
    approved_fingerprint: Option<ExecutableFingerprint>,
    cwd: &ApprovedRoot,
    environment: &ApprovedEnv,
    process: &dyn ProcessPort,
    observed_at: &str,
) -> AppResult<ToolDetection> {
    if profile.tool_id != candidate.tool_id {
        return Err(AppError::Validation(vec![Diagnostic::error(
            "tool.profile.mismatch",
            "Tool profile and candidate identifiers do not match.",
        )]));
    }

    let (availability, version, summary) = match candidate.kind {
        ToolCandidateKind::Missing => (
            ToolAvailability::Missing,
            None,
            "No registered executable candidate was found.",
        ),
        ToolCandidateKind::CommandShim => (
            ToolAvailability::Blocked,
            None,
            "The command shim is not executed by the registered version probe.",
        ),
        ToolCandidateKind::NativeExecutable => {
            let Some(path) = candidate.path.as_ref() else {
                return Err(AppError::Validation(vec![Diagnostic::error(
                    "tool.candidate.invalid",
                    "Native executable candidate has no path.",
                )]));
            };
            let Some(fingerprint) = approved_fingerprint else {
                return tool_detection(
                    profile,
                    ToolAvailability::Candidate,
                    None,
                    observed_at,
                    "Candidate requires explicit executable fingerprint approval.",
                );
            };
            let executable = ApprovedExecutable::from_absolute_path(path.clone())
                .map_err(AppError::Validation)?;
            let cwd = ApprovedRoot::from_absolute_path(cwd.path().to_path_buf())
                .map_err(AppError::Validation)?;
            let environment =
                ApprovedEnv::new(environment.entries().clone()).map_err(AppError::Validation)?;
            let args = profile
                .version_args
                .iter()
                .cloned()
                .map(OsString::from)
                .collect();
            let spec = ProcessSpec::new(
                executable,
                profile.tool_id,
                args,
                cwd,
                environment,
                Duration::from_secs(3),
                MAX_VERSION_OUTPUT_BYTES,
                ProcessPermission::ReadOnlyCheck,
                CancellationToken::new(),
            )
            .map_err(AppError::Validation)?
            .with_approved_executable_fingerprint(fingerprint);

            match process.run(spec).await {
                Ok(output) if output.exit_code() == Some(0) => (
                    ToolAvailability::Candidate,
                    parse_tool_version_output(profile, output.stdout()),
                    "Registered version probe completed; capabilities remain unverified.",
                ),
                Ok(_) | Err(AppError::ExternalTool { .. }) => (
                    ToolAvailability::Candidate,
                    None,
                    "Registered version probe produced no usable version.",
                ),
                Err(AppError::PermissionDenied { .. }) => (
                    ToolAvailability::Blocked,
                    None,
                    "Executable identity did not match its approved fingerprint.",
                ),
                Err(error) => return Err(error),
            }
        }
    };
    tool_detection(profile, availability, version, observed_at, summary)
}

fn tool_detection(
    profile: &ToolProfile,
    availability: ToolAvailability,
    version: Option<semver::Version>,
    observed_at: &str,
    summary: &'static str,
) -> AppResult<ToolDetection> {
    ToolDetection::from_probe(
        profile.tool_id,
        availability,
        version,
        &profile.version_range,
        &profile.operations,
        &profile.source_id,
        observed_at,
        summary,
    )
    .map_err(AppError::Validation)
}

fn parse_version_token(token: &str) -> Option<semver::Version> {
    let token = token.strip_prefix('v').unwrap_or(token);
    if let Ok(version) = semver::Version::parse(token) {
        return Some(version);
    }
    let (base, windows_suffix) = token.split_once(".windows.")?;
    if windows_suffix.is_empty()
        || windows_suffix
            .split('.')
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    semver::Version::parse(base).ok()
}

fn parse_tool_id(value: &str) -> Option<ToolId> {
    match value {
        "git" => Some(ToolId::Git),
        "gitleaks" => Some(ToolId::Gitleaks),
        "commitlint" => Some(ToolId::Commitlint),
        "gh" => Some(ToolId::Gh),
        "cargo" => Some(ToolId::Cargo),
        "npm" => Some(ToolId::Npm),
        "node" => Some(ToolId::Node),
        "rustc" => Some(ToolId::Rustc),
        "cargo-audit" => Some(ToolId::CargoAudit),
        "cargo-deny" => Some(ToolId::CargoDeny),
        _ => None,
    }
}

fn parse_tool_operation(value: &str) -> Option<ToolOperation> {
    match value {
        "repository-root" => Some(ToolOperation::RepositoryRoot),
        "ignore-check" => Some(ToolOperation::IgnoreCheck),
        "scan-tracked" => Some(ToolOperation::ScanTracked),
        "lint-message" => Some(ToolOperation::LintMessage),
        "branch-rules" => Some(ToolOperation::BranchRules),
        "repository-read" => Some(ToolOperation::RepositoryRead),
        "check-runs" => Some(ToolOperation::CheckRuns),
        "quality-suite" => Some(ToolOperation::QualitySuite),
        "version" => Some(ToolOperation::Version),
        "audit" => Some(ToolOperation::Audit),
        "deny" => Some(ToolOperation::Deny),
        _ => None,
    }
}

fn profile_operation_allowed(tool: ToolId, operation: ToolOperation) -> bool {
    matches!(
        (tool, operation),
        (
            ToolId::Git,
            ToolOperation::Version | ToolOperation::RepositoryRoot | ToolOperation::IgnoreCheck
        ) | (
            ToolId::Gitleaks,
            ToolOperation::Version | ToolOperation::ScanTracked
        ) | (
            ToolId::Commitlint,
            ToolOperation::Version | ToolOperation::LintMessage
        ) | (
            ToolId::Gh,
            ToolOperation::Version
                | ToolOperation::BranchRules
                | ToolOperation::RepositoryRead
                | ToolOperation::CheckRuns
        ) | (
            ToolId::Cargo | ToolId::Npm,
            ToolOperation::Version | ToolOperation::QualitySuite
        ) | (ToolId::Node | ToolId::Rustc, ToolOperation::Version)
            | (
                ToolId::CargoAudit,
                ToolOperation::Version | ToolOperation::Audit
            )
            | (
                ToolId::CargoDeny,
                ToolOperation::Version | ToolOperation::Deny
            )
    )
}

fn valid_executable_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
        && !value.ends_with('.')
        && !value.ends_with(".cmd")
        && !value.ends_with(".bat")
}

fn valid_source_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
        && value
            .split(['.', '-'])
            .all(|component| !component.is_empty())
}

fn profile_diagnostic() -> Vec<Diagnostic> {
    vec![Diagnostic::error(
        "tool.registry.invalid",
        "The application tool registry is invalid.",
    )]
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DirectoryBases {
    config: PathBuf,
    data: PathBuf,
    cache: PathBuf,
}

fn namespace_directories(platform: HostPlatform, bases: DirectoryBases) -> UserDirectories {
    let product_directory = match platform {
        HostPlatform::Windows => "JameSkills",
        HostPlatform::Linux | HostPlatform::Other => "jameskills",
    };
    match platform {
        HostPlatform::Windows => UserDirectories {
            config: bases.config.join(product_directory),
            data: bases.data.join(product_directory).join("Data"),
            cache: bases.cache.join(product_directory).join("Cache"),
        },
        HostPlatform::Linux | HostPlatform::Other => UserDirectories {
            config: bases.config.join(product_directory),
            data: bases.data.join(product_directory),
            cache: bases.cache.join(product_directory),
        },
    }
}

fn detect_from(
    platform: HostPlatform,
    architecture: &str,
    has_environment: impl Fn(&str) -> bool,
    path_exists: impl Fn(&Path) -> bool,
) -> PlatformFacts {
    let (display_environment, gpu_device) = match platform {
        HostPlatform::Linux => {
            let display = if ["DISPLAY", "WAYLAND_DISPLAY"]
                .into_iter()
                .any(&has_environment)
            {
                Observation::Present
            } else {
                Observation::Absent
            };
            let gpu = if ["/dev/dri/renderD128", "/dev/dri/card0", "/dev/nvidia0"]
                .into_iter()
                .map(Path::new)
                .any(&path_exists)
            {
                Observation::Present
            } else {
                Observation::Absent
            };
            (display, gpu)
        }
        HostPlatform::Windows | HostPlatform::Other => (Observation::Unknown, Observation::Unknown),
    };
    PlatformFacts {
        platform,
        architecture: architecture.to_owned(),
        display_environment,
        gpu_device,
    }
}

pub fn resolve_user_dirs() -> Result<UserDirectories, PlatformError> {
    let bases = BaseDirs::new().ok_or(PlatformError::BaseDirectoriesUnavailable)?;
    Ok(namespace_directories(
        current_platform(),
        DirectoryBases {
            config: bases.config_dir().to_path_buf(),
            data: bases.data_local_dir().to_path_buf(),
            cache: bases.cache_dir().to_path_buf(),
        },
    ))
}

impl PlatformFacts {
    pub fn detect() -> Self {
        detect_from(
            current_platform(),
            env::consts::ARCH,
            |name| env::var_os(name).is_some_and(|value| !value.is_empty()),
            Path::exists,
        )
    }
}

fn current_platform() -> HostPlatform {
    if cfg!(target_os = "linux") {
        HostPlatform::Linux
    } else if cfg!(target_os = "windows") {
        HostPlatform::Windows
    } else {
        HostPlatform::Other
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DirectoryBases, HostPlatform, Observation, PlatformFacts, TOOL_PROFILE_SOURCE, detect_from,
        namespace_directories, parse_tool_profiles, resolve_user_dirs,
    };
    use jameskills_core::domain::policy::{ToolId, ToolOperation};
    use std::path::PathBuf;

    #[test]
    fn linux_dirs_are_namespaced_to_jameskills() {
        let dirs = namespace_directories(
            HostPlatform::Linux,
            DirectoryBases {
                config: PathBuf::from("/home/example/.config"),
                data: PathBuf::from("/home/example/.local/share"),
                cache: PathBuf::from("/home/example/.cache"),
            },
        );

        assert_eq!(
            dirs.config,
            PathBuf::from("/home/example/.config/jameskills")
        );
        assert_eq!(
            dirs.data,
            PathBuf::from("/home/example/.local/share/jameskills")
        );
        assert_eq!(dirs.cache, PathBuf::from("/home/example/.cache/jameskills"));
    }

    #[test]
    fn windows_dirs_use_product_folder_under_known_folder_roots() {
        let dirs = namespace_directories(
            HostPlatform::Windows,
            DirectoryBases {
                config: PathBuf::from("known-config"),
                data: PathBuf::from("known-data"),
                cache: PathBuf::from("known-cache"),
            },
        );

        assert!(dirs.config.ends_with("JameSkills"));
        assert!(dirs.data.ends_with("JameSkills/Data"));
        assert!(dirs.cache.ends_with("JameSkills/Cache"));
        assert_ne!(
            dirs.config,
            PathBuf::from("/home/example/.config/JameSkills")
        );
    }

    #[test]
    fn windows_local_data_and_cache_are_separate_children_of_local_app_data() {
        let dirs = namespace_directories(
            HostPlatform::Windows,
            DirectoryBases {
                config: PathBuf::from("known-roaming"),
                data: PathBuf::from("known-local"),
                cache: PathBuf::from("known-local"),
            },
        );

        assert_eq!(dirs.data, PathBuf::from("known-local/JameSkills/Data"));
        assert_eq!(dirs.cache, PathBuf::from("known-local/JameSkills/Cache"));
    }

    #[test]
    fn linux_facts_report_only_observed_display_and_gpu_inputs() {
        let facts = detect_from(
            HostPlatform::Linux,
            "x86_64",
            |name| name == "WAYLAND_DISPLAY",
            |path| path == std::path::Path::new("/dev/dri/renderD128"),
        );

        assert_eq!(facts.architecture, "x86_64");
        assert_eq!(facts.display_environment, Observation::Present);
        assert_eq!(facts.gpu_device, Observation::Present);
    }

    #[test]
    fn windows_facts_do_not_assume_display_or_gpu() {
        let facts = detect_from(HostPlatform::Windows, "x86_64", |_| true, |_| true);

        assert_eq!(facts.display_environment, Observation::Unknown);
        assert_eq!(facts.gpu_device, Observation::Unknown);
    }

    #[test]
    fn detected_facts_use_current_architecture_and_known_linux_environment() {
        let facts = PlatformFacts::detect();

        assert_eq!(facts.architecture, std::env::consts::ARCH);
        if cfg!(target_os = "linux") {
            assert!(matches!(
                facts.display_environment,
                Observation::Present | Observation::Absent
            ));
        }
    }

    #[test]
    fn resolver_returns_app_scoped_platform_directories() {
        let dirs = resolve_user_dirs().expect("test host provides user directories");
        let application_dir = if cfg!(target_os = "windows") {
            "JameSkills"
        } else {
            "jameskills"
        };

        assert!(dirs.config.ends_with(application_dir));
        if cfg!(target_os = "windows") {
            assert!(
                dirs.data
                    .ends_with(PathBuf::from(application_dir).join("Data"))
            );
            assert!(
                dirs.cache
                    .ends_with(PathBuf::from(application_dir).join("Cache"))
            );
        } else {
            assert!(dirs.data.ends_with(application_dir));
            assert!(dirs.cache.ends_with(application_dir));
        }
    }

    #[test]
    fn tool_profile_parser_rejects_unknown_fields_and_crossed_operations() {
        let unknown_field = TOOL_PROFILE_SOURCE
            .replace("schema_version = 1", "schema_version = 1\ncommand = \"sh\"");
        assert!(parse_tool_profiles(&unknown_field).is_err());

        let crossed_operation = TOOL_PROFILE_SOURCE.replace(
            "operations = [\"version\", \"repository-root\", \"ignore-check\"]",
            "operations = [\"version\", \"repository-root\", \"audit\"]",
        );
        assert!(parse_tool_profiles(&crossed_operation).is_err());

        let github_operation_on_git = TOOL_PROFILE_SOURCE.replace(
            "operations = [\"version\", \"repository-root\", \"ignore-check\"]",
            "operations = [\"version\", \"repository-root\", \"repository-read\"]",
        );
        assert!(parse_tool_profiles(&github_operation_on_git).is_err());

        let unregistered_guide =
            TOOL_PROFILE_SOURCE.replace("git-install-windows", "https://example.invalid/install");
        assert!(parse_tool_profiles(&unregistered_guide).is_err());

        let crossed_guide = TOOL_PROFILE_SOURCE.replace(
            "windows = \"git-install-windows\"",
            "windows = \"git-install-linux\"",
        );
        assert!(parse_tool_profiles(&crossed_guide).is_err());

        let invalid_scan_exit =
            TOOL_PROFILE_SOURCE.replace("findings_exit_code = 3", "findings_exit_code = 1");
        assert!(parse_tool_profiles(&invalid_scan_exit).is_err());
    }

    #[test]
    fn github_profile_registers_read_only_repository_identity_operation() {
        let profiles = parse_tool_profiles(TOOL_PROFILE_SOURCE).unwrap();
        let gh = profiles
            .iter()
            .find(|profile| profile.tool_id() == ToolId::Gh);
        assert!(gh.is_some_and(|profile| {
            profile
                .operations()
                .contains(&ToolOperation::RepositoryRead)
        }));
    }
}
