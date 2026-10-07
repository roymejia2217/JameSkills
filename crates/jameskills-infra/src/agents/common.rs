use jameskills_core::{
    AppError, AppResult,
    domain::{BundleEntry, EntryKind, ValidatedBundle, hash_bundle, validate_bundle_inventory},
    ports::filesystem::BundleFiles,
};
use std::path::{Path, PathBuf};

pub struct AgentArtifact {
    target: PathBuf,
    files: BundleFiles,
}

impl AgentArtifact {
    pub fn target(&self) -> &Path {
        &self.target
    }

    pub fn files(&self) -> &BundleFiles {
        &self.files
    }
}

pub(crate) fn plan_file_copy_artifact(
    target: PathBuf,
    validated: &ValidatedBundle,
    files: BundleFiles,
) -> AppResult<AgentArtifact> {
    let entries = files
        .iter()
        .map(|(path, bytes)| {
            let size = u64::try_from(bytes.len()).map_err(|_| AppError::CryptoInvalid)?;
            Ok(BundleEntry::new(
                path.clone(),
                EntryKind::RegularFile,
                size,
                size,
            ))
        })
        .collect::<AppResult<Vec<_>>>()?;
    let inventory = validate_bundle_inventory(&entries).map_err(AppError::Validation)?;
    if hash_bundle(&inventory, &files)? != *validated.content_hash() {
        return Err(AppError::Conflict { current: vec![] });
    }
    Ok(AgentArtifact { target, files })
}

pub(crate) fn find_agent_executable_candidate(executable_name: &str) -> Option<PathBuf> {
    if !valid_agent_executable_name(executable_name) {
        return None;
    }
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path)
        .filter(|directory| directory.is_absolute())
        .take(128)
    {
        #[cfg(windows)]
        let names = [
            format!("{executable_name}.exe"),
            format!("{executable_name}.cmd"),
        ];
        #[cfg(not(windows))]
        let names = [executable_name.to_owned()];
        for name in names {
            let candidate = directory.join(name);
            let Ok(metadata) = std::fs::symlink_metadata(&candidate) else {
                continue;
            };
            if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
                continue;
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    continue;
                }
            }
            return Some(candidate);
        }
    }
    None
}

fn valid_agent_executable_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
