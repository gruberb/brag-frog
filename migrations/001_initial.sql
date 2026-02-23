-- Brag Frog Schema
-- Fresh database — all tables created in a single migration.

CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    google_id TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    avatar_url TEXT,
    role TEXT,
    wants_promotion INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_login_at TEXT NOT NULL DEFAULT (datetime('now')),
    display_name TEXT,
    team TEXT,
    manager_name TEXT,
    skip_level_name TEXT,
    direct_reports TEXT,
    timezone TEXT DEFAULT 'UTC',
    week_start TEXT DEFAULT 'monday',
    work_start_time TEXT DEFAULT '09:00',
    work_end_time TEXT DEFAULT '17:00'
);

CREATE TABLE brag_phases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE goals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    title BLOB NOT NULL,
    description BLOB,
    category TEXT,
    status TEXT NOT NULL DEFAULT 'in_progress',
    sort_order INTEGER NOT NULL DEFAULT 0,
    weight INTEGER,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE key_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    color TEXT,
    is_archived INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'not_started',
    goal_id INTEGER REFERENCES goals(id),
    progress INTEGER NOT NULL DEFAULT 0,
    kr_type TEXT NOT NULL DEFAULT 'manual',
    direction TEXT,
    unit TEXT,
    baseline REAL,
    target REAL,
    current_value REAL,
    target_date TEXT,
    score REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, name)
);

CREATE TABLE initiatives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    title BLOB NOT NULL,
    description BLOB,
    status TEXT NOT NULL DEFAULT 'planned',
    scope TEXT,
    is_planned INTEGER NOT NULL DEFAULT 1,
    started_at TEXT,
    completed_at TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE initiative_key_results (
    initiative_id INTEGER NOT NULL REFERENCES initiatives(id) ON DELETE CASCADE,
    key_result_id INTEGER NOT NULL REFERENCES key_results(id) ON DELETE CASCADE,
    PRIMARY KEY (initiative_id, key_result_id)
);

CREATE TABLE weeks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    week_number INTEGER NOT NULL,
    iso_week INTEGER NOT NULL,
    year INTEGER NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    UNIQUE(phase_id, iso_week, year)
);

CREATE TABLE brag_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_id INTEGER NOT NULL REFERENCES weeks(id),
    key_result_id INTEGER REFERENCES key_results(id),
    initiative_id INTEGER REFERENCES initiatives(id),
    source TEXT NOT NULL,
    source_id TEXT,
    source_url TEXT,
    title BLOB NOT NULL,
    description BLOB,
    entry_type TEXT NOT NULL,
    status TEXT,
    repository TEXT,
    occurred_at TEXT NOT NULL,
    teams BLOB,
    collaborators BLOB,
    outcome_statement BLOB,
    evidence_urls TEXT,
    role TEXT,
    impact_tags TEXT,
    reach TEXT,
    complexity TEXT,
    decision_alternatives BLOB,
    decision_reasoning BLOB,
    decision_outcome BLOB,
    meeting_role TEXT,
    recurring_group TEXT,
    start_time TEXT,
    end_time TEXT,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT,
    UNIQUE(source, source_id)
);

CREATE TABLE weekly_checkins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_id INTEGER NOT NULL REFERENCES weeks(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    proud_of BLOB,
    learned BLOB,
    wants_to_change BLOB,
    frustrations BLOB,
    notes BLOB,
    energy_level INTEGER,
    productivity_rating INTEGER,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(week_id, user_id)
);

CREATE TABLE kr_checkin_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    checkin_id INTEGER NOT NULL REFERENCES weekly_checkins(id) ON DELETE CASCADE,
    key_result_id INTEGER NOT NULL REFERENCES key_results(id) ON DELETE CASCADE,
    current_value REAL,
    confidence TEXT NOT NULL DEFAULT 'yellow',
    blockers BLOB,
    next_week_bet BLOB,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    UNIQUE(checkin_id, key_result_id)
);

CREATE TABLE impact_stories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    title BLOB NOT NULL,
    situation BLOB,
    actions BLOB,
    result BLOB,
    status TEXT NOT NULL DEFAULT 'draft',
    sort_order INTEGER NOT NULL DEFAULT 0,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE story_entries (
    story_id INTEGER NOT NULL REFERENCES impact_stories(id) ON DELETE CASCADE,
    entry_id INTEGER NOT NULL REFERENCES brag_entries(id) ON DELETE CASCADE,
    PRIMARY KEY (story_id, entry_id)
);

