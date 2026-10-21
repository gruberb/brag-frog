# Conventions: Brag Frog

Reference material for the entry system, data model, and API and web conventions. See [CLAUDE.md](../CLAUDE.md) for the core development guide.

## Entry System

- **Sources:** `github`, `phabricator`, `bugzilla`, `jira`, `confluence`, `google_drive`, `google_calendar`, `manual`
- **37 entry types:** see `EntryType::ALL` in `apps/api/src/worklog/model.rs`. The web app reads them from `GET /api/v1/meta/entry-types`.
- **Pending requests:** `pr_review_requested` mirrors an open PR still waiting for the user's review (`EntryType::is_pending_request`). By default only direct requests (`user-review-requested:`) in non-archived repositories; the GitHub setting "Include review requests to my teams" switches to `review-requested:`. "Repositories to skip" (`skip_repos`, validated by `github::parse_repo_list`) adds `-repo:` to every GitHub search. Each sync deletes the ones GitHub no longer reports, and trends skip them. Once reviewed, the same row becomes `pr_reviewed`.
- **Open work (Today):** entries of `OPEN_WORK_TYPES` (`cycle/service/dashboard.rs`) anywhere in the active cycle whose status is not finished (`is_finished_status` in `worklog/model.rs`, shared with trends). Jira epics, themes and initiatives sync as `jira_epic` and are left out: their work shows through their child issues.
- **Refresh-only sync items:** `SyncedEntry::refresh_only` updates a stored row but never creates one. Jira sends issues the sync settings exclude this way, so a ticket imported while in progress still turns Done.
- Entries belong to a Week (which belongs to a cycle)
- **Impact fields:** `outcome_statement`, `evidence_urls`, `role`, `impact_tags`, `reach`, `complexity`
- **Decision fields:** `decision_alternatives`, `decision_reasoning`, `decision_outcome`
- **Meeting fields:** `meeting_role`, `recurring_group`
- Encrypted columns: `title`, `description`, `teams`, `collaborators`, `outcome_statement`, `decision_*`. Filter them in application code after decryption.

## Data Model

Hierarchy: **DepartmentGoal → Priority → Entry** (via `department_goal_id` on priorities and `priority_id` on entries). A cycle is a `brag_phases` row.

Tables in use: `users`, `brag_phases`, `weeks`, `brag_entries`, `department_goals`, `priorities`, `priority_updates`, `summaries`, `reports`, `ai_documents`, `meeting_rules`, `meeting_prep_notes`, `people_aliases`, `integration_configs`, `sync_logs`.

Read-only legacy tables (no write path, shown by `GET /api/v1/cycles/{id}/archive`): `quarterly_checkins`, `contribution_examples`, `contribution_example_entries`, `status_updates`, `last_week_reports`.

## Priorities

- `status`: plain string. The UI treats `active | not_started | on_hold` as active and `completed | cancelled` as done.
- `tracking_status`: `on_track`, `off_track`, and similar plain strings, set through priority updates.
- Measurement: `measure_type`, `measure_start`, `measure_target`, `measure_current`.
- Lattice CSV import: `POST /api/v1/priorities/import` (multipart `csv_file`, `cycle_id` in the query string).

## Reports and Review

- **Review sections** come from `review_sections.toml`. Each saved section has `content` and `content_format` (`markdown` for old text, `blocknote` after the first save in the editor).
- **Reports** are created from the current Logbook filters (`POST /api/v1/reports`). A report stores the filters, a stats snapshot and optional AI text. There is no edit endpoint: create a new report to refresh it.
- **Exports** (Markdown, PDF, DOCX) are generated in the browser from `GET /api/v1/cycles/{id}/bundle`.

## API Conventions

- All routes live in `apps/api/src/api/` and are registered in `api/mod.rs`. Authenticated routes sit behind `csrf_protection` then `require_auth_api`.
- Handlers return `ApiResult<T>`. Errors are `{"error":{"code","message"}}` with codes such as `not_found`, `invalid_input` (422), `bad_request`, `unauthorized`, `csrf`, `internal`.
- Use the `ApiAuth` extractor before `State(state)`. Resolve and ownership-check cycles with `api/scope.rs`.
- State-changing requests need `X-Requested-With: brag-frog`.
- Optional request fields: `deserialize_optional_i64` and `deserialize_optional_string` in `kernel/serde_helpers.rs` convert `""` to `None`.
- Wire types derive `TS` with `#[ts(export)]`. Run `moon run api:types` after changing them.

## Web Conventions

- Layers: shared (`components hooks lib stores types utils config`) then `features` then `app`. No feature imports another feature; compose in `src/app/routes`. Enforced by ESLint.
- Each feature has `api/` (query hooks and keys), `components/`, and optionally `utils/`.
- No barrel files. Route paths come from `src/config/paths.ts`.
- Navigation: Today, Logbook, Priorities, Review, Trends in the main nav. Settings (profile, cycles, people, integrations, AI) opens from the gear icon. Cmd+K opens the command palette.
- Fonts and colours are fixed (see CLAUDE.md gotchas 11 to 13).
- Details of how each view maps to the API: [apps/web/README.md](../apps/web/README.md).

## Config System

Three TOML files loaded at startup into `OnceLock` statics: `clg_levels.toml`, `review_sections.toml`, `services.toml`. The API checks `BRAGFROG_CUSTOM_DIR` (default `custom`) first, then falls back to `BRAGFROG_CONFIG_DIR` (default `config`). The custom directory is gitignored. Only `custom/tokens.css` is served over HTTP.
