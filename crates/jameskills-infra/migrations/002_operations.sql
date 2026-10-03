-- 002_operations.sql — drafts, deletions, operations and installations (T037.a).
-- Runs inside one transaction and is re-runnable via IF NOT EXISTS.
-- user_version is owned by the storage runner, never set here.
-- Journals and receipts carry redacted payloads only: no tokens or keys.
BEGIN;
CREATE TABLE IF NOT EXISTS drafts(
    skill_id TEXT PRIMARY KEY,
    base_head TEXT,
    draft_json BLOB NOT NULL,
    generation INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS deletions(
    skill_id TEXT NOT NULL,
    deletion_revision_id TEXT PRIMARY KEY,
    observed_heads_json BLOB NOT NULL
);
CREATE TABLE IF NOT EXISTS operations(
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    state TEXT NOT NULL,
    journal_json BLOB NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS installations(
    id TEXT PRIMARY KEY,
    agent TEXT NOT NULL,
    scope TEXT NOT NULL,
    target_path TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    revision_id TEXT NOT NULL,
    receipt_json BLOB NOT NULL
);
COMMIT;
