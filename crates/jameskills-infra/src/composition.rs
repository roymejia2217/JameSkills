use crate::sqlite::SqliteStore;
use crate::{
    fs::LocalFileSystem,
    platform::{PlatformFacts, UserDirectories},
};
use chrono::{SecondsFormat, Utc};
use jameskills_core::application::GuidanceFactsProvider;
use jameskills_core::{
    AppError, AppResult, Diagnostic,
    application::{GuidanceService, LibraryService, PolicyService, policy::PolicyCheckProvider},
    domain::{
        GuidanceFactObservation, GuidanceFacts, Requirement,
        policy::{ApplicabilityFact, CheckEvidence, CheckObservation},
    },
    ports::{ClockPort, StoragePort},
};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, sync::Arc, time::Instant};

/// Process-local monotonic reference and UTC wall clock.
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockPort for SystemClock {
    fn now_utc(&self) -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    fn monotonic_ms(&self) -> u64 {
        self.origin.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
}

/// Shared process runtime providers currently available to application hosts.
#[derive(Clone)]
pub struct RuntimeServices {
    facts: PlatformFacts,
    directories: UserDirectories,
    clock: Arc<SystemClock>,
    library: Arc<LibraryService>,
    policy: Arc<PolicyService>,
    guidance: Arc<GuidanceService>,
}

impl RuntimeServices {
    pub fn facts(&self) -> &PlatformFacts {
        &self.facts
    }

    pub fn directories(&self) -> &UserDirectories {
        &self.directories
    }

    pub fn clock(&self) -> &dyn ClockPort {
        self.clock.as_ref()
    }

    pub fn library(&self) -> &LibraryService {
        self.library.as_ref()
    }

    pub fn policy(&self) -> &PolicyService {
        self.policy.as_ref()
    }

    pub fn guidance(&self) -> &GuidanceService {
        self.guidance.as_ref()
    }
}

struct UnavailablePolicyCheckProvider;

#[async_trait::async_trait]
impl PolicyCheckProvider for UnavailablePolicyCheckProvider {
    async fn observe(&self, _requirement: &Requirement) -> AppResult<CheckObservation> {
        Ok(CheckObservation::unknown())
    }
}

struct SystemGuidanceFactsProvider {
    facts: PlatformFacts,
    clock: Arc<SystemClock>,
}

#[async_trait::async_trait]
impl GuidanceFactsProvider for SystemGuidanceFactsProvider {
    async fn observe_facts(&self) -> AppResult<GuidanceFacts> {
        let operating_system = match self.facts.platform {
            crate::platform::HostPlatform::Linux => "linux",
            crate::platform::HostPlatform::Windows => "windows",
            crate::platform::HostPlatform::Other => "other",
        };
        let architecture = ApplicabilityFact::Architecture
            .accepts_value(&self.facts.architecture)
            .then_some(self.facts.architecture.as_str());
        let fingerprint_source = format!(
            "guidance-platform-v1;os={operating_system};arch={}",
            architecture.unwrap_or("unknown"),
        );
        let digest = Sha256::digest(fingerprint_source.as_bytes());
        let mut environment_fingerprint = String::with_capacity(71);
        environment_fingerprint.push_str("sha256:");
        for byte in digest {
            let _ = write!(environment_fingerprint, "{byte:02x}");
        }
        let observed_at = self.clock.now_utc();
        let expires_at = Some(self.clock.monotonic_ms().saturating_add(30_000));
        let mut observations = Vec::with_capacity(2);
        for (fact, value) in [
            (ApplicabilityFact::Os, Some(operating_system)),
            (ApplicabilityFact::Architecture, architecture),
        ] {
            if let Some(value) = value {
                let evidence = CheckEvidence::new(
                    "guidance.platform.facts",
                    &observed_at,
                    None,
                    &environment_fingerprint,
                    "Observed host platform facts.",
                    expires_at,
                )
                .map_err(AppError::Validation)?;
                observations.push(
                    GuidanceFactObservation::new(fact, value, evidence)
                        .map_err(AppError::Validation)?,
                );
            }
        }
        GuidanceFacts::new(&environment_fingerprint, observations).map_err(AppError::Validation)
    }
}

/// Build the available runtime adapters after validating caller-supplied paths.
/// Opening the persistent library creates the data directory/database as needed.
pub fn build_services(directories: UserDirectories) -> AppResult<RuntimeServices> {
    validate_directories(&directories)?;
    let storage: Arc<dyn StoragePort> = Arc::new(SqliteStore::open(
        &directories.data.join("library.sqlite3"),
    )?);
    let facts = PlatformFacts::detect();
    let clock = Arc::new(SystemClock::new());
    let policy_clock: Arc<dyn ClockPort> = clock.clone();
    let policy = Arc::new(PolicyService::new(
        Arc::new(UnavailablePolicyCheckProvider),
        policy_clock,
    ));
    let guidance_facts = Arc::new(SystemGuidanceFactsProvider {
        facts: facts.clone(),
        clock: clock.clone(),
    });
    let guidance = Arc::new(GuidanceService::new(
        policy.clone(),
        guidance_facts,
        clock.clone(),
    ));
    Ok(RuntimeServices {
        facts,
        directories,
        clock: clock.clone(),
        library: Arc::new(LibraryService::new(Arc::new(LocalFileSystem)).with_storage(storage)),
        policy,
        guidance,
    })
}

fn validate_directories(directories: &UserDirectories) -> AppResult<()> {
    let paths = [&directories.config, &directories.data, &directories.cache];
    let invalid = paths.iter().any(|path| {
        !path.is_absolute()
            || path.parent().is_none()
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::CurDir | std::path::Component::ParentDir
                )
            })
    });
    let overlapping = paths.iter().enumerate().any(|(index, path)| {
        paths
            .iter()
            .skip(index + 1)
            .any(|other| paths_overlap(path, other, cfg!(windows)))
    });

    if invalid || overlapping {
        return Err(AppError::Validation(vec![Diagnostic::error(
            "config.directories.invalid",
            "Application directories must be absolute, distinct, and non-overlapping",
        )]));
    }
    Ok(())
}

