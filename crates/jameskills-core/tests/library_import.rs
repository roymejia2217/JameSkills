use jameskills_core::domain::{
    ImportClassification, ImportPreview, ImportResolution, ImportSourceKind, PortablePath,
    RevisionId, TrustState,
};
use std::collections::BTreeMap;

fn valid_bundle_files() -> BTreeMap<PortablePath, Vec<u8>> {
    [
        (
            "SKILL.md",
            include_bytes!("../../../docs/examples/repository-foundation/SKILL.md").as_slice(),
        ),
        (
            "jameskills.toml",
            include_bytes!("../../../docs/examples/repository-foundation/jameskills.toml")
                .as_slice(),
        ),
        (
            "policies/repository.toml",
            include_bytes!("../../../docs/examples/repository-foundation/policies/repository.toml")
                .as_slice(),
        ),
        (
            "guidance/repository.toml",
            include_bytes!("../../../docs/examples/repository-foundation/guidance/repository.toml")
                .as_slice(),
        ),
    ]
    .into_iter()
    .map(|(path, bytes)| (PortablePath::new(path.to_owned()).unwrap(), bytes.to_vec()))
    .collect()
}

#[test]
fn import_preview_enumerates_valid_candidate_and_starts_quarantined() {
    let preview = ImportPreview::new(
        valid_bundle_files(),
        ImportSourceKind::Archive,
        false,
        vec![],
        None,
    )
    .unwrap();

    assert_eq!(preview.slug(), "repository-foundation");
    assert_eq!(preview.semantic_version(), "1.0.0");
    assert_eq!(preview.files().len(), 4);
    assert_eq!(preview.source_kind(), ImportSourceKind::Archive);
    assert_eq!(*preview.classification(), ImportClassification::NewSkill);
    assert_eq!(preview.trust_state(), TrustState::Quarantined);
    let confirmation = preview
        .confirmation_digest(ImportResolution::AddConcurrentRoot)
        .unwrap();
    assert_eq!(
        preview
            .confirmation_digest(ImportResolution::AddConcurrentRoot)
            .unwrap(),
        confirmation
    );
    assert!(
        preview
            .confirmation_digest(ImportResolution::KeepExisting)
            .is_err()
    );
}

#[test]
fn import_preview_preserves_existing_heads_and_classifies_duplicates_without_writes() {
    let first = RevisionId::from_digest([1; 32]);
    let second = RevisionId::from_digest([2; 32]);
    let conflict = ImportPreview::new(
        valid_bundle_files(),
        ImportSourceKind::Directory,
        true,
        vec![second.clone(), first.clone()],
        None,
    )
    .unwrap();
    assert_eq!(conflict.current_heads(), &[first.clone(), second.clone()]);
    assert_eq!(
        conflict.classification(),
        &ImportClassification::Conflict {
            current_heads: vec![first.clone(), second.clone()],
        }
    );

    let draft_only_conflict = ImportPreview::new(
        valid_bundle_files(),
        ImportSourceKind::Directory,
        true,
        vec![],
        None,
    )
    .unwrap();
    assert_eq!(
        draft_only_conflict.classification(),
        &ImportClassification::Conflict {
            current_heads: vec![],
        }
    );

    let duplicate = ImportPreview::new(
        valid_bundle_files(),
        ImportSourceKind::Archive,
        true,
        vec![first.clone()],
        Some(second.clone()),
    )
    .unwrap();
    assert_eq!(
        duplicate.classification(),
        &ImportClassification::Identical {
            revision_id: second,
        }
    );
    assert_eq!(duplicate.current_heads(), &[first]);
    assert_eq!(duplicate.trust_state(), TrustState::Quarantined);
}

#[test]
fn import_preview_revalidates_supplied_files_and_bounds_conflict_heads() {
    let invalid: BTreeMap<PortablePath, Vec<u8>> = [(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"not a complete suite".to_vec(),
    )]
    .into_iter()
    .collect();
    assert!(ImportPreview::new(invalid, ImportSourceKind::Directory, false, vec![], None).is_err());

    let heads = (0..129)
        .map(|value| RevisionId::from_digest([value as u8; 32]))
        .collect();
    assert!(
        ImportPreview::new(
            valid_bundle_files(),
            ImportSourceKind::Archive,
            true,
            heads,
            None
        )
        .is_err()
    );
}

#[test]
fn plain_skill_confirmation_only_allows_creation_of_a_quarantined_draft() {
    let source = b"---\nname: plain-instructions\ndescription: Plain imported guidance.\n---\n# Instructions\n";
    let (_, draft) =
        jameskills_core::domain::CreateSkill::from_plain_skill_source(source.to_vec()).unwrap();
    let preview = ImportPreview::new(
        draft.files().clone(),
        ImportSourceKind::PlainSkill,
        false,
        vec![],
        None,
    )
    .unwrap();

    assert!(
        preview
            .confirmation_digest(ImportResolution::CreateQuarantinedDraft)
            .is_ok()
    );
    assert!(
        preview
            .confirmation_digest(ImportResolution::AddConcurrentRoot)
            .is_err()
    );
}
