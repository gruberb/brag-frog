-- 10x Protocol weekly checklist.
CREATE TABLE IF NOT EXISTS protocol_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_id INTEGER NOT NULL REFERENCES weeks(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    slug TEXT NOT NULL,
    checked INTEGER NOT NULL DEFAULT 0,
    UNIQUE(week_id, user_id, slug)
);

-- Personal task list.
CREATE TABLE IF NOT EXISTS todos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    title BLOB NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_todos_user ON todos(user_id, completed, sort_order);
