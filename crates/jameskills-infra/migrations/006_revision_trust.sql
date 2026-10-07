-- 006_revision_trust.sql — local-only reviewed/quarantined trust per revision (T042).
BEGIN;
CREATE TABLE IF NOT EXISTS revision_trust(
    revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
    trust_state TEXT NOT NULL CHECK(trust_state IN ('quarantined', 'reviewed')),
    source_kind TEXT NOT NULL CHECK(source_kind IN ('directory', 'archive', 'plain-skill'))
);
COMMIT;
