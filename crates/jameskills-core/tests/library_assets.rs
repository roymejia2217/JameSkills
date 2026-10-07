use jameskills_core::domain::{
    AssetFiles, AssetPreviewKind, ContentHash, PortablePath, add_asset, preview_asset,
    remove_asset, rename_bundle_path, replace_asset,
};

fn file(path: &str, bytes: &[u8]) -> (PortablePath, Vec<u8>) {
    (PortablePath::new(path.to_owned()).unwrap(), bytes.to_vec())
}

fn assets_with_reference() -> AssetFiles {
    [
        file("SKILL.md", b"See [guide](references/guide.md).\n"),
        file("references/guide.md", b"A guide\n"),
        file("assets/logo.svg", b"<svg><text>data</text></svg>"),
    ]
    .into_iter()
    .collect()
}

fn digest(bytes: &[u8]) -> ContentHash {
    use sha2::{Digest, Sha256};
    ContentHash::from_digest(Sha256::digest(bytes).into())
}

#[test]
fn asset_paths_and_inventory_mutations_are_portable_and_do_not_overwrite() {
    let files: AssetFiles = [file("SKILL.md", b"instructions")].into_iter().collect();
    let added = add_asset(&files, "assets/logo.svg", b"<svg/>".to_vec()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(added.len(), 2);
    assert!(add_asset(&added, "assets/logo.svg", b"replace".to_vec()).is_err());
    assert!(add_asset(&files, "../escape.txt", b"bad".to_vec()).is_err());
    assert!(add_asset(&files, "policies/extra.toml", b"not an asset".to_vec()).is_err());
    assert!(
        add_asset(
            &files,
            "assets/oversized.txt",
            vec![b'x'; 2 * 1024 * 1024 + 1],
        )
        .is_err()
    );
}

#[test]
fn replace_remove_and_rename_require_expected_bytes_and_preserve_references() {
    let files = assets_with_reference();
    let old = b"<svg><text>data</text></svg>";
    let expected = digest(old);
    assert!(replace_asset(&files, "assets/logo.svg", &digest(b"stale"), b"x".to_vec()).is_err());
    let replaced = replace_asset(
        &files,
        "assets/logo.svg",
        &expected,
        b"<svg>new</svg>".to_vec(),
    )
    .unwrap();
    assert_eq!(
        files[&PortablePath::new("assets/logo.svg".to_owned()).unwrap()],
        old
    );
    assert_eq!(
        replaced[&PortablePath::new("assets/logo.svg".to_owned()).unwrap()],
        b"<svg>new</svg>"
    );

    let referenced = digest(b"A guide\n");
    assert!(remove_asset(&files, "references/guide.md", &referenced).is_err());
    assert!(
        rename_bundle_path(
            &files,
            "references/guide.md",
            "references/new.md",
            &referenced
        )
        .is_err()
    );

    let mut unlinked = files.clone();
    unlinked.insert(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"No resource links here.\n".to_vec(),
    );
    let renamed = rename_bundle_path(
        &unlinked,
        "references/guide.md",
        "references/new.md",
        &referenced,
    )
    .unwrap();
    assert!(!renamed.contains_key(&PortablePath::new("references/guide.md".to_owned()).unwrap()));
    assert!(renamed.contains_key(&PortablePath::new("references/new.md".to_owned()).unwrap()));
    let removed = remove_asset(&unlinked, "references/guide.md", &referenced).unwrap();
    assert!(!removed.contains_key(&PortablePath::new("references/guide.md".to_owned()).unwrap()));
}

#[test]
fn asset_inventory_rejects_casefold_collisions_and_preview_never_executes() {
    let files: AssetFiles = [file("SKILL.md", b"safe")].into_iter().collect();
    let first = add_asset(
        &files,
        "assets/Logo.svg",
        b"<svg><script>data</script></svg>".to_vec(),
    )
    .unwrap();
    assert!(add_asset(&first, "assets/logo.svg", b"case collision".to_vec()).is_err());
    let path = PortablePath::new("assets/Logo.svg".to_owned()).unwrap();
    let preview = preview_asset(&path, &first[&path]);
    assert_eq!(preview.kind(), AssetPreviewKind::Text);
    assert!(preview.text().unwrap().contains("<script>data</script>"));

    let binary_path = PortablePath::new("assets/run.bin".to_owned()).unwrap();
    let binary = preview_asset(&binary_path, b"MZ\0executable data");
    assert_eq!(binary.kind(), AssetPreviewKind::Unsupported);
    assert!(binary.text().is_none());
}

#[test]
fn remove_rejects_percent_encoded_and_case_insensitive_references() {
    let files: AssetFiles = [
        file("SKILL.md", b"See [guide](references/GUIDE%20FILE.md).\n"),
        file("references/guide file.md", b"guide"),
    ]
    .into_iter()
    .collect();
    let expected = digest(b"guide");
    assert!(remove_asset(&files, "references/guide file.md", &expected).is_err());
}
