# Changelog

All notable changes to Brag Frog will be documented in this file.

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
- Set to `delete` in Mozilla Dockerfile to fix stale NFS file handle errors on FUSE-mounted volumes

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
