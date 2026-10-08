-- 007_repository_bindings.sql — local-only repository-to-suite links (T045).
BEGIN;
CREATE TABLE IF NOT EXISTS repository_bindings(
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    suite_revision_id TEXT NOT NULL REFERENCES revisions(id) ON DELETE RESTRICT,
    repository_root TEXT NOT NULL CHECK(length(repository_root) BETWEEN 1 AND 4096),
    profile TEXT NOT NULL CHECK(profile IN ('rust', 'node', 'generic')),
    strict INTEGER NOT NULL CHECK(strict IN (0, 1)),
    repository_head TEXT NOT NULL CHECK(
        length(repository_head) IN (40, 64)
        AND repository_head NOT GLOB '*[^0-9a-f]*'
    ),
    environment_fingerprint TEXT NOT NULL CHECK(
        length(environment_fingerprint) = 71
        AND substr(environment_fingerprint, 1, 7) = 'sha256:'
        AND substr(environment_fingerprint, 8) NOT GLOB '*[^0-9a-f]*'
    )
);
CREATE INDEX IF NOT EXISTS repository_bindings_skill ON repository_bindings(skill_id, id);
COMMIT;
