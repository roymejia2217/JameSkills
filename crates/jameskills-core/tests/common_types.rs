use jameskills_core::{
    AppError, AppResult, ContentHash, Diagnostic, DiagnosticSeverity, OperationId, PortablePath,
    RevisionId, SkillId,
};
use serde::Deserialize;
use serde::de::value::StringDeserializer;

#[test]
fn common_errors_have_stable_redacted_display_and_no_source_chain() {
    let secret = "canary-secret-17";
    let errors = [
        AppError::Validation(vec![Diagnostic::error("input.invalid", "Invalid input")]),
        AppError::Conflict { current: vec![] },
        AppError::CapabilityUnavailable {
            id: secret.to_owned(),
            guidance_id: "setup.tool".to_owned(),
        },
        AppError::AuthenticationRequired,
        AppError::PermissionDenied {
            operation: secret.to_owned(),
        },
        AppError::Cancelled,
    ];

    for error in errors {
        assert!(!error.to_string().contains(secret));
        assert!(!format!("{error:?}").contains(secret));
        assert!(std::error::Error::source(&error).is_none());
    }
}

#[test]
fn app_result_alias_accepts_typed_errors() {
    let result: AppResult<()> = Err(AppError::Cancelled);
    assert!(matches!(result, Err(AppError::Cancelled)));
}

#[test]
fn uuid_ids_parse_and_generate_without_exposing_tuple_constructors() {
    let skill = SkillId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let operation = OperationId::parse("550e8400-e29b-41d4-a716-446655440001").unwrap();
    assert_eq!(
        skill.as_uuid().to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(
        operation.as_uuid().to_string(),
        "550e8400-e29b-41d4-a716-446655440001"
    );
    assert_ne!(SkillId::new(), SkillId::new());
    assert!(SkillId::parse("not-a-uuid").is_err());
}

#[test]
fn content_and_revision_hashes_require_lowercase_sha256_hex() {
    let digest = [0xab; 32];
    let content_hash = ContentHash::from_digest(digest);
    let revision = RevisionId::from_digest(digest);
    assert_eq!(content_hash.as_str(), &"ab".repeat(32));
    assert_eq!(revision.as_str(), &"ab".repeat(32));
    assert!(ContentHash::parse_hex(&"AB".repeat(32)).is_err());
    assert!(RevisionId::parse_hex(&"a".repeat(63)).is_err());
    assert!(RevisionId::parse_hex(&format!("{}g", "0".repeat(63))).is_err());
}

#[test]
fn portable_paths_accept_only_canonical_relative_paths() {
    assert_eq!(
        PortablePath::new("policies/repository.toml".into())
            .unwrap()
            .as_str(),
        "policies/repository.toml"
    );

    for invalid in [
        "",
        "/absolute/file",
        "C:/drive/file",
        "C:drive-relative",
        "\\\\server\\share",
        "folder\\file",
        "folder//file",
        "./file",
        "folder/../file",
        "folder/.",
        "folder/CON",
        "folder/prn.txt",
        "AUX.md",
        "nul.log",
        "COM1.txt",
        "nested/LPT9.json",
        "CON .txt",
        "dir/name.",
        "dir/name ",
        "bad\0name",
        "bad\nname",
        "café/e\u{301}.txt",
        &"a".repeat(241),
    ] {
        assert!(
            PortablePath::new(invalid.into()).is_err(),
            "accepted invalid path {invalid:?}"
        );
    }
}

#[test]
fn deserialization_cannot_bypass_portable_path_validation() {
    let result = PortablePath::deserialize(StringDeserializer::<serde::de::value::Error>::new(
        "../escape".to_owned(),
    ));
    assert!(result.is_err());
}

#[test]
fn deserialization_cannot_bypass_hash_identifier_validation() {
    let invalid_revision = RevisionId::deserialize(
        StringDeserializer::<serde::de::value::Error>::new("not-a-digest".to_owned()),
    );
    let invalid_content_hash = ContentHash::deserialize(StringDeserializer::<
        serde::de::value::Error,
    >::new("F".repeat(64)));
    assert!(invalid_revision.is_err());
    assert!(invalid_content_hash.is_err());
}

#[test]
fn diagnostics_are_serializable_and_keep_a_typed_severity() {
    let diagnostic = Diagnostic::new(
        "path.invalid",
        None,
        None,
        None,
        "Invalid portable path",
        DiagnosticSeverity::Error,
    );
    assert_eq!(diagnostic.code(), "path.invalid");
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Error);
    fn assert_serializable<T: serde::Serialize>(_: &T) {}
    assert_serializable(&diagnostic);
}
