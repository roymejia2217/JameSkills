use std::path::PathBuf;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserDirectories {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformError {
    BaseDirectoriesUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    UserDirectories {
        config: bases.config.join(product_directory),
        data: bases.data.join(product_directory),
        cache: bases.cache.join(product_directory),
    }
}

#[cfg(test)]
mod tests {
    use super::{DirectoryBases, HostPlatform, namespace_directories};
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
        assert!(dirs.data.ends_with("JameSkills"));
        assert!(dirs.cache.ends_with("JameSkills"));
        assert_ne!(
            dirs.config,
            PathBuf::from("/home/example/.config/JameSkills")
        );
    }
}
