-- 005_import_lookup.sql — efficient identity/hash deduplication for imports (T042).
BEGIN;
CREATE INDEX IF NOT EXISTS revisions_by_skill_and_bundle
    ON revisions(skill_id, bundle_hash, state, id);
COMMIT;
