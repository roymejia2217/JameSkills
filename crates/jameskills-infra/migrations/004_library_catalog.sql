-- 004_library_catalog.sql — indexed catalog metadata (T039.b).
-- Keep query-time listing metadata-only; never unpack blobs while paging.
BEGIN;
CREATE TABLE IF NOT EXISTS library_catalog(
    skill_id TEXT PRIMARY KEY REFERENCES skills(id),
    normalized_display_name TEXT NOT NULL
);
INSERT OR IGNORE INTO library_catalog(skill_id, normalized_display_name)
    SELECT id, display_name FROM skills;
CREATE INDEX IF NOT EXISTS skills_catalog_order
    ON library_catalog(normalized_display_name, skill_id);
CREATE INDEX IF NOT EXISTS revisions_by_skill_and_id
    ON revisions(skill_id, id);
CREATE TABLE IF NOT EXISTS revision_tags(
    revision_id TEXT NOT NULL REFERENCES revisions(id),
    tag TEXT NOT NULL,
    PRIMARY KEY(revision_id, tag)
);
CREATE INDEX IF NOT EXISTS revision_tags_by_tag ON revision_tags(tag, revision_id);
CREATE TABLE IF NOT EXISTS revision_capabilities(
    revision_id TEXT NOT NULL REFERENCES revisions(id),
    capability TEXT NOT NULL,
    PRIMARY KEY(revision_id, capability)
);
CREATE INDEX IF NOT EXISTS revision_capabilities_by_capability
    ON revision_capabilities(capability, revision_id);
COMMIT;
