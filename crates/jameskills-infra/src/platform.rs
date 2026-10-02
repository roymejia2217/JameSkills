use directories::BaseDirs;
use std::{
    env,
    path::{Path, PathBuf},
};

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
        DirectoryBases, HostPlatform, Observation, PlatformFacts, detect_from,
        namespace_directories, resolve_user_dirs,
    };
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
}
