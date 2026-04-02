-- Add blocker tracking and tradeoff framing to priority updates.
ALTER TABLE priority_updates ADD COLUMN is_blocker INTEGER NOT NULL DEFAULT 0;
ALTER TABLE priority_updates ADD COLUMN tradeoff_text BLOB;
