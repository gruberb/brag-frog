-- Add planning notes and completion tracking to weekly focus items.
ALTER TABLE weekly_focus ADD COLUMN notes BLOB;
ALTER TABLE weekly_focus ADD COLUMN completed INTEGER NOT NULL DEFAULT 0;
