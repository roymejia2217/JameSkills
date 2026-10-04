use crate::{
    Diagnostic,
    domain::{ValidatedBundle, validate_bundle},
    ports::filesystem::FileSystemPort,
};
use std::path::Path;
use std::sync::Arc;

/// Read-only library use cases. Persistence is introduced in a later slice;
/// this service validates imports through the filesystem port and pure domain.
pub struct LibraryService {
    filesystem: Arc<dyn FileSystemPort>,
}

impl LibraryService {
    pub fn new(filesystem: Arc<dyn FileSystemPort>) -> Self {
        Self { filesystem }
    }

    pub fn validate_import(&self, root: &Path) -> Result<ValidatedBundle, Vec<Diagnostic>> {
        let files = self.filesystem.read_bundle_directory(root)?;
        validate_bundle(&files)
    }
}

#[cfg(test)]
mod tests {
    use super::LibraryService;
    use crate::{
        Diagnostic, PortablePath,
        ports::filesystem::{BundleFiles, FileSystemPort},
    };
    use std::path::Path;
    use std::sync::Arc;

    struct MemoryFileSystem {
        result: Result<BundleFiles, Vec<Diagnostic>>,
    }

    impl FileSystemPort for MemoryFileSystem {
        fn read_bundle_directory(&self, _root: &Path) -> Result<BundleFiles, Vec<Diagnostic>> {
            self.result.clone()
        }
    }

    fn valid_files() -> BundleFiles {
        [
            (
                "SKILL.md",
                include_str!("../../../../docs/examples/repository-foundation/SKILL.md"),
            ),
            (
                "jameskills.toml",
                include_str!("../../../../docs/examples/repository-foundation/jameskills.toml"),
            ),
            (
                "policies/repository.toml",
                include_str!(
                    "../../../../docs/examples/repository-foundation/policies/repository.toml"
                ),
            ),
            (
                "guidance/repository.toml",
                include_str!(
                    "../../../../docs/examples/repository-foundation/guidance/repository.toml"
                ),
            ),
        ]
        .into_iter()
        .map(|(path, source)| {
            (
                PortablePath::new(path.to_owned()).unwrap(),
                source.as_bytes().to_vec(),
            )
        })
        .collect()
    }

    #[test]
    fn validation_uses_the_injected_filesystem_and_returns_domain_summary() {
        let service = LibraryService::new(Arc::new(MemoryFileSystem {
            result: Ok(valid_files()),
        }));

        let result = service.validate_import(Path::new("selected-bundle"));
        assert_eq!(result.unwrap().manifest().slug(), "repository-foundation");
    }

    #[test]
    fn validation_preserves_filesystem_diagnostics() {
        let service = LibraryService::new(Arc::new(MemoryFileSystem {
            result: Err(vec![Diagnostic::error(
                "bundle.file.unreadable",
                "Bundle file is not readable.",
            )]),
        }));

        let errors = match service.validate_import(Path::new("selected-bundle")) {
            Ok(_) => panic!("filesystem errors must not be reported as success"),
            Err(errors) => errors,
        };
        assert_eq!(errors[0].code(), "bundle.file.unreadable");
    }
}