CREATE TABLE ai_documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    doc_type TEXT NOT NULL,
    title TEXT NOT NULL,
    content BLOB NOT NULL,
    prompt_used BLOB,
    model_used TEXT,
    context_week_id INTEGER REFERENCES weeks(id),
    meeting_entry_id INTEGER REFERENCES brag_entries(id),
    meeting_role TEXT,
    recurring_group TEXT,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    generated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE meeting_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    match_type TEXT NOT NULL,
    match_value TEXT NOT NULL,
    meeting_role TEXT NOT NULL,
    person_name TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, match_type, match_value)
);

CREATE TABLE entry_competencies (
    entry_id INTEGER NOT NULL REFERENCES brag_entries(id) ON DELETE CASCADE,
    competency TEXT NOT NULL,
    dimension TEXT NOT NULL,
    PRIMARY KEY (entry_id, competency)
);

CREATE TABLE integration_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    service TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 0,
    encrypted_token BLOB,
    config_json TEXT,
    last_sync_at TEXT,
    last_sync_status TEXT,
    last_sync_error TEXT,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, service)
);

CREATE TABLE summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    section TEXT NOT NULL,
    content BLOB NOT NULL,
    prompt_used BLOB,
    model_used TEXT,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    generated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE sync_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    service TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL,
    entries_created INTEGER NOT NULL DEFAULT 0,
    entries_updated INTEGER NOT NULL DEFAULT 0,
    entries_deleted INTEGER NOT NULL DEFAULT 0,
    entries_fetched INTEGER NOT NULL DEFAULT 0,
    entries_skipped INTEGER NOT NULL DEFAULT 0,
    error_message TEXT
);

CREATE TABLE weekly_focus (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_id INTEGER NOT NULL REFERENCES weeks(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    sort_order INTEGER NOT NULL DEFAULT 0,
    title BLOB NOT NULL,
    linked_type TEXT,
    linked_id INTEGER,
    link_1 TEXT,
    link_2 TEXT,
    link_3 TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE weekly_focus_entries (
    focus_id INTEGER NOT NULL REFERENCES weekly_focus(id) ON DELETE CASCADE,
    entry_id INTEGER NOT NULL REFERENCES brag_entries(id) ON DELETE CASCADE,
    PRIMARY KEY (focus_id, entry_id)
);

CREATE TABLE meeting_prep_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    week_id INTEGER NOT NULL REFERENCES weeks(id),
    entry_id INTEGER REFERENCES brag_entries(id),
    notes BLOB,
    doc_urls TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, week_id, entry_id)
);

-- Indexes
CREATE INDEX idx_brag_entries_week_id ON brag_entries(week_id);
CREATE INDEX idx_brag_entries_source ON brag_entries(source, source_id);
CREATE INDEX idx_brag_entries_not_deleted ON brag_entries(week_id, deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_brag_entries_initiative ON brag_entries(initiative_id);
CREATE INDEX idx_brag_entries_occurred ON brag_entries(occurred_at);
CREATE INDEX idx_brag_entries_meeting ON brag_entries(source, entry_type, occurred_at) WHERE entry_type = 'meeting';
CREATE INDEX idx_weeks_phase_id ON weeks(phase_id);
CREATE INDEX idx_goals_phase_id ON goals(phase_id);
CREATE INDEX idx_brag_phases_user_active ON brag_phases(user_id, is_active);
CREATE INDEX idx_integration_configs_user_id ON integration_configs(user_id, service);
CREATE INDEX idx_sync_logs_user_id ON sync_logs(user_id, started_at);
CREATE INDEX idx_initiatives_phase ON initiatives(phase_id);
CREATE INDEX idx_weekly_checkins_week ON weekly_checkins(week_id);
CREATE INDEX idx_kr_checkin_snapshots_checkin ON kr_checkin_snapshots(checkin_id);
CREATE INDEX idx_impact_stories_phase ON impact_stories(phase_id);
CREATE INDEX idx_ai_documents_user ON ai_documents(user_id, doc_type, generated_at DESC);
CREATE INDEX idx_entry_competencies_comp ON entry_competencies(competency);
