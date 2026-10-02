use chrono::DateTime;
use jameskills_infra::{
    composition::{RuntimeServices, build_services},
    platform::UserDirectories,
};
use std::path::PathBuf;

fn directories(root: PathBuf) -> UserDirectories {
    UserDirectories {
        config: root.join("config"),
        data: root.join("data"),
        cache: root.join("cache"),
    }
}

fn accepts_runtime(_: RuntimeServices) {}

#[test]
fn factory_builds_shared_runtime_facts_dirs_and_clock_without_creating_dirs() {
    let root = std::env::temp_dir().join(format!("jameskills-factory-{}", std::process::id()));
    let dirs = directories(root);
    let expected = dirs.clone();
    let runtime = build_services(dirs).unwrap();

    assert!(runtime.directories() == &expected);
    assert!(!expected.config.exists());
    assert!(!expected.data.exists());
    assert!(!expected.cache.exists());
    assert!(!runtime.facts().architecture.is_empty());
    assert!(DateTime::parse_from_rfc3339(&runtime.clock().now_utc()).is_ok());
    assert!(runtime.clock().monotonic_ms() <= runtime.clock().monotonic_ms());
    accepts_runtime(runtime);
}

#[test]
fn factory_rejects_relative_and_overlapping_directories() {
    let relative = UserDirectories {
        config: PathBuf::from("config"),
        data: PathBuf::from("data"),
        cache: PathBuf::from("cache"),
    };
    assert!(build_services(relative).is_err());

    let root = std::env::temp_dir().join("jameskills-invalid-layout");
    let overlapping = UserDirectories {
        config: root.clone(),
        data: root.join("nested-data"),
        cache: std::env::temp_dir().join("jameskills-cache-only"),
    };
    assert!(build_services(overlapping).is_err());
}
