# Changelog

All notable changes to Brag Frog will be documented in this file.

## [5.0.0] - 2026-04-22

### Added
- **Reports page.** New top-level page (`/reports`) with two tabs sharing a consistent shell:
  - *Last Week* — rolling AI-generated summary of logbook entries from Monday of the previous ISO week through today. Grouped by linked priority (and its parent department goal), with an `Unassigned` bucket for un-linked work. Read-only, regenerated on demand, never persisted; the logbook remains the source of truth.
  - *Latest Updates* — stakeholder-facing status narrative, editable and persisted per week. Markdown preview with an Edit toggle.
- **Migration 021 — `remove_focus_and_todos`.** Drops `weekly_focus`, `weekly_focus_entries`, and `todos` tables. Phase-delete cascade updated to match.

### Changed
- **Last Week summary prompt restructured.** Now takes pre-grouped `EntryGroup` slices keyed by priority/department goal and emits one `## [priority title]` section per bucket, rather than the previous type-grouped "What Shipped / What Progressed / Key Meetings / Help Given" layout. Reflects how stakeholders actually read progress.
- **Status Update moved from slide-over panel to inline section.** `status_update_panel` handler removed; `generate_status_update` and `save_status_update` now render the shared `components/status_update_section.html` fragment so HTMX can swap it in-place on the Reports page.
- **Meeting prep prompt no longer injects focus items.** Focus section removed from `build_meeting_prep_prompt` inputs and output.
- **Sidebar navigation.** Todos link replaced with Reports (document icon). Reflections keeps its current position.

### Removed
- **Weekly Focus.** Dashboard focus card, carryover suggestions, focus-entry linking, completion toggles, and Last Week button on the dashboard are gone. Handlers (`create_focus`, `update_focus`, `delete_focus`, `toggle_focus_complete`, `last_week_summary`), model (`src/cycle/model/focus.rs`), and repo (`src/cycle/repo/focus.rs`) deleted. The Last Week summary now lives on the Reports page and reads directly from the logbook.
- **Todos feature.** Standalone `/todos` page, encrypted todo titles, toggle/delete handlers, and the `src/todos/` module are removed. The lightweight planning use-case is better served by the logbook plus the Reports "Latest Updates" tab.
- Dashboard template simplified: removed focus card, carryover section, focus entry picker autocomplete, and the `has_ai` / `focus_items` / `picker_entries` / `carryover_items` context keys that fed them. Associated CSS blocks in `static/css/components/dashboard.css` and JS in `static/js/dashboard.js` removed.

### Migration Notes
Existing Weekly Focus and Todos data is dropped by migration 021. Export anything worth keeping before upgrading — there is no in-app export path for these tables.

## [4.0.0] - 2026-04-02

### Added
- **10x Engineer habits framework.** Weekly reflection reworded around ownership, blockers-as-tradeoffs, relationships, giving, and weekly planning. Habit tracker added (boundary protection, curiosity conversations, blocker communication, end-to-end ownership).
- **Enhanced weekly focus.** Focus items now support planning notes (task breakdown), completion toggles with strikethrough, and carryover suggestions from the previous week's incomplete items.
- **Tradeoff-framed blocker log.** Priority updates can be flagged as blockers with a structured tradeoff template ("We can [A] if [sacrifice], or [B] but [consequence]"). Unresolved blocker count shown on dashboard.
- **"What did I do last week?" AI summary.** Button on dashboard generates a structured summary (What Shipped, What Progressed, Key Meetings, Help Given) from last week's entries, focus items, and priorities.
- **Stakeholder status update composer.** Slide-over panel with AI-generated or manual status updates (Progress, Blockers as Tradeoffs, Next Week). Drafts saved per week, copy-to-clipboard for Slack/email.
- **Relationship health tracking.** People aliases now track relationship type (peer, cross-team, PM, designer, stakeholder) and last interaction dates. Stale relationships (30+ days) surfaced in monthly check-ins.
- **Monthly growth check-in.** New monthly reflection with four sections: Learning or Coasting, Reconnect List (auto-populated from stale relationships), Energy & Sustainability (trend from recent weeks), What Can You Let Go.
- **10x Protocol page.** Standalone page with article summary and 14-item weekly checklist grouped by Monday/Daily/Mid-week/Friday/Monthly. Persisted per week with clear-all reset.
- **Todos page.** Personal task list with encrypted titles, completion toggle, and delete. Completed items shown in collapsible section.
- **Reflections page tabs.** Weekly and Monthly reflections in one tabbed view with full CRUD for both.

