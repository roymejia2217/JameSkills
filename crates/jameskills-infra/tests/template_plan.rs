use jameskills_core::{
    AppError, ContentHash,
    domain::{RepoTemplateId, policy::RepositoryHead},
    ports::process::ApprovedRoot,
};
use jameskills_infra::fs::{plan_repo_template, repository_root_fingerprint};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_REPOSITORY: AtomicU64 = AtomicU64::new(0);

struct TempRepository(PathBuf);

impl TempRepository {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jameskills-template-plan-{}-{}",
            std::process::id(),
            NEXT_REPOSITORY.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn approved_root(&self) -> ApprovedRoot {
        ApprovedRoot::from_absolute_path(std::fs::canonicalize(&self.0).unwrap()).unwrap()
    }
}

impl Drop for TempRepository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn head() -> RepositoryHead {
    RepositoryHead::parse(&"a".repeat(40)).unwrap()
}

#[test]
fn missing_target_preview_is_bounded_and_does_not_create_files_or_directories() {
    let repository = TempRepository::new();
    let root = repository.approved_root();
    let fingerprint = repository_root_fingerprint(&root).unwrap();
    let preview = plan_repo_template(&root, &fingerprint, &head(), RepoTemplateId::RustCi).unwrap();
    let target = repository.path().join(preview.target().as_str());

    assert!(
        preview
            .diff()
            .contains("+++ b/.github/workflows/jameskills-ci.yml")
    );
    assert!(preview.diff().contains("+name: JameSkills Rust CI"));
    assert!(
        preview
            .diff()
            .contains("actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803")
    );
    assert!(
        preview
            .diff()
            .contains("dtolnay/rust-toolchain@6bed0761d98439e5a578e2877258200ad565ba87 # v1.98.1")
    );
    assert!(preview.diff().contains("toolchain: 1.95.0"));
    assert!(!preview.diff().contains("TEMPLATE DE PLAN"));
    assert!(!preview.diff().contains("run: exit 1"));
    assert!(!target.exists());
    assert!(!repository.path().join(".github").exists());
}

#[test]
fn existing_unowned_target_is_conflict_and_remains_byte_identical() {
    let repository = TempRepository::new();
    let target = repository.path().join(RepoTemplateId::RustCi.target());
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let original = b"user-owned workflow\n";
    std::fs::write(&target, original).unwrap();
    let root = repository.approved_root();
    let fingerprint = repository_root_fingerprint(&root).unwrap();

    assert!(matches!(
        plan_repo_template(&root, &fingerprint, &head(), RepoTemplateId::RustCi),
        Err(AppError::Conflict { .. })
    ));
    assert_eq!(std::fs::read(target).unwrap(), original);
}

#[test]
fn root_fingerprint_mismatch_is_rejected_before_preview() {
    let repository = TempRepository::new();
    let root = repository.approved_root();
    let wrong_fingerprint = ContentHash::parse_hex(&"b".repeat(64)).unwrap();

    assert!(matches!(
        plan_repo_template(&root, &wrong_fingerprint, &head(), RepoTemplateId::RustCi),
        Err(AppError::Conflict { .. })
    ));
    assert!(!repository.path().join(".github").exists());
}

#[cfg(unix)]
#[test]
fn symlinked_target_parent_is_blocked_without_writing_outside_root() {
    use std::os::unix::fs::symlink;

    let repository = TempRepository::new();
    let outside = TempRepository::new();
    symlink(outside.path(), repository.path().join(".github")).unwrap();
    let root = repository.approved_root();
    let fingerprint = repository_root_fingerprint(&root).unwrap();

    assert!(matches!(
        plan_repo_template(&root, &fingerprint, &head(), RepoTemplateId::RustCi),
        Err(AppError::PermissionDenied { .. })
    ));
    assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
}
