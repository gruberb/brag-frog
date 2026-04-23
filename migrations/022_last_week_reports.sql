-- Persisted AI-generated "Last Week" reports on the Reports page.
-- Keyed on (week_id, user_id) so each anchor week has at most one report;
-- regenerating overwrites in place. Content is AES-256-GCM encrypted.
-- window_start/end are the date range the report narrates, captured at
-- generation time so the stored dates don't drift if the user revisits later.
CREATE TABLE IF NOT EXISTS last_week_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_id INTEGER NOT NULL REFERENCES weeks(id),
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    content BLOB,
    window_start TEXT NOT NULL,
    window_end TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(week_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_last_week_reports_week ON last_week_reports(week_id, user_id);