### Changed
- **Dashboard simplified.** Removed weekly reflection card and monthly check-in prompt. Focus section is now standalone with Status Update and Last Week buttons in header.
- **Review page quarter cards.** All quarter cards now behave consistently — clicking loads content inline (conversation prep or review sections). First quarter auto-selected on page load.
- **Quarterly prep section titles.** New `quarterly_title` field in config prevents weekly titles ("Next Week") from appearing in quarterly context. Quarterly titles: Impact & Outcomes, Lessons & Adjustments, Growth & Relationships, Contributions & Collaboration, Looking Ahead.
- **Sidebar navigation.** Added 10x Protocol (clipboard-check icon) and Todos (checkmark icon) between Priorities and Reflections. Reflections icon changed to book.

### Fixed
- **Select dropdown arrows missing.** Added explicit `appearance: auto` to `.form-select`.
- **Cancel buttons on reflection forms.** Now use direct navigation to bypass HTMX form interception.
- **Nudge bar dismiss.** Removed nudges from dashboard entirely; guidance moved to 10x Protocol checklist.

## [3.0.1] - 2026-03-18

### Fixed
- **Completed priorities missing from dropdowns.** Logbook filter, dashboard quick-add, and entry edit dropdowns now include all priorities regardless of status, not just active ones.
- **Entry edit priority dropdown not grouped by goals.** The priority select when editing a logbook entry now uses `<optgroup>` headers to group priorities under their department goals, matching the logbook filter dropdown.

## [3.0.0] - 2026-03-13

### Changed
- **DDD decomposition of cycle/ module.** Split into three focused bounded contexts:
  - `cycle/` — Phase lifecycle, weeks, focus items, dashboard, logbook, meeting prep, trends
  - `reflections/` — Weekly and quarterly check-ins with config
  - `review/` — Self-review summaries, contribution examples, AI documents, export, assessment/rating config
- **AI helper functions moved to ai/ module.** `get_ai_client()` and `has_ai_for_user()` shared by all modules.
- **Config initialization decentralized.** Each module loads its own config; `initialize_config()` removed.
- Saving a weekly reflection now redirects to /checkins instead of /dashboard.
- Conversation prep opens in slide-over panel on review page instead of navigating away.

### Removed
- Dead code: `get_checkin_section()`, `BragPhase::is_active()` method
- Dead template: `department_goal_item.html`
- Dead CSS: sidebar panel, sidebar list, page mascot, bulk merge toggle classes
- Empty directories: `tools/`, `static/img/product/`

## [2.4.1] - 2026-03-09

### Fixed
- **Google Docs comment-only interactions missing from sync.** The Drive Activity API does not reliably surface comment activities on shared/team documents. Added a supplementary fetch using the Drive Files API and Comments API to catch comments that the Activity API misses. Files recently viewed by the user are checked for authored comments within the sync date range, creating `drive_commented` entries for any found.
- **Google Drive OAuth scope expanded.** Added `drive.readonly` scope alongside the existing `drive.activity.readonly` to support the supplementary comments fetch. Users need to re-connect Google Drive in integrations settings to grant the new scope. Existing tokens without the new scope degrade gracefully — the Activity API results are still returned.

## [2.4.0] - 2026-03-05

### Added
- **Live sync status on integrations page.** Sync All spawns a background sync and the activity area polls every 3s, showing a progress banner with the current service. Sync All button stays in spinner state until sync completes.
- **Sidebar sync indicator.** Green dot when synced, red dot on error, spinning arrows during active sync. Links to integrations page. Updates immediately via OOB swaps when sync starts/stops.
- **Lattice import via sidebar panel.** Import from Lattice now opens in the slide-over panel with inline success/error feedback instead of a full-page form.
- **Auto-create department goals from Lattice parent references.** Individual-only Lattice exports include Parent ID and Parent goal columns — the importer now auto-creates department goals from these and nests priorities under them.
- **Import upsert with external_id.** Re-importing the same Lattice CSV updates existing records instead of creating duplicates. New `external_id` column on priorities and department_goals tables.
- **Priorities page status summary bar.** Colored dot counts for Active, Not Started, On Hold, Completed, Cancelled. Click a status to filter.
- **Priorities search and filter toolbar.** Text search filters by title, dropdown filters by status.
- **Click-to-edit priorities and department goals.** Clicking a priority or department goal row opens the edit panel. No more inline edit/delete/info buttons.
- **Delete button in edit panels.** Both priority and department goal edit panels include a delete action at the bottom.
- **Background sync infrastructure.** `SyncStatusMap` (DashMap) tracks per-user sync state in memory, updated by background sync tasks.

