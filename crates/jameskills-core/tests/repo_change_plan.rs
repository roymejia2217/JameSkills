use jameskills_core::domain::{
    ApprovedRepoChange, ContentHash, RepoChangePlan, RepoTemplateId, policy::RepositoryHead,
};

fn hash(byte: char) -> ContentHash {
    ContentHash::parse_hex(&byte.to_string().repeat(64)).unwrap()
}

fn head(byte: char) -> RepositoryHead {
    RepositoryHead::parse(&byte.to_string().repeat(40)).unwrap()
}

fn plan(root: ContentHash, head: RepositoryHead, previous: Option<&[u8]>) -> RepoChangePlan {
    RepoChangePlan::new(
        RepoTemplateId::RustCi,
        root,
        head,
        previous,
        b"app-owned template bytes",
        "+ generated workflow\n".to_owned(),
    )
    .unwrap()
}

#[test]
fn confirmation_digest_binds_root_head_target_prior_and_proposed_state() {
    let base = plan(hash('a'), head('b'), Some(b"previous"));
    let next_preview = plan(hash('a'), head('b'), Some(b"previous"));

    assert_eq!(
        base.target().as_str(),
        ".github/workflows/jameskills-ci.yml"
    );
    assert_eq!(
        base.confirmation_digest(),
        next_preview.confirmation_digest(),
        "operation UUID must not make identical previews differ"
    );
    assert_ne!(
        base.confirmation_digest(),
        plan(hash('c'), head('b'), Some(b"previous")).confirmation_digest()
    );
    assert_ne!(
        base.confirmation_digest(),
        plan(hash('a'), head('c'), Some(b"previous")).confirmation_digest()
    );
    assert_ne!(
        base.confirmation_digest(),
        plan(hash('a'), head('b'), Some(b"changed")).confirmation_digest()
    );
    assert_ne!(
        base.confirmation_digest(),
        plan(hash('a'), head('b'), None).confirmation_digest()
    );
    let changed_preview = RepoChangePlan::new(
        RepoTemplateId::RustCi,
        hash('a'),
        head('b'),
        Some(b"previous"),
        b"app-owned template bytes",
        "+ different displayed diff\n".to_owned(),
    )
    .unwrap();
    assert_ne!(
        base.confirmation_digest(),
        changed_preview.confirmation_digest(),
        "the exact displayed diff is part of confirmation"
    );
}

#[test]
fn approval_requires_the_exact_digest_and_retains_the_immutable_plan() {
    let preview = plan(hash('a'), head('b'), None);
    let expected_digest = preview.confirmation_digest().clone();
    let approved =
        ApprovedRepoChange::after_explicit_digest_confirmation(preview, &expected_digest).unwrap();
    assert_eq!(approved.plan().confirmation_digest(), &expected_digest);

    let preview = plan(hash('a'), head('b'), None);
    assert!(ApprovedRepoChange::after_explicit_digest_confirmation(preview, &hash('f')).is_err());
}

#[test]
fn plan_rejects_oversized_template_and_diff_bytes() {
    let oversized = vec![b'x'; 1024 * 1024 + 1];
    assert!(
        RepoChangePlan::new(
            RepoTemplateId::RustCi,
            hash('a'),
            head('b'),
            None,
            &oversized,
            "+ template\n".to_owned(),
        )
        .is_err()
    );
    let oversized_previous = vec![b'x'; 1024 * 1024 + 1];
    assert!(
        RepoChangePlan::new(
            RepoTemplateId::RustCi,
            hash('a'),
            head('b'),
            Some(&oversized_previous),
            b"template",
            "+ template\n".to_owned(),
        )
        .is_err()
    );
    assert!(
        RepoChangePlan::new(
            RepoTemplateId::RustCi,
            hash('a'),
            head('b'),
            None,
            b"template",
            String::new(),
        )
        .is_err(),
        "a plan with no visible diff cannot be approved"
    );
    assert!(
        RepoChangePlan::new(
            RepoTemplateId::RustCi,
            hash('a'),
            head('b'),
            None,
            b"template",
            "x".repeat(2 * 1024 * 1024 + 1),
        )
        .is_err()
    );
    assert!(
        RepoChangePlan::new(
            RepoTemplateId::RustCi,
            hash('a'),
            head('b'),
            None,
            b"template",
            "unsafe\u{1b}[31m diff".to_owned(),
        )
        .is_err()
    );
}
