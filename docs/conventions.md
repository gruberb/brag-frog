# Conventions — Brag Frog

Reference material for the entry system, data model, and subsystem conventions. See [CLAUDE.md](../CLAUDE.md) for the core development guide.

## Entry System

- **Sources:** `github`, `phabricator`, `bugzilla`, `jira`, `confluence`, `google_drive`, `google_calendar`, `manual`
- **35 entry types** — see `EntryType::ALL` in `src/models/entry.rs`
- Entries belong to a Week (which belongs to a Phase)
- **Impact fields:** `outcome_statement`, `evidence_urls`, `role`, `impact_tags`, `reach`, `complexity`
- **Decision fields:** `decision_alternatives`, `decision_reasoning`, `decision_outcome`
- **Meeting fields:** `meeting_role`, `recurring_group`

## Data Model

Hierarchy: **Goal → KeyResult → Entry** (via `key_result_id` → `goal_id` FKs). **Initiative → Entry** (via `initiative_id`). **Initiative ↔ KeyResult** (many-to-many via `initiative_key_results`).

Additional tables: `initiatives`, `initiative_key_results`, `weekly_checkins`, `kr_checkin_snapshots`, `impact_stories`, `story_entries`, `ai_documents`, `meeting_rules`, `entry_competencies`, `weekly_focus`, `weekly_focus_items`, `meeting_prep_notes`.

## KR Measurement System

- `kr_type`: `manual` (default), `numeric`, `boolean`, `milestone`
- `direction`: `increase`, `decrease`, `maintain` (for numeric)
- Score: auto-calculated 0.0–1.0 via `KeyResult::recalculate_score()`
- `progress` field kept in sync: `progress = (score * 100).round()`

## Panel System

- Slide-over panels for entry detail, initiative detail, KR updates
- JS: `openPanel(title, html)`, `openPanelFromUrl(title, url)`, `closePanel()`
- Escape key closes panel. Scrim click closes panel.
- Panel content loaded into `#panel-body` div via `htmx.ajax('GET', url, {target: '#panel-body'})`

## Navigation

- **Primary nav:** Dashboard, Logbook, Goals, Prep, Review
- **Secondary (avatar dropdown):** Settings, Integrations, Level Guide, Trends, Export, Sync All, Sign Out
- `GET /` → redirects to `/dashboard`
- `/goals` — OKR management (goals, key results, initiatives, phase management)
- `/meeting-prep` — meeting preparation (notes per meeting)
- `/checkins` — check-in history
- `/integrations` — service connections (split from settings)
- `/settings` — profile and preferences
- `/review/:id` — self-review page
- `GET /phases` route removed (merged into `/goals`)

## Config System

Three TOML files loaded at startup into `OnceLock` statics: `clg_levels.toml`, `review_sections.toml`, `services.toml`. App checks `custom/` first, falls back to `config/`. The `custom/` directory is gitignored.
