use jameskills_core::domain::{
    BundleEntry, EntryKind, PortablePath, ValidatedInventory, validate_bundle_inventory,
};

fn path(value: &str) -> PortablePath {
    PortablePath::new(value.to_owned()).unwrap()
}

fn file(value: &str, size: u64) -> BundleEntry {
    BundleEntry::new(path(value), EntryKind::RegularFile, size, size)
}

fn code<T>(result: &Result<T, Vec<jameskills_core::Diagnostic>>) -> &'static str {
    match result {
        Err(errors) => errors[0].code(),
        Ok(_) => panic!("expected bundle inventory rejection"),
    }
}

fn valid_inventory() -> Vec<BundleEntry> {
    vec![file("SKILL.md", 512), file("assets/logo.png", 4096)]
}

#[test]
fn accepts_bounded_regular_files_and_reports_inventory_totals() {
    let inventory: ValidatedInventory = validate_bundle_inventory(&valid_inventory()).unwrap();
    assert_eq!(inventory.file_count(), 2);
    assert_eq!(inventory.total_compressed_bytes(), 4608);
    assert_eq!(inventory.total_uncompressed_bytes(), 4608);
    assert!(
        inventory
            .files()
            .iter()
            .any(|entry| entry.path().as_str() == "SKILL.md")
    );
}

#[test]
fn rejects_duplicate_paths_and_full_unicode_case_fold_collisions() {
    let duplicate = vec![file("SKILL.md", 12), file("SKILL.md", 12)];
    assert_eq!(
        code(&validate_bundle_inventory(&duplicate)),
        "bundle.path.duplicate"
    );

    let unicode_collision = vec![
        file("SKILL.md", 12),
        file("Straße.txt", 1),
        file("STRASSE.txt", 1),
    ];
    assert_eq!(
        code(&validate_bundle_inventory(&unicode_collision)),
        "bundle.path.case_collision"
    );

    let directory_collision = vec![
        file("SKILL.md", 12),
        file("Docs/one.md", 1),
        file("docs/two.md", 1),
    ];
    assert_eq!(
        code(&validate_bundle_inventory(&directory_collision)),
        "bundle.path.case_collision"
    );
}

#[test]
fn rejects_symlinks_hardlinks_directories_and_reparse_points() {
    for kind in [
        EntryKind::Directory,
        EntryKind::SymbolicLink,
        EntryKind::HardLink,
        EntryKind::ReparsePoint,
    ] {
        let entry = BundleEntry::new(path("SKILL.md"), kind, 0, 0);
        assert_eq!(
            code(&validate_bundle_inventory(&[entry])),
            "bundle.entry.kind.unsupported"
        );
    }
}

#[test]
fn rejects_missing_skill_document_and_oversized_skill_text() {
    assert_eq!(
        code(&validate_bundle_inventory(&[file("README.md", 1)])),
        "bundle.skill.missing"
    );

    let oversized_skill = file("SKILL.md", 256 * 1024 + 1);
    assert_eq!(
        code(&validate_bundle_inventory(&[oversized_skill])),
        "bundle.file.too_large"
    );
}

#[test]
fn rejects_oversized_text_entry_file_count_and_total_archive_size() {
    let oversized_text = vec![file("SKILL.md", 1), file("large.md", 2 * 1024 * 1024 + 1)];
    assert_eq!(
        code(&validate_bundle_inventory(&oversized_text)),
        "bundle.file.too_large"
    );

    let too_many = (0..2001)
        .map(|index| file(&format!("file-{index}.bin"), 1))
        .collect::<Vec<_>>();
    assert_eq!(
        code(&validate_bundle_inventory(&too_many)),
        "bundle.file_count.limit"
    );

    let bomb = vec![
        file("SKILL.md", 12),
        BundleEntry::new(
            path("assets/blob.bin"),
            EntryKind::RegularFile,
            1,
            21 * 1024 * 1024,
        ),
    ];
    assert_eq!(
        code(&validate_bundle_inventory(&bomb)),
        "bundle.total_size.limit"
    );
}
