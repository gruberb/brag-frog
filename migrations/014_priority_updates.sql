-- Priority update log: timestamped status snapshots with optional comment.
-- Mirrors Lattice's "Post Update" flow for progress communication.
CREATE TABLE IF NOT EXISTS priority_updates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    priority_id INTEGER NOT NULL REFERENCES priorities(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id),
    tracking_status TEXT,
    measure_value REAL,
    comment BLOB,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_priority_updates_priority ON priority_updates(priority_id, created_at DESC)