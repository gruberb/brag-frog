-- 002: Priorities Redesign
-- Replaces goals/key_results/initiatives with department_goals/priorities.
-- Replaces impact_stories with contribution_examples.
-- Adds quarterly check-ins, annual alignment, and new weekly check-in fields.

-- ---------------------------------------------------------------------------
-- New tables
-- ---------------------------------------------------------------------------

-- Department Goals (renamed from goals, drop weight)
CREATE TABLE IF NOT EXISTS department_goals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    title BLOB NOT NULL,
    description BLOB,
    category TEXT,
    status TEXT NOT NULL DEFAULT 'in_progress',
    sort_order INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL DEFAULT 'manual',
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_department_goals_phase ON department_goals(phase_id);

-- Priorities (merges key_results + initiatives)
CREATE TABLE IF NOT EXISTS priorities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    title BLOB NOT NULL,
    description BLOB,
    status TEXT NOT NULL DEFAULT 'active',
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    scope TEXT,
    started_at TEXT,
    completed_at TEXT,
    impact_narrative BLOB,
    department_goal_id INTEGER REFERENCES department_goals(id),
    -- Optional measurement (preserved from KRs for those who want it)
    kr_type TEXT,
    direction TEXT,
    unit TEXT,
    baseline REAL,
    target REAL,
    current_value REAL,
    target_date TEXT,
    score REAL,
    progress INTEGER NOT NULL DEFAULT 0,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_priorities_phase ON priorities(phase_id);
CREATE INDEX IF NOT EXISTS idx_priorities_user ON priorities(user_id);
CREATE INDEX IF NOT EXISTS idx_priorities_dept_goal ON priorities(department_goal_id);

-- Contribution Examples (replaces impact_stories)
CREATE TABLE IF NOT EXISTS contribution_examples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    title BLOB NOT NULL,
    outcome BLOB,
    behaviors BLOB,
    impact_level TEXT,
    learnings BLOB,
    assessment_type TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    sort_order INTEGER NOT NULL DEFAULT 0,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_contribution_examples_phase ON contribution_examples(phase_id);

-- Junction: contribution examples <-> entries
CREATE TABLE IF NOT EXISTS contribution_example_entries (
    example_id INTEGER NOT NULL REFERENCES contribution_examples(id) ON DELETE CASCADE,
    entry_id INTEGER NOT NULL REFERENCES brag_entries(id) ON DELETE CASCADE,
    PRIMARY KEY (example_id, entry_id)
);

-- Quarterly Check-ins (synthesized from weekly)
CREATE TABLE IF NOT EXISTS quarterly_checkins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    quarter TEXT NOT NULL,
    year INTEGER NOT NULL,
    highlights_impact BLOB,
    learnings_adjustments BLOB,
    growth_development BLOB,
    support_feedback BLOB,
    looking_ahead BLOB,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(phase_id, user_id, quarter, year)
);

CREATE INDEX IF NOT EXISTS idx_quarterly_checkins_phase ON quarterly_checkins(phase_id);

-- Annual Alignment (priority setting)
CREATE TABLE IF NOT EXISTS annual_alignment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id INTEGER NOT NULL REFERENCES brag_phases(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    year INTEGER NOT NULL,
    top_outcomes BLOB,
    why_it_matters BLOB,
    success_criteria BLOB,
    learning_goals BLOB,
    support_needed BLOB,
    encryption_version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(phase_id, user_id, year)
);

-- ---------------------------------------------------------------------------
-- Schema additions to existing tables
-- ---------------------------------------------------------------------------

-- Entry: add priority_id
ALTER TABLE brag_entries ADD COLUMN priority_id INTEGER REFERENCES priorities(id);

