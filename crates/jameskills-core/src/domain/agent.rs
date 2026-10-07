use crate::{Diagnostic, domain::Scope};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_SOURCE_ID_BYTES: usize = 64;
const MAX_FIXTURE_ID_BYTES: usize = 64;
const MAX_OBSERVED_AT_BYTES: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AgentId {
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "opencode")]
    OpenCode,
    #[serde(rename = "pi")]
    Pi,
    #[serde(rename = "antigravity")]
    Antigravity,
    #[serde(rename = "grok")]
    Grok,
}

impl AgentId {
    pub const ALL: [Self; 5] = [
        Self::Codex,
        Self::OpenCode,
        Self::Pi,
        Self::Antigravity,
        Self::Grok,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
            Self::Pi => "pi",
            Self::Antigravity => "antigravity",
            Self::Grok => "grok",
        }
    }

    pub const fn executable_name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
            Self::Pi => "pi",
            Self::Antigravity => "agy",
            Self::Grok => "grok",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|agent| agent.as_str() == value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentCapabilityId {
    UserInstall,
    ProjectInstall,
    DiscoveryVerification,
    VendorPluginInstall,
    OpenAiMetadata,
}

impl AgentCapabilityId {
    pub const ALL: [Self; 5] = [
        Self::UserInstall,
        Self::ProjectInstall,
        Self::DiscoveryVerification,
        Self::VendorPluginInstall,
        Self::OpenAiMetadata,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilitySupport {
    Supported,
    NeedsVerification,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityEvidence {
    source_id: String,
    observed_at: String,
    tested_version: Option<Version>,
    fixture_id: Option<String>,
}

impl CapabilityEvidence {
    pub fn new(
        source_id: impl Into<String>,
        observed_at: impl Into<String>,
        tested_version: Option<Version>,
        fixture_id: Option<&str>,
    ) -> Result<Self, Vec<Diagnostic>> {
        let source_id = source_id.into();
        let observed_at = observed_at.into();
        let fixture_id = fixture_id.map(str::to_owned);
        if !valid_registry_id(&source_id, MAX_SOURCE_ID_BYTES)
            || !valid_rfc3339_utc_or_offset(&observed_at)
            || fixture_id
                .as_deref()
                .is_some_and(|id| !valid_registry_id(id, MAX_FIXTURE_ID_BYTES))
        {
            return Err(agent_diagnostic(
                "agent.capability.evidence.invalid",
                "Capability evidence metadata is invalid.",
            ));
        }
        Ok(Self {
            source_id,
            observed_at,
            tested_version,
            fixture_id,
        })
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }

    pub fn tested_version(&self) -> Option<&Version> {
        self.tested_version.as_ref()
    }

    pub fn fixture_id(&self) -> Option<&str> {
        self.fixture_id.as_deref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentCapabilityAssessment {
    support: CapabilitySupport,
    evidence: Option<CapabilityEvidence>,
}

impl AgentCapabilityAssessment {
    pub fn new(
        support: CapabilitySupport,
        evidence: Option<CapabilityEvidence>,
    ) -> Result<Self, Vec<Diagnostic>> {
        if evidence.is_none()
            || (support == CapabilitySupport::Supported
                && evidence.as_ref().is_none_or(|evidence| {
                    evidence.tested_version.is_none() || evidence.fixture_id.is_none()
                }))
        {
            return Err(agent_diagnostic(
                "agent.capability.support.evidence.required",
                "Capabilities require source/date evidence; Supported also requires tested version and fixture evidence.",
            ));
        }
        Ok(Self { support, evidence })
    }

    pub fn support(&self) -> CapabilitySupport {
        self.support
    }

    pub fn evidence(&self) -> Option<&CapabilityEvidence> {
        self.evidence.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentCapabilities(BTreeMap<AgentCapabilityId, AgentCapabilityAssessment>);

impl AgentCapabilities {
    pub fn new(
        entries: Vec<(AgentCapabilityId, AgentCapabilityAssessment)>,
    ) -> Result<Self, Vec<Diagnostic>> {
        let mut capabilities = BTreeMap::new();
        for (id, assessment) in entries {
            if capabilities.insert(id, assessment).is_some() {
                return Err(agent_diagnostic(
                    "agent.capability.duplicate",
                    "Agent capability IDs must be unique.",
                ));
            }
        }
        if AgentCapabilityId::ALL
            .iter()
            .any(|id| !capabilities.contains_key(id))
            || capabilities.len() != AgentCapabilityId::ALL.len()
        {
            return Err(agent_diagnostic(
                "agent.capability.incomplete",
                "Every registered agent capability must have an explicit status.",
            ));
        }
        Ok(Self(capabilities))
    }

    pub fn get(&self, id: AgentCapabilityId) -> CapabilitySupport {
        self.0[&id].support()
    }

    pub fn assessment(&self, id: AgentCapabilityId) -> &AgentCapabilityAssessment {
        &self.0[&id]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentInstallMode {
    FileCopy,
    VendorPlugin,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentProfile {
    id: AgentId,
    install_mode: AgentInstallMode,
    capabilities: AgentCapabilities,
}

impl AgentProfile {
    pub fn new(
        id: AgentId,
        install_mode: AgentInstallMode,
        capabilities: AgentCapabilities,
    ) -> Result<Self, Vec<Diagnostic>> {
        let mechanism_matches_id = match id {
            AgentId::Antigravity => install_mode == AgentInstallMode::VendorPlugin,
            AgentId::Codex | AgentId::OpenCode | AgentId::Pi | AgentId::Grok => {
                install_mode == AgentInstallMode::FileCopy
            }
        };
        let user = capabilities.get(AgentCapabilityId::UserInstall);
        let project = capabilities.get(AgentCapabilityId::ProjectInstall);
        let plugin = capabilities.get(AgentCapabilityId::VendorPluginInstall);
        let valid_mode = match install_mode {
            AgentInstallMode::FileCopy => plugin == CapabilitySupport::Unsupported,
            AgentInstallMode::VendorPlugin => {
                user != CapabilitySupport::Unsupported
                    && project == CapabilitySupport::Unsupported
                    && plugin != CapabilitySupport::Unsupported
            }
        };
        if !mechanism_matches_id || user == CapabilitySupport::Unsupported || !valid_mode {
            return Err(agent_diagnostic(
                "agent.profile.capability.inconsistent",
                "Agent install mode conflicts with its independently declared scope capabilities.",
            ));
        }
        Ok(Self {
            id,
            install_mode,
            capabilities,
        })
    }

    pub fn id(&self) -> AgentId {
        self.id
    }

    pub fn cli_name(&self) -> &'static str {
        self.id.executable_name()
    }

    pub fn install_mode(&self) -> AgentInstallMode {
        self.install_mode
    }

    pub fn capabilities(&self) -> &AgentCapabilities {
        &self.capabilities
    }

    pub fn supports_scope(&self, scope: Scope) -> CapabilitySupport {
        let capability = match scope {
            Scope::User => AgentCapabilityId::UserInstall,
            Scope::Project => AgentCapabilityId::ProjectInstall,
        };
        self.capabilities.get(capability)
    }
}

fn valid_registry_id(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

fn valid_rfc3339_utc_or_offset(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.len() > MAX_OBSERVED_AT_BYTES
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
        || !bytes[11..13].iter().all(u8::is_ascii_digit)
        || !bytes[14..16].iter().all(u8::is_ascii_digit)
        || !bytes[17..19].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let Some(year) = parse_digits(&bytes[0..4]) else {
        return false;
    };
    let Some(month) = parse_digits(&bytes[5..7]) else {
        return false;
    };
    let Some(day) = parse_digits(&bytes[8..10]) else {
        return false;
    };
    let Some(hour) = parse_digits(&bytes[11..13]) else {
        return false;
    };
    let Some(minute) = parse_digits(&bytes[14..16]) else {
        return false;
    };
    let Some(second) = parse_digits(&bytes[17..19]) else {
        return false;
    };
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return false,
    };
    if day == 0 || day > days_in_month || hour > 23 || minute > 59 || second > 60 {
        return false;
    }

    let mut zone_start = 19;
    if bytes.get(zone_start) == Some(&b'.') {
        zone_start += 1;
        let fraction_start = zone_start;
        while bytes.get(zone_start).is_some_and(u8::is_ascii_digit) {
            zone_start += 1;
        }
        if zone_start == fraction_start {
            return false;
        }
    }
    match bytes.get(zone_start..) {
        Some([b'Z']) => true,
        Some([b'+' | b'-', h1, h2, b':', m1, m2]) => {
            let Some(hours) = parse_digits(&[*h1, *h2]) else {
                return false;
            };
            let Some(minutes) = parse_digits(&[*m1, *m2]) else {
                return false;
            };
            hours <= 23 && minutes <= 59
        }
        _ => false,
    }
}

fn parse_digits(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0_u32, |value, byte| {
        byte.is_ascii_digit()
            .then(|| value * 10 + u32::from(*byte - b'0'))
    })
}

fn agent_diagnostic(code: &'static str, message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error(code, message)]
}
