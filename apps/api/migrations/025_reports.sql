-- Saved reports created from Logbook filters. title, content and stats_json are AES-256-GCM
-- encrypted. filters_json holds the Logbook filter set as plain JSON (ids and dates only) so
-- a report can show what it was built from. stats_json is a snapshot taken at generation
-- time and does not change when entries are edited later.
CREATE TABLE IF NOT EXISTS reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id) ON DELETE CASCADE,
    title BLOB NOT NULL,
    filters_json TEXT NOT NULL,
    content BLOB NOT NULL,
    content_format TEXT NOT NULL DEFAULT 'blocknote',
    stats_json BLOB NOT NULL,
    model_used TEXT,
    generated_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_reports_user_phase ON reports(user_id, phase_id, generated_at DESC);
