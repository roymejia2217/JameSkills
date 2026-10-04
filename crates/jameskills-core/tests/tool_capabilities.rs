use jameskills_core::domain::guidance::{
    ToolAvailability, ToolCapabilitySupport, ToolDetection, ToolVersionStatus,
};
use jameskills_core::domain::{ToolId, ToolOperation};

fn requirement() -> semver::VersionReq {
    semver::VersionReq::parse(">=2.0.0, <3.0.0").unwrap()
}

fn detected(availability: ToolAvailability, version: Option<semver::Version>) -> ToolDetection {
    ToolDetection::from_probe(
        ToolId::Git,
        availability,
        version,
        &requirement(),
        &[ToolOperation::RepositoryRoot],
        "git.binary",
        "2026-10-04T00:00:00Z",
        "Git version probe completed.",
    )
    .unwrap()
}

#[test]
fn missing_incompatible_unknown_and_supported_tools_remain_distinct() {
    let missing = detected(ToolAvailability::Missing, None);
    assert_eq!(missing.availability(), ToolAvailability::Missing);
    assert_eq!(missing.version_status(), ToolVersionStatus::NotApplicable);
    assert_eq!(
        missing.capability(ToolOperation::RepositoryRoot),
        Some(ToolCapabilitySupport::Unsupported)
    );

    let unknown = detected(ToolAvailability::Candidate, None);
    assert_eq!(unknown.availability(), ToolAvailability::Candidate);
    assert_eq!(unknown.version_status(), ToolVersionStatus::Unknown);
    assert_eq!(
        unknown.capability(ToolOperation::RepositoryRoot),
        Some(ToolCapabilitySupport::NeedsVerification)
    );

    let candidate = detected(
        ToolAvailability::Candidate,
        Some(semver::Version::new(2, 55, 0)),
    );
    assert_eq!(candidate.version_status(), ToolVersionStatus::Compatible);
    assert_eq!(
        candidate.capability(ToolOperation::RepositoryRoot),
        Some(ToolCapabilitySupport::NeedsVerification)
    );
    let incompatible = detected(
        ToolAvailability::Candidate,
        Some(semver::Version::new(1, 9, 9)),
    );
    assert_eq!(incompatible.availability(), ToolAvailability::Candidate);
    assert_eq!(
        incompatible.version_status(),
        ToolVersionStatus::Incompatible
    );
    assert_eq!(
        incompatible.capability(ToolOperation::RepositoryRoot),
        Some(ToolCapabilitySupport::Unsupported)
    );

    let supported = detected(
        ToolAvailability::Verified,
        Some(semver::Version::new(2, 55, 0)),
    );
    assert_eq!(supported.availability(), ToolAvailability::Verified);
    assert_eq!(supported.version_status(), ToolVersionStatus::Compatible);
    assert_eq!(
        supported.capability(ToolOperation::RepositoryRoot),
        Some(ToolCapabilitySupport::Supported)
    );
    assert_eq!(supported.evidence().source_id(), "git.binary");
    assert_eq!(
        supported.evidence().tested_version(),
        Some(&semver::Version::new(2, 55, 0))
    );
}

#[test]
fn unknown_and_blocked_presence_never_become_supported() {
    for availability in [ToolAvailability::Unknown, ToolAvailability::Blocked] {
        let detection = detected(availability, None);
        assert_eq!(detection.version_status(), ToolVersionStatus::Unknown);
        assert_eq!(
            detection.capability(ToolOperation::RepositoryRoot),
            Some(ToolCapabilitySupport::NeedsVerification)
        );
    }
}

#[test]
fn tool_evidence_requires_an_app_owned_source_identifier() {
    let invalid = ToolDetection::from_probe(
        ToolId::Git,
        ToolAvailability::Verified,
        Some(semver::Version::new(2, 55, 0)),
        &requirement(),
        &[ToolOperation::RepositoryRoot],
        "https://example.invalid/git",
        "2026-10-04T00:00:00Z",
        "Git version probe completed.",
    );
    assert!(invalid.is_err());

    let invalid_timestamp = ToolDetection::from_probe(
        ToolId::Git,
        ToolAvailability::Verified,
        Some(semver::Version::new(2, 55, 0)),
        &requirement(),
        &[ToolOperation::RepositoryRoot],
        "git.binary",
        "not-a-dateZ",
        "Git version probe completed.",
    );
    assert!(invalid_timestamp.is_err());
}
