-- Drop legacy OKR and impact story tables superseded by priorities and contribution examples.
-- Also removes brag_entries columns that reference dropped tables.

-- Drop FK indexes on brag_entries before column removal
DROP INDEX IF EXISTS idx_brag_entries_initiative;
DROP INDEX IF EXISTS idx_brag_entries_key_result;

-- Remove orphaned FK columns from brag_entries (requires SQLite 3.35.0+)
ALTER TABLE brag_entries DROP COLUMN initiative_id;
ALTER TABLE brag_entries DROP COLUMN key_result_id;

-- Drop tables that reference key_results
DROP TABLE IF EXISTS kr_checkin_snapshots;

-- Junction tables first (FK constraints), then parent tables
DROP TABLE IF EXISTS initiative_key_results;
DROP TABLE IF EXISTS initiatives;
DROP TABLE IF EXISTS key_results;
DROP TABLE IF EXISTS goals;
DROP TABLE IF EXISTS story_entries;
DROP TABLE IF EXISTS impact_stories;
DROP TABLE IF EXISTS entry_competencies;
