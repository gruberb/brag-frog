-- Status updates composed for stakeholders.
CREATE TABLE IF NOT EXISTS status_updates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_id INTEGER NOT NULL REFERENCES weeks(id),
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    content BLOB,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(week_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_status_updates_week ON status_updates(week_id, user_id);
