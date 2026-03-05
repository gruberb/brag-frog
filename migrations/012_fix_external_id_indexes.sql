-- 012: Fix external_id indexes — remove partial WHERE clause.
-- SQLite does not support ON CONFLICT with partial unique indexes.
-- NULL values in unique indexes are treated as distinct, so non-partial is safe.

DROP INDEX IF EXISTS idx_priorities_external_id;
DROP INDEX IF EXISTS idx_department_goals_external_id;

CREATE UNIQUE INDEX idx_priorities_external_id ON priorities(phase_id, external_id);
CREATE UNIQUE INDEX idx_department_goals_external_id ON department_goals(phase_id, external_id);
