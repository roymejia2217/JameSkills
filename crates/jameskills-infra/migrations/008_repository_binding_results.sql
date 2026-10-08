-- 008_repository_binding_results.sql — local-only prior check summaries (T045).
BEGIN;
CREATE TABLE IF NOT EXISTS repository_binding_results(
    binding_id TEXT PRIMARY KEY REFERENCES repository_bindings(id) ON DELETE CASCADE,
    report_json TEXT NOT NULL CHECK(length(report_json) BETWEEN 1 AND 8388608)
);
COMMIT;
