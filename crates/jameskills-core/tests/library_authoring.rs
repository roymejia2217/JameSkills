use jameskills_core::{
    SkillId,
    application::PublishDraft,
    domain::{CreateSkill, PortablePath, SkillDraft, TrustState, validate_bundle},
    ports::SaveDraftRequest,
};
use std::collections::BTreeMap;

fn draft_files() -> BTreeMap<PortablePath, Vec<u8>> {
    [(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"not valid frontmatter yet\n".to_vec(),
    )]
    .into_iter()
    .collect()
}

#[test]
fn semantically_invalid_content_is_preserved_as_an_editable_draft() {
    let files = draft_files();
    assert!(validate_bundle(&files).is_err());
    let draft = SkillDraft::new(SkillId::new(), None, 1, files.clone()).unwrap();

    assert_eq!(draft.generation(), 1);
    assert_eq!(draft.files(), &files);
}

#[test]
fn replacing_draft_bytes_advances_generation_without_moving_its_base_head() {
    let skill_id = SkillId::new();
    let base_head = jameskills_core::domain::RevisionId::from_digest([7; 32]);
    let first = SkillDraft::new(skill_id, Some(base_head.clone()), 5, draft_files()).unwrap();
    let replacement: BTreeMap<PortablePath, Vec<u8>> = [(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"edited but still invalid\n".to_vec(),
    )]
    .into_iter()
    .collect();
    let second = first.replace_files(replacement.clone()).unwrap();

    assert_eq!(second.generation(), 6);
    assert_eq!(second.base_head(), Some(&base_head));
    assert_eq!(second.files(), &replacement);
}

#[test]
fn drafts_reject_zero_generation_and_resource_overflow() {
    assert!(SkillDraft::new(SkillId::new(), None, 0, BTreeMap::new()).is_err());

    let oversized = [(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        vec![b'x'; 20 * 1024 * 1024 + 1],
    )]
    .into_iter()
    .collect();
    assert!(SkillDraft::new(SkillId::new(), None, 1, oversized).is_err());
}

#[test]
fn quarantined_draft_state_survives_content_edits() {
    let draft = SkillDraft::new_quarantined(SkillId::new(), None, 1, draft_files()).unwrap();
    let edited = draft.replace_files(draft_files()).unwrap();

    assert_eq!(draft.trust_state(), TrustState::Quarantined);
    assert_eq!(edited.trust_state(), TrustState::Quarantined);
}

#[test]
fn draft_write_requests_require_the_next_generation_and_compare_base_head() {
    let base = jameskills_core::domain::RevisionId::from_digest([8; 32]);
    let first = SkillDraft::new(SkillId::new(), Some(base.clone()), 1, draft_files()).unwrap();
    assert!(SaveDraftRequest::new(first.clone(), None, None).is_ok());

    let second = first.replace_files(draft_files()).unwrap();
    assert!(SaveDraftRequest::new(second.clone(), Some(1), Some(base.clone())).is_ok());
    assert!(SaveDraftRequest::new(second, Some(0), Some(base)).is_err());
}

#[test]
fn create_skill_generates_one_identity_and_a_valid_editable_template() {
    let create = CreateSkill::new(
        "repository-basics".to_owned(),
        "Repository basics".to_owned(),
    )
    .unwrap();
    let draft = create.initial_draft().unwrap();
    let validated = validate_bundle(draft.files()).unwrap();

    assert_eq!(draft.skill_id(), create.skill_id());
    assert_eq!(validated.manifest().id(), create.skill_id());
    assert_eq!(validated.manifest().slug(), "repository-basics");
    assert_eq!(validated.manifest().display_name(), "Repository basics");
    assert_eq!(draft.generation(), 1);
}

#[test]
fn create_skill_rejects_multicomponent_slugs_and_controlled_names() {
    assert!(CreateSkill::new("nested/slug".to_owned(), "Good name".to_owned()).is_err());
    assert!(CreateSkill::new("valid-slug".to_owned(), "Bad\nName".to_owned()).is_err());
}

#[test]
fn publish_request_binds_generation_and_canonicalizes_the_observed_head_set() {
    let skill_id = SkillId::new();
    let first = jameskills_core::domain::RevisionId::from_digest([1; 32]);
    let second = jameskills_core::domain::RevisionId::from_digest([2; 32]);
    let request = PublishDraft::new(
        skill_id,
        3,
        vec![second.clone(), first.clone(), first.clone()],
    )
    .unwrap();

    assert_eq!(request.skill_id(), skill_id);
    assert_eq!(request.draft_generation(), 3);
    assert_eq!(request.expected_heads(), &[first, second]);
    assert!(PublishDraft::new(skill_id, 0, vec![]).is_err());
}
