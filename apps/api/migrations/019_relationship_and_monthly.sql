-- Relationship health tracking on people aliases.
ALTER TABLE people_aliases ADD COLUMN relationship_type TEXT;
ALTER TABLE people_aliases ADD COLUMN last_helped_at TEXT;
ALTER TABLE people_aliases ADD COLUMN last_helped_by_at TEXT;

-- Monthly growth check-ins.
CREATE TABLE IF NOT EXISTS monthly_checkins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    month INTEGER NOT NULL,
    year INTEGER NOT NULL,
    learning_or_coasting BLOB,
    reconnect_list BLOB,
    energy_trend_note BLOB,
    letting_go BLOB,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(phase_id, user_id, month, year)
);
CREATE INDEX IF NOT EXISTS idx_monthly_checkins_user ON monthly_checkins(user_id, year, month);