### Changed
- Sync All button always visible on integrations page (not just when no entries exist).
- Synced Entries card no longer has its own Sync All button (single source in Services header).
- Lattice CSV parser accepts flexible column names (`ID`/`Goal ID`, `Goal name`/`Name`, etc.) and strips BOM.
- Department goal rows: chevron toggles collapse, clicking the row opens edit panel.

### Fixed
- Sync All button no longer returns to clickable state before sync finishes (was caused by background spawn returning immediately).
- Sidebar sync indicator was invisible due to `hx-preserve` conflicts with hx-boost page transitions and competing `margin-left: auto` in flex layout.

## [2.2.0] - 2026-02-26

### Added
- Impact narrative field on priority create form. Previously only available when editing an existing priority.
- Info toggle button on department goals and priorities to reveal description/narrative text inline.
- Description textarea in department goal inline edit form on priorities page.
### Fixed
- Department goal edit form on priorities page was missing the description field — only title and status could be edited.
- Department goal description now displays below the title row with a "Description" label, matching priority narrative layout.
- Priority impact narrative aligned flush left with an "Impact Narrative" label instead of indented italic text.
- Integration cards no longer auto-expand when token is missing — all cards start collapsed.
- Bulk edit toolbar on analyze page moved to overlay block with two-row layout (controls + fields) and body padding to keep footer reachable.

## [2.1.1] - 2026-02-26

### Fixed
- People alias table now uses a single `<table>` with fixed column widths (`table-layout: fixed` + `<colgroup>`). Add form inputs, data rows, and edit-mode inputs all share the same columns so everything stays aligned.
- Editing an alias no longer shifts column widths. Edit inputs use the same `alias-input` class as the add row.
- HTMX swap target changed to `innerHTML` on the `<tbody>` so the partial returns bare `<tr>` rows instead of a wrapping `<div>`.

## [2.1.0] - 2026-02-26

### Added
- **People aliases with team mapping.** Settings > People now supports mapping emails, GitHub usernames, and Jira accounts to display names and an optional team. Collaborator emails are stored raw and aliased only at display time, so alias changes take effect immediately without re-syncing.
- **Auto team enrichment from aliases.** When a collaborator matches an alias with a team, that team is automatically added to the entry — both at sync time (persisted) and at display time (for existing entries without re-sync).
- **Inline editing for people aliases.** Each alias row has an Edit button that turns it into editable input fields with Save/Cancel.
- **Collaborator extraction for GitHub, Jira, and Bugzilla.** GitHub sync now extracts PR reviewers (for authored/merged PRs) and PR authors (for reviewed PRs). Jira extracts assignee/creator. Bugzilla extracts assigned_to/creator. All stored as collaborators on entries.
- **Bulk edit for logbook entries.** Select multiple entries and apply priority, reach, complexity, role, teams, or collaborators in one action. Supports append or replace merge modes.
- **Bulk edit toolbar** with select all/deselect, field dropdowns, and append/replace toggle.

### Changed
- **Trends page rebuilt with native charts.** Replaced the insights module with inline CSS-only bar charts for categories, priorities, collaborators, repos, teams, weekdays, and impact signals. Removed the separate analytics/insights service.
- **Logbook report sidebar replaced by bulk edit.** The "Report" button is now "Bulk Edit" with a sticky bottom toolbar.
- **Entry cards show bulk-edit checkboxes** when bulk edit mode is active.
- **Entry meta row wraps on narrow screens** (flex-wrap added to `.entry-meta-left`).

### Removed
- `compute_insights()` function and `insights` service module (replaced by inline Trends page logic).
- `analyze_report.html` template (report sidebar replaced by bulk edit).

## [2.0.1] - 2026-02-26

### Fixed
- Calendar sync no longer stores event bodies. Invite descriptions (Zoom boilerplate, HTML fragments, attendee lists) were being written into `brag_entries.description`, cluttering the logbook. Sync now sets `description: None` for calendar entries, keeping descriptions exclusively for user-authored content.