-- Weekly check-ins: add new aligned fields
ALTER TABLE weekly_checkins ADD COLUMN highlights_impact BLOB;
ALTER TABLE weekly_checkins ADD COLUMN learnings_adjustments BLOB;
ALTER TABLE weekly_checkins ADD COLUMN growth_development BLOB;
ALTER TABLE weekly_checkins ADD COLUMN support_feedback BLOB;
ALTER TABLE weekly_checkins ADD COLUMN looking_ahead BLOB;

-- ---------------------------------------------------------------------------
-- Data migration: goals -> department_goals
-- ---------------------------------------------------------------------------
INSERT INTO department_goals (id, phase_id, title, description, category, status, sort_order, source, encryption_version, created_at)
SELECT id, phase_id, title, description, category, status, sort_order, 'manual', encryption_version, created_at
FROM goals;

-- ---------------------------------------------------------------------------
-- Data migration: initiatives -> priorities
-- Status mapping: planned->not_started, active->active, paused->on_hold
-- ---------------------------------------------------------------------------
INSERT INTO priorities (phase_id, user_id, title, description, status, scope, sort_order, started_at, completed_at, color, encryption_version, created_at)
SELECT
    i.phase_id,
    p.user_id,
    i.title,
    i.description,
    CASE i.status
        WHEN 'planned' THEN 'not_started'
        WHEN 'active' THEN 'active'
        WHEN 'paused' THEN 'on_hold'
        WHEN 'completed' THEN 'completed'
        WHEN 'cancelled' THEN 'cancelled'
        ELSE i.status
    END,
    i.scope,
    i.sort_order,
    i.started_at,
    i.completed_at,
    -- Random color from palette will be set by Rust migration helper
    NULL,
    i.encryption_version,
    i.created_at
FROM initiatives i
JOIN brag_phases p ON i.phase_id = p.id;

-- ---------------------------------------------------------------------------
-- Data migration: impact_stories -> contribution_examples
-- result -> outcome, situation+actions left empty for manual backfill
-- ---------------------------------------------------------------------------
INSERT INTO contribution_examples (id, phase_id, title, outcome, status, sort_order, encryption_version, created_at, updated_at)
SELECT id, phase_id, title, result, status, sort_order, encryption_version, created_at, updated_at
FROM impact_stories;

-- ---------------------------------------------------------------------------
-- Data migration: story_entries -> contribution_example_entries
-- ---------------------------------------------------------------------------
INSERT INTO contribution_example_entries (example_id, entry_id)
SELECT story_id, entry_id FROM story_entries;

-- ---------------------------------------------------------------------------
-- Data migration: brag_entries.initiative_id -> priority_id
-- Map via initiative position in insertion order (initiatives inserted in order)
-- ---------------------------------------------------------------------------
UPDATE brag_entries
SET priority_id = (
    SELECT pr.id FROM priorities pr
    JOIN brag_phases bp ON pr.phase_id = bp.id
    JOIN initiatives i ON i.phase_id = bp.id
    WHERE i.id = brag_entries.initiative_id
      AND pr.title = i.title
      AND pr.phase_id = i.phase_id
    LIMIT 1
)
WHERE initiative_id IS NOT NULL;

-- ---------------------------------------------------------------------------
-- Data migration: weekly checkin content to new fields
-- proud_of -> highlights_impact
-- learned -> learnings_adjustments
-- wants_to_change + frustrations -> support_feedback (keep wants_to_change)
-- notes -> looking_ahead
-- ---------------------------------------------------------------------------
UPDATE weekly_checkins SET
    highlights_impact = proud_of,
    learnings_adjustments = learned,
    support_feedback = COALESCE(wants_to_change, frustrations),
    looking_ahead = notes;

-- ---------------------------------------------------------------------------
-- Data migration: weekly_focus linked_type remapping
-- ---------------------------------------------------------------------------
UPDATE weekly_focus SET linked_type = 'priority'
WHERE linked_type IN ('key_result', 'initiative');

-- Index for new priority_id column
CREATE INDEX IF NOT EXISTS idx_brag_entries_priority ON brag_entries(priority_id);