fn paths_overlap(left: &std::path::Path, right: &std::path::Path, case_insensitive: bool) -> bool {
    path_is_within(left, right, case_insensitive) || path_is_within(right, left, case_insensitive)
}

fn path_is_within(path: &std::path::Path, root: &std::path::Path, case_insensitive: bool) -> bool {
    let path_components = path.components().collect::<Vec<_>>();
    let root_components = root.components().collect::<Vec<_>>();
    path_components.len() >= root_components.len()
        && path_components
            .iter()
            .zip(root_components.iter())
            .all(|(left, right)| {
                if case_insensitive {
                    left.as_os_str().to_string_lossy().to_lowercase()
                        == right.as_os_str().to_string_lossy().to_lowercase()
                } else {
                    left == right
                }
            })
}

#[cfg(test)]
mod tests {
    use super::paths_overlap;
    use super::{UserDirectories, build_services};
    use jameskills_core::{
        application::policy::CheckRequest,
        domain::{parse_policy, policy::CheckStatus},
    };
    use std::path::Path;

    #[test]
    fn windows_directory_overlap_check_ignores_case() {
        assert!(paths_overlap(
            Path::new("C:/Users/Ada/AppData/Local/JameSkills"),
            Path::new("c:/users/ada/appdata/local/jameskills/config"),
            true,
        ));
    }

    #[test]
    fn unavailable_runtime_policy_provider_returns_unknown_not_pass() {
        let root =
            std::env::temp_dir().join(format!("jameskills policy runtime-{}", std::process::id()));
        let directories = UserDirectories {
            config: root.join("config"),
            data: root.join("data"),
            cache: root.join("cache"),
        };
        let runtime = build_services(directories).unwrap();
        let policy = parse_policy(
            include_str!("../../../tests/fixtures/valid-suite/policies/repository.toml").as_bytes(),
        )
        .unwrap();
        let report = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(
                runtime
                    .policy()
                    .check(CheckRequest::new(policy, Default::default())),
            )
            .unwrap();

        assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
        assert_eq!(report.strict_exit(), 1);
    }
}
