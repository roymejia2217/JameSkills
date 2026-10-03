-- 001_library.sql — skills, revisions and heads (T037.a).
-- Runs inside one transaction and is re-runnable via IF NOT EXISTS.
-- user_version is owned by the storage runner, never set here.
-- revision_parents.parent_revision_id carries no foreign key by design:
-- a parent may not be downloaded yet and stays quarantined until validated.
BEGIN;
CREATE TABLE IF NOT EXISTS skills(
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS revisions(
    id TEXT PRIMARY KEY,
    skill_id TEXT NOT NULL REFERENCES skills(id),
    bundle_hash TEXT NOT NULL,
    semantic_version TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    state TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS revision_parents(
    revision_id TEXT NOT NULL REFERENCES revisions(id),
    parent_revision_id TEXT NOT NULL,
    PRIMARY KEY(revision_id, parent_revision_id)
);
CREATE TABLE IF NOT EXISTS skill_heads(
    skill_id TEXT NOT NULL REFERENCES skills(id),
    revision_id TEXT NOT NULL REFERENCES revisions(id),
    PRIMARY KEY(skill_id, revision_id)
);
COMMIT;
