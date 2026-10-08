use jameskills_core::domain::{
    ContentHash, ImportSourceKind, RevisionKind, RevisionTrust, SaveRevisionRequest, SkillId,
    TrustState,
};

#[test]
fn committed_revision_trust_defaults_to_reviewed_and_can_preserve_quarantine_provenance() {
    let skill_id = SkillId::new();
    let default = SaveRevisionRequest::new(
        skill_id,
        Some(ContentHash::from_digest([1; 32])),
        Vec::new(),
        RevisionKind::Content,
        "1.0.0".to_owned(),
        1,
        Vec::new(),
    );
    assert_eq!(default.trust(), RevisionTrust::reviewed());

    let preserved = default
        .with_trust(RevisionTrust::quarantined(ImportSourceKind::Archive))
        .unwrap();
    assert_eq!(preserved.trust().state(), TrustState::Quarantined);
    assert_eq!(
        preserved.trust().source_kind(),
        Some(ImportSourceKind::Archive)
    );

    let tombstone = SaveRevisionRequest::new(
        skill_id,
        None,
        Vec::new(),
        RevisionKind::Tombstone {
            observed_heads: Vec::new(),
        },
        "0.0.0".to_owned(),
        1,
        Vec::new(),
    );
    assert!(
        tombstone
            .with_trust(RevisionTrust::quarantined(ImportSourceKind::Archive))
            .is_err()
    );
}
