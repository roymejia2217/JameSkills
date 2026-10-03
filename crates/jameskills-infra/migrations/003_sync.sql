-- 003_sync.sql — remote files, snapshots and guidance sessions (T037.a).
-- Runs inside one transaction and is re-runnable via IF NOT EXISTS.
-- user_version is owned by the storage runner, never set here.
-- Snapshots reference immutable Drive files; evidence stays redacted.
BEGIN;
CREATE TABLE IF NOT EXISTS remote_files(
    account_binding_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    remote_file_id TEXT NOT NULL,
    ciphertext_hash TEXT NOT NULL,
    PRIMARY KEY(account_binding_id, remote_file_id)
);
CREATE TABLE IF NOT EXISTS snapshots(
    id TEXT PRIMARY KEY,
    vault_id TEXT NOT NULL,
    encrypted_cache_path TEXT,
    metadata_json BLOB NOT NULL,
    sync_state TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS guidance_sessions(
    id TEXT PRIMARY KEY,
    skill_id TEXT NOT NULL,
    revision_id TEXT NOT NULL,
    environment_fingerprint TEXT NOT NULL,
    progress_json BLOB NOT NULL
);
COMMIT;
