use jameskills_infra::{composition::build_services, platform::UserDirectories};
use std::path::PathBuf;

#[test]
fn composed_library_service_validates_a_real_bundle_without_creating_user_dirs() {
    let root = std::env::temp_dir().join(format!("jameskills-validation-{}", std::process::id()));
    let directories = UserDirectories {
        config: root.join("config"),
        data: root.join("data"),
        cache: root.join("cache"),
    };
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/repository-foundation");

    let runtime = build_services(directories.clone()).unwrap();
    let report = runtime.library().validate_import(&fixture).unwrap();

    assert_eq!(report.manifest().slug(), "repository-foundation");
    assert_eq!(report.file_count(), 10);
    assert!(!directories.config.exists());
    assert!(!directories.data.exists());
    assert!(!directories.cache.exists());
}
