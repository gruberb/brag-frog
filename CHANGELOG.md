# Changelog

All notable changes to Brag Frog will be documented in this file.

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
