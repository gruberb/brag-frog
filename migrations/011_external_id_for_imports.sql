-- 011: Add external_id to priorities and department_goals for import dedup.
-- Allows upsert on re-import from Lattice or other external sources.

ALTER TABLE priorities ADD COLUMN external_id TEXT;
ALTER TABLE department_goals ADD COLUMN external_id TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_priorities_external_id ON priorities(phase_id, external_id) WHERE external_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_department_goals_external_id ON department_goals(phase_id, external_id) WHERE external_id IS NOT NULL;
