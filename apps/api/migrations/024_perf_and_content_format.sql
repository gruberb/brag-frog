-- Speeds up per-week entry listings and marks how review text is stored
-- ('markdown' for legacy rows, 'blocknote' once saved from the new editor).
CREATE INDEX IF NOT EXISTS idx_entries_week_occurred ON brag_entries(week_id, occurred_at) WHERE deleted_at IS NULL;
ALTER TABLE summaries ADD COLUMN content_format TEXT NOT NULL DEFAULT 'markdown';
