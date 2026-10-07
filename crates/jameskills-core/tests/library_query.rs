use jameskills_core::{
    SkillId,
    ports::{LibraryCursor, LibraryHistoryQuery, LibraryItemState, LibraryQuery},
};

#[test]
fn library_query_trims_casefolds_and_deduplicates_filters() {
    let query = LibraryQuery::new(
        Some("  ÄBC%_  "),
        vec!["Rust".to_owned(), " rust ".to_owned(), "CI".to_owned()],
        vec!["Repository-Read".to_owned()],
        LibraryItemState::Conflicted,
        None,
        50,
    )
    .unwrap();

    assert_eq!(query.search(), Some("äbc%_"));
    assert_eq!(query.tags(), &["ci".to_owned(), "rust".to_owned()]);
    assert_eq!(query.capabilities(), &["repository-read".to_owned()]);
    assert_eq!(query.state(), LibraryItemState::Conflicted);
    assert_eq!(query.page_size(), 50);
}

#[test]
fn library_query_rejects_unbounded_page_search_and_filter_inputs() {
    assert!(LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 0).is_err());
    assert!(LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 51).is_err());
    assert!(
        LibraryQuery::new(
            Some(&"x".repeat(257)),
            vec![],
            vec![],
            LibraryItemState::Any,
            None,
            50,
        )
        .is_err()
    );
    assert!(
        LibraryQuery::new(
            None,
            vec!["x".repeat(129)],
            vec![],
            LibraryItemState::Any,
            None,
            50,
        )
        .is_err()
    );
}

#[test]
fn library_cursor_uses_the_stable_normalized_name_and_skill_id_pair() {
    let id = SkillId::parse("f9c0199f-c4ce-4b04-85dd-ae12a7db292b").unwrap();
    let cursor = LibraryCursor::new("  Répository  ", id).unwrap();

    assert_eq!(cursor.normalized_display_name(), "répository");
    assert_eq!(cursor.skill_id(), id);
    assert!(LibraryCursor::new("  ", id).is_err());
}

#[test]
fn library_history_pages_are_bounded() {
    let id = SkillId::parse("f9c0199f-c4ce-4b04-85dd-ae12a7db292b").unwrap();
    assert!(LibraryHistoryQuery::new(id, None, 0).is_err());
    assert!(LibraryHistoryQuery::new(id, None, 101).is_err());
    assert!(LibraryHistoryQuery::new(id, None, 100).is_ok());
}