### Changed
- Calendar attendees stored as collaborators. Attendee names and emails extracted during Google Calendar sync are now written to the `collaborators` field instead of being embedded in the description. They render as collaborator chips on entry cards.
- Sync upsert preserves user-written descriptions. The `ON CONFLICT` clause for `description` changed from unconditional overwrite to `COALESCE(existing, incoming)`, so descriptions set by the user (e.g., meeting prep notes) survive re-syncs.
- `SyncedEntry` carries a `collaborators` field. All non-calendar sync sources pass `None`; only Google Calendar populates it from attendee data.

### Added
- Meeting prep notes flow into logbook entries. Saving prep notes via the dashboard meeting panel encrypts and writes them into `brag_entries.description`, so prep content appears as the meeting body in the logbook and feeds into AI draft generation.

## [2.0.0] - 2026-02-24

### Changed
- **Priorities replace OKRs.** Goals, Key Results, and Initiatives removed in favour of a flat Priorities model with color coding and progress tracking
- **Dashboard layout restructured.** Weekly Focus + Weekly Check-in share a top row at equal height; sidebar reordered to Active Work → Priorities → Focus Time This Week
- **Trends page simplified.** Category Distribution bars now include per-type tooltip breakdowns (hover the ⓘ icon); separate Code Activity and Ticket Activity sections removed to eliminate conflicting counts
- **Category Distribution split.** Bug/Jira entry types moved from "Code" to a new "Tickets" category so counts align with what each category actually represents
- **Priority Activity bars use stacked layout.** Label row on top, bar below — long priority names no longer misalign bars
- **Focus Time day labels shortened** to Mo, Tu, We, Th, Fr
- **Summary page rebuilt** with config-driven sections from `review_sections.toml`
- **Annual Alignment page removed** (replaced by Review Guide)

### Added
- Priorities management page with color picker, drag-to-reorder, and progress tracking
- Review Guide page for self-review preparation
- Summary section components driven by `review_sections.toml` config
- Migration `003_drop_legacy_tables.sql` to clean up legacy OKR/initiative/impact-story tables
- Public routes module for unauthenticated pages

### Fixed
- Trends entry count now matches logbook by filtering out future synced entries
- Calendar sync includes all non-declined events from the user's primary calendar (previously required explicit accept/tentative, missing auto-added org and team meetings)
- Calendar sync handles large org-wide events where the API omits the full attendee list

### Removed
- Goals, Key Results, and Initiatives (model, routes, templates, CSS)
- Annual Alignment page and route
- Analytics module (insights moved to logbook routes)
- Sync button component
- Dead `just-added-list` div and CSS from dashboard quick capture form

## [1.1.4] - 2026-02-24

### Fixed
- SQLite journal mode now configurable via `SQLITE_JOURNAL_MODE` env var (default: WAL)

## [1.1.3] - 2026-02-24

### Fixed
- Dashboard meetings widget now shows all synced calendar meetings, including ones deleted from the logbook
- Manual meeting entries no longer appear in the dashboard meetings widget

## [1.1.2] - 2026-02-24

### Fixed
- Google Calendar sync now includes events where the user is the organizer (responseStatus may not be "accepted")
- Also accept tentatively-accepted events in calendar sync

## [1.1.1] - 2026-02-24

### Fixed
- Logbook entry count now excludes future synced entries, so total matches visible entries with no filters
- "Unlinked" filter now correctly treats entries linked to an initiative as linked

## [1.1.0] - 2026-02-24

### Added
- Link entries to initiatives via dropdown in both the inline edit form and dashboard quick-add
- Initiative filter on the logbook page (first toolbar row, next to Key Results)
- Inline edit form for initiatives on the Goals page, matching the existing goal/KR pattern
- Collapsible goals in the dashboard OKR snapshot with localStorage-persisted state
- "Show only" dropdown in logbook filters (Unlinked, Missing team, Missing collaborator)

### Changed
- Dashboard OKR snapshot hides fully-completed goals (all KRs at 100%) to reduce clutter
- Logbook filter toolbar reorganized: types and show-only moved to second row
- Initiative detail panel is now read-only; editing happens inline on the Goals page

### Fixed
- Google Calendar sync now includes large events with hidden guest lists (attendeesOmitted support)
- 1Password autofill popup no longer triggers on integration token fields (data-1p-ignore)
