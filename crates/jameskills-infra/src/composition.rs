use crate::{
    fs::LocalFileSystem,
    platform::{PlatformFacts, UserDirectories},
};
use chrono::{SecondsFormat, Utc};
use jameskills_core::{
    AppError, AppResult, Diagnostic, application::LibraryService, ports::ClockPort,
};
use std::{sync::Arc, time::Instant};

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
}

/// Build the available runtime adapters after validating caller-supplied paths.
/// This function does not create directories or initialize unimplemented services.
pub fn build_services(directories: UserDirectories) -> AppResult<RuntimeServices> {
    validate_directories(&directories)?;
    Ok(RuntimeServices {
        facts: PlatformFacts::detect(),
        directories,
        clock: Arc::new(SystemClock::new()),
        library: Arc::new(LibraryService::new(Arc::new(LocalFileSystem))),
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
    use std::path::Path;

    #[test]
    fn windows_directory_overlap_check_ignores_case() {
        assert!(paths_overlap(
            Path::new("C:/Users/Ada/AppData/Local/JameSkills"),
            Path::new("c:/users/ada/appdata/local/jameskills/config"),
            true,
        ));
    }
}
