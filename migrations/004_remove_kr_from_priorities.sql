-- Remove legacy KR (Key Result) measurement columns from priorities.
-- Priorities are now qualitative focus areas, not measurable key results.
ALTER TABLE priorities DROP COLUMN kr_type;
ALTER TABLE priorities DROP COLUMN direction;
ALTER TABLE priorities DROP COLUMN unit;
ALTER TABLE priorities DROP COLUMN baseline;
ALTER TABLE priorities DROP COLUMN target;
ALTER TABLE priorities DROP COLUMN current_value;
ALTER TABLE priorities DROP COLUMN target_date;
ALTER TABLE priorities DROP COLUMN score;
ALTER TABLE priorities DROP COLUMN progress
